// rates — query Maple Finance pool exchange rates and TVL
// Uses eth_call: totalAssets, totalSupply, convertToExitAssets for 1 share

use crate::{config, rpc};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct RateInfo {
    pub pool: String,
    pub pool_address: String,
    pub underlying_symbol: String,
    pub tvl_raw: String,
    pub tvl_formatted: String,
    pub exchange_rate: String,
    pub note: String,
}

pub async fn run(rpc_url: &str) -> Result<()> {
    let pools = config::pools();
    let mut rates: Vec<RateInfo> = Vec::new();

    for pool_cfg in &pools {
        let total_assets = rpc::total_assets(rpc_url, pool_cfg.pool).await.unwrap_or(0);
        let total_supply = rpc::total_supply(rpc_url, pool_cfg.pool).await.unwrap_or(0);

        // Exchange rate: how many underlying per share
        // Use convertToExitAssets(1e6 shares) to get rate
        let one_share_unit: u128 = 10u128.pow(pool_cfg.decimals);
        let assets_per_share = if total_supply > 0 {
            rpc::convert_to_exit_assets(rpc_url, pool_cfg.pool, one_share_unit)
                .await
                .unwrap_or(one_share_unit)
        } else {
            one_share_unit
        };

        let exchange_rate = assets_per_share as f64 / one_share_unit as f64;
        let tvl_fmt = format!(
            "{:.2} {}",
            rpc::format_amount(total_assets, pool_cfg.decimals),
            pool_cfg.token_symbol
        );

        rates.push(RateInfo {
            pool: pool_cfg.name.to_string(),
            pool_address: pool_cfg.pool.to_string(),
            underlying_symbol: pool_cfg.token_symbol.to_string(),
            tvl_raw: total_assets.to_string(),
            tvl_formatted: tvl_fmt,
            exchange_rate: format!("{:.8}", exchange_rate),
            note: "Exchange rate = underlying value per 1 syrup share. Higher rate = more yield accrued.".to_string(),
        });
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "data": {
                "rates": rates,
                "chain": "ethereum",
                "chain_id": 1,
                "note": "APY not available via on-chain data. Exchange rate growth over time reflects yield."
            }
        }))?
    );
    Ok(())
}
