// get-position: show user's validator LST holdings across all tracked mints.
//
// Flow:
//   1. Resolve wallet via onchainos
//   2. getTokenAccountsByOwner (all SPL tokens) → filter against registry mints
//   3. GET /v1/sol-value/current for held mints → convert to SOL equivalent

use anyhow::Result;
use clap::Args;
use serde_json::Value;
use std::collections::HashMap;

use crate::api;
use crate::config::{self, LST_DECIMALS};
use crate::onchainos;
use crate::rpc;

#[derive(Args)]
pub struct GetPositionArgs {
    /// Chain ID (must be 501)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,
}

pub async fn run(args: GetPositionArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("sanctum-validator-lst only supports Solana (chain 501)");
    }

    // Resolve wallet
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet. Make sure onchainos is logged in.");
    }

    // Build registry mint set for O(1) lookup
    let registry: HashMap<&str, &config::LstConfig> =
        config::LSTS.iter().map(|l| (l.mint, l)).collect();

    // Fetch all token accounts
    let all_accounts = rpc::get_all_token_accounts(&wallet).await?;

    // Filter to registry mints with non-zero balance
    let mut held: Vec<(&config::LstConfig, f64, u64, String)> = Vec::new();
    for (mint, ui, raw, addr) in &all_accounts {
        if let Some(lst_cfg) = registry.get(mint.as_str()) {
            if *raw > 0 {
                held.push((lst_cfg, *ui, *raw, addr.clone()));
            }
        }
    }

    if held.is_empty() {
        return Ok(serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "holdings": [],
                "total_sol_value": "0.000000000",
                "note": "No tracked validator LST holdings found."
            }
        }));
    }

    // Fetch SOL value for held mints
    let held_mints: Vec<&str> = held.iter().map(|(cfg, _, _, _)| cfg.mint).collect();
    let client = reqwest::Client::new();
    let sol_value_map = api::get_sol_value(&client, &held_mints)
        .await
        .map(|r| r.sol_values)
        .unwrap_or_default();

    let mut total_sol_value = 0.0f64;
    let mut entries = Vec::new();

    for (lst_cfg, ui_balance, raw_balance, account_addr) in &held {
        let sol_value_lamports = sol_value_map
            .get(lst_cfg.mint)
            .and_then(|s| s.parse::<u64>().ok());

        // sol_value_lamports is the lamports per 1 token (in atomic units, i.e. per 1e9 atomics)
        // So: sol_value = ui_balance * (lamports / 1e9)
        let sol_value = sol_value_lamports.map(|l| {
            ui_balance * (l as f64 / 1e9)
        });

        if let Some(sv) = sol_value {
            total_sol_value += sv;
        }

        let sol_per_lst = sol_value_lamports
            .map(|l| api::atomics_to_ui(l, LST_DECIMALS));

        entries.push(serde_json::json!({
            "symbol": lst_cfg.symbol,
            "mint": lst_cfg.mint,
            "token_account": account_addr,
            "balance_ui": format!("{:.9}", ui_balance),
            "balance_raw": raw_balance.to_string(),
            "sol_value": sol_value.map(|v| format!("{:.9}", v)).unwrap_or_else(|| "N/A".to_string()),
            "sol_per_lst": sol_per_lst.map(|v| format!("{:.9}", v)).unwrap_or_else(|| "N/A".to_string()),
        }));
    }

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "wallet": wallet,
            "holdings": entries,
            "total_sol_value": format!("{:.9}", total_sol_value),
        }
    }))
}
