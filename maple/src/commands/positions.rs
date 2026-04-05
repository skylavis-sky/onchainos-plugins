// positions — query user's Maple Finance lending positions
// Checks balanceOf on both syrupUSDC and syrupUSDT pools

use crate::{config, onchainos, rpc};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct Position {
    pub pool: String,
    pub pool_address: String,
    pub shares_raw: String,
    pub shares_formatted: String,
    pub underlying_value_raw: String,
    pub underlying_value_formatted: String,
    pub underlying_symbol: String,
}

pub async fn run(rpc_url: &str, wallet: Option<String>) -> Result<()> {
    // Resolve wallet address
    let wallet_addr = if let Some(w) = wallet {
        w
    } else {
        onchainos::resolve_wallet()?
    };

    if wallet_addr.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --from or ensure onchainos is logged in.");
    }

    let pools = config::pools();
    let mut positions: Vec<Position> = Vec::new();

    for pool_cfg in &pools {
        let shares = rpc::balance_of(rpc_url, pool_cfg.pool, &wallet_addr)
            .await
            .unwrap_or(0);

        // Only include pools where user has a balance
        if shares == 0 {
            positions.push(Position {
                pool: pool_cfg.name.to_string(),
                pool_address: pool_cfg.pool.to_string(),
                shares_raw: "0".to_string(),
                shares_formatted: "0.000000".to_string(),
                underlying_value_raw: "0".to_string(),
                underlying_value_formatted: format!("0.00 {}", pool_cfg.token_symbol),
                underlying_symbol: pool_cfg.token_symbol.to_string(),
            });
            continue;
        }

        // Get underlying value using convertToExitAssets (accounts for unrealized losses)
        let assets = rpc::convert_to_exit_assets(rpc_url, pool_cfg.pool, shares)
            .await
            .unwrap_or(0);

        let shares_fmt = format!(
            "{:.6}",
            rpc::format_amount(shares, pool_cfg.decimals)
        );
        let assets_fmt = format!(
            "{:.6} {}",
            rpc::format_amount(assets, pool_cfg.decimals),
            pool_cfg.token_symbol
        );

        positions.push(Position {
            pool: pool_cfg.name.to_string(),
            pool_address: pool_cfg.pool.to_string(),
            shares_raw: shares.to_string(),
            shares_formatted: shares_fmt,
            underlying_value_raw: assets.to_string(),
            underlying_value_formatted: assets_fmt,
            underlying_symbol: pool_cfg.token_symbol.to_string(),
        });
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet_addr,
                "positions": positions,
                "chain": "ethereum",
                "chain_id": 1
            }
        }))?
    );
    Ok(())
}
