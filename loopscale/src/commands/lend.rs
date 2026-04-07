// commands/lend.rs — Deposit into a Loopscale lending vault
use anyhow::Result;
use serde_json::json;

use crate::api;
use crate::config::{cbps_to_pct, default_vault_for_token, to_lamports};
use crate::onchainos;

pub async fn run(
    token: String,
    amount: f64,
    vault: Option<String>,
    dry_run: bool,
) -> Result<()> {
    // Resolve wallet
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet address. Ensure onchainos is logged in.");
    }

    // Resolve vault address
    let vault_addr = vault
        .as_deref()
        .unwrap_or_else(|| default_vault_for_token(&token));

    // Convert UI amount to lamports
    let lamports = to_lamports(amount, &token);

    // Get vault APY for display (used in both dry-run and live paths)
    let vaults_resp = api::get_vaults(vec![]).await.unwrap_or(json!([]));
    let apy_cbps = vaults_resp.as_array()
        .and_then(|arr| arr.iter().find(|v| v["vaultAddress"].as_str() == Some(vault_addr)))
        .and_then(|v| v["apy"].as_u64())
        .unwrap_or(0);

    if dry_run {
        let preview = json!({
            "operation": "lend",
            "vault": vault_addr,
            "token": token,
            "amount": amount,
            "lamports": lamports,
            "estimated_apy": format!("{:.2}%", cbps_to_pct(apy_cbps)),
            "wallet": wallet,
            "note": "Lend amounts are in human-readable units; plugin converts to lamports internally"
        });
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Build deposit transaction
    let tx_resp = api::build_lend_tx(&wallet, vault_addr, lamports).await?;

    // Extract base64 transaction from response
    // Response: { "transaction": { "message": "<BASE64>", ... }, "stakeAccount": ... }
    let b64_tx = tx_resp["transaction"]["message"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No transaction.message in deposit response: {}", tx_resp))?;

    // Submit transaction (base64 → base58 conversion happens in submit_solana_tx)
    let result = onchainos::submit_solana_tx(b64_tx, vault_addr, false).await?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    println!("{}", json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "operation": "lend",
            "vault": vault_addr,
            "token": token,
            "amount_deposited": amount,
            "estimated_apy": format!("{:.2}%", cbps_to_pct(apy_cbps)),
            "solscan": format!("https://solscan.io/tx/{}", tx_hash)
        }
    }));
    Ok(())
}
