// commands/withdraw.rs — Withdraw from a Loopscale lending vault
use anyhow::Result;
use serde_json::json;

use crate::api;
use crate::config::{default_vault_for_token, to_lamports};
use crate::onchainos;

pub async fn run(
    token: String,
    amount: Option<f64>,
    vault: Option<String>,
    withdraw_all: bool,
    dry_run: bool,
) -> Result<()> {
    if amount.is_none() && !withdraw_all {
        anyhow::bail!("Provide --amount <value> or --all to withdraw everything.");
    }

    // Resolve wallet
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet address. Ensure onchainos is logged in.");
    }

    // Resolve vault address
    let vault_addr = vault
        .as_deref()
        .unwrap_or_else(|| default_vault_for_token(&token));

    // Determine lamports to withdraw
    // If --all, find the user's deposited amount from the vault data
    let lamports = if withdraw_all {
        // Fetch user balance from vault deposits
        let vaults_resp = api::get_vaults(vec![]).await?;
        let vaults_arr = vaults_resp.as_array()
            .cloned()
            .unwrap_or_else(|| vaults_resp["vaults"].as_array().cloned().unwrap_or_default());
        let user_balance = vaults_arr.iter()
            .find(|v| v["vaultAddress"].as_str() == Some(vault_addr))
            .and_then(|v| v["userDeposits"].as_array())
            .and_then(|deps| {
                deps.iter().find(|d| {
                    d["userAddress"].as_str()
                        .map(|a| a.eq_ignore_ascii_case(&wallet))
                        .unwrap_or(false)
                })
            })
            .and_then(|d| {
                d["amountSupplied"].as_u64()
                    .or_else(|| d["amountSupplied"].as_str().and_then(|s| s.parse().ok()))
            })
            .unwrap_or(0);
        if user_balance == 0 {
            anyhow::bail!("No deposit found for wallet {} in vault {}", wallet, vault_addr);
        }
        user_balance
    } else {
        to_lamports(amount.unwrap(), &token)
    };

    let preview = json!({
        "operation": "withdraw",
        "vault": vault_addr,
        "token": token,
        "lamports": lamports,
        "withdraw_all": withdraw_all,
        "wallet": wallet,
        "note": "Instant withdrawal available if vault liquidity buffer has capacity; otherwise a small exit fee applies"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Build withdrawal transaction
    let tx_resp = api::build_withdraw_tx(&wallet, vault_addr, lamports, withdraw_all).await?;
    let b64_tx = tx_resp["transaction"]["message"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No transaction.message in withdraw response: {}", tx_resp))?;

    // Submit transaction
    let result = onchainos::submit_solana_tx(b64_tx, vault_addr, false).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    println!("{}", json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "operation": "withdraw",
            "vault": vault_addr,
            "token": token,
            "lamports_withdrawn": lamports,
            "solscan": format!("https://solscan.io/tx/{}", tx_hash)
        }
    }));
    Ok(())
}
