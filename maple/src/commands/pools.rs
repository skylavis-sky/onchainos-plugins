// pools — list Maple Finance syrup pools with TVL info
// Uses direct eth_call to syrupUSDC and syrupUSDT pool contracts

use crate::{config, rpc};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct PoolInfo {
    pub name: String,
    pub pool_address: String,
    pub underlying_token: String,
    pub underlying_symbol: String,
    pub total_assets_raw: String,
    pub total_assets_formatted: String,
    pub total_supply_raw: String,
    pub exchange_rate: String,
}

pub async fn run(rpc_url: &str) -> Result<()> {
    let pools = config::pools();
    let mut results: Vec<PoolInfo> = Vec::new();

    for pool_cfg in &pools {
        let total_assets = rpc::total_assets(rpc_url, pool_cfg.pool).await.unwrap_or(0);
        let total_supply = rpc::total_supply(rpc_url, pool_cfg.pool).await.unwrap_or(0);

        // Exchange rate: assets per share (1 share = X underlying)
        let exchange_rate = if total_supply > 0 {
            let rate = total_assets as f64 / total_supply as f64;
            format!("{:.6}", rate)
        } else {
            "1.000000".to_string()
        };

        let total_assets_fmt = format!(
            "{:.2} {}",
            rpc::format_amount(total_assets, pool_cfg.decimals),
            pool_cfg.token_symbol
        );

        results.push(PoolInfo {
            name: pool_cfg.name.to_string(),
            pool_address: pool_cfg.pool.to_string(),
            underlying_token: pool_cfg.token.to_string(),
            underlying_symbol: pool_cfg.token_symbol.to_string(),
            total_assets_raw: total_assets.to_string(),
            total_assets_formatted: total_assets_fmt,
            total_supply_raw: total_supply.to_string(),
            exchange_rate,
        });
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "pools": results,
                "chain": "ethereum",
                "chain_id": 1
            }
        }))?
    );
    Ok(())
}
