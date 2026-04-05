/// list-vaults: list all Umami GM vaults with TVL and pricePerShare

use crate::{config, rpc};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct VaultSummary {
    pub name: String,
    pub address: String,
    pub asset_symbol: String,
    pub asset_address: String,
    pub total_assets_raw: String,   // u128 as string to avoid serde overflow
    pub total_assets_human: String,
    pub price_per_share_raw: String,
    pub price_per_share_human: String,
    pub description: String,
}

pub async fn execute(chain_id: u64) -> Result<()> {
    let rpc = config::ARBITRUM_RPC;

    let mut vaults = Vec::new();

    for v in config::VAULTS {
        let total_assets = rpc::total_assets(rpc, v.address).await.unwrap_or(0);
        // price per share: convert 1 unit of asset (10^decimals) to shares then back
        let one_unit = 10u128.pow(v.asset_decimals);
        let pps = rpc::convert_to_assets(rpc, v.address, one_unit).await.unwrap_or(one_unit);

        let total_human = format!(
            "{:.6}",
            total_assets as f64 / 10f64.powi(v.asset_decimals as i32)
        );
        let pps_human = format!(
            "{:.8}",
            pps as f64 / 10f64.powi(v.asset_decimals as i32)
        );

        vaults.push(VaultSummary {
            name: v.name.to_string(),
            address: v.address.to_string(),
            asset_symbol: v.asset_symbol.to_string(),
            asset_address: v.asset_address.to_string(),
            total_assets_raw: total_assets.to_string(),
            total_assets_human: format!("{} {}", total_human, v.asset_symbol),
            price_per_share_raw: pps.to_string(),
            price_per_share_human: format!("{} {}/share", pps_human, v.asset_symbol),
            description: v.description.to_string(),
        });
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "vaults": vaults
        }))?
    );
    Ok(())
}
