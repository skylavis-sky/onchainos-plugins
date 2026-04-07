// commands/get_position.rs — Fetch user's active lend/borrow positions on Loopscale
use anyhow::Result;
use serde_json::json;

use crate::api;
use crate::config::{cbps_to_pct, from_lamports, MINT_USDC, MINT_WSOL, VAULT_SOL_PRIMARY};
use crate::onchainos;

pub async fn run(wallet: Option<String>) -> Result<()> {
    // Resolve wallet address
    let wallet_addr = match wallet {
        Some(w) => w,
        None => onchainos::resolve_wallet_solana()?,
    };
    if wallet_addr.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet address. Provide --wallet or ensure onchainos is logged in.");
    }

    // Fetch active borrow positions (filterType=0 = Active)
    let loans_resp = api::get_loans(&wallet_addr, 0, 50, 1).await?;

    let loan_infos = loans_resp["loanInfos"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut borrow_positions: Vec<serde_json::Value> = Vec::new();
    for info in &loan_infos {
        let loan = &info["loan"];
        let loan_addr = loan["address"].as_str().unwrap_or("unknown");
        let status = loan["loanStatus"].as_u64().unwrap_or(99);
        if status != 0 { continue; } // Only active loans

        // Ledgers: principal owed, APY, maturity
        let ledger = info["ledgers"].get(0).cloned().unwrap_or(json!({}));
        let principal_mint = ledger["principalMint"].as_str().unwrap_or(MINT_USDC);
        let token_sym = if principal_mint == MINT_WSOL { "SOL" } else { "USDC" };
        let principal_due = ledger["principalDue"].as_u64()
            .or_else(|| ledger["principalDue"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let principal_repaid = ledger["principalRepaid"].as_u64()
            .or_else(|| ledger["principalRepaid"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);
        let apy_cbps = ledger["apy"].as_u64().unwrap_or(0);
        let end_time = ledger["endTime"].as_u64().unwrap_or(0);

        // Collateral
        let collateral = info["collateral"].get(0).cloned().unwrap_or(json!({}));
        let coll_mint = collateral["assetMint"].as_str().unwrap_or(MINT_USDC);
        let coll_sym = if coll_mint == MINT_WSOL { "SOL" } else { "USDC" };
        let coll_amount = collateral["amount"].as_u64()
            .or_else(|| collateral["amount"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0);

        borrow_positions.push(json!({
            "loan_address": loan_addr,
            "principal_token": token_sym,
            "principal_due": from_lamports(principal_due, token_sym),
            "principal_repaid": from_lamports(principal_repaid, token_sym),
            "apy": format!("{:.2}%", cbps_to_pct(apy_cbps)),
            "maturity_unix": end_time,
            "collateral_token": coll_sym,
            "collateral_amount": from_lamports(coll_amount, coll_sym)
        }));
    }

    // Fetch vault deposit positions
    let vaults_resp = api::get_vaults(vec![]).await?;
    let vaults_raw = vaults_resp.as_array()
        .cloned()
        .unwrap_or_else(|| vaults_resp["vaults"].as_array().cloned().unwrap_or_default());

    let mut lend_positions: Vec<serde_json::Value> = Vec::new();
    for vault in &vaults_raw {
        let vault_addr = vault["vaultAddress"].as_str()
            .or_else(|| vault["address"].as_str())
            .unwrap_or("unknown");
        // Infer token from known vault address; deposits endpoint doesn't include principalMint
        let token_sym = if vault_addr == VAULT_SOL_PRIMARY { "SOL" } else { "USDC" };
        let _principal_mint = if token_sym == "SOL" { MINT_WSOL } else { MINT_USDC };

        // Check if user is a depositor
        if let Some(deposits) = vault["userDeposits"].as_array() {
            for dep in deposits {
                let user_addr = dep["userAddress"].as_str().unwrap_or("");
                if user_addr.eq_ignore_ascii_case(&wallet_addr) {
                    let supplied = dep["amountSupplied"].as_u64()
                        .or_else(|| dep["amountSupplied"].as_str().and_then(|s| s.parse().ok()))
                        .unwrap_or(0);
                    if supplied > 0 {
                        let apy_cbps = vault["apy"].as_u64().unwrap_or(0);
                        lend_positions.push(json!({
                            "vault_address": vault_addr,
                            "token": token_sym,
                            "amount_supplied": from_lamports(supplied, token_sym),
                            "apy": format!("{:.2}%", cbps_to_pct(apy_cbps))
                        }));
                    }
                }
            }
        }
    }

    println!("{}", json!({
        "ok": true,
        "data": {
            "wallet": wallet_addr,
            "lend_positions": lend_positions,
            "borrow_positions": borrow_positions,
            "summary": {
                "active_loans": borrow_positions.len(),
                "vault_deposits": lend_positions.len()
            }
        }
    }));
    Ok(())
}
