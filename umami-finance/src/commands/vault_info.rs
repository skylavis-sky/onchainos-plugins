/// vault-info: detailed info for a specific vault

use crate::{config, rpc};
use anyhow::Result;

pub async fn execute(vault_identifier: &str, chain_id: u64) -> Result<()> {
    let rpc = config::ARBITRUM_RPC;

    let vault = config::find_vault(vault_identifier)
        .ok_or_else(|| anyhow::anyhow!("Unknown vault: {}. Use list-vaults to see available vaults.", vault_identifier))?;

    let total_assets = rpc::total_assets(rpc, vault.address).await?;
    let total_supply = rpc::total_supply(rpc, vault.address).await?;
    let one_unit = 10u128.pow(vault.asset_decimals);
    let pps = rpc::convert_to_assets(rpc, vault.address, one_unit).await?;

    let total_human = format!(
        "{:.6}",
        total_assets as f64 / 10f64.powi(vault.asset_decimals as i32)
    );
    let pps_human = format!(
        "{:.8}",
        pps as f64 / 10f64.powi(vault.asset_decimals as i32)
    );

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "vault": {
                "name": vault.name,
                "address": vault.address,
                "asset_symbol": vault.asset_symbol,
                "asset_address": vault.asset_address,
                "asset_decimals": vault.asset_decimals,
                "total_assets_raw": total_assets.to_string(),
                "total_assets_human": format!("{} {}", total_human, vault.asset_symbol),
                "total_supply_raw": total_supply.to_string(),
                "price_per_share_raw": pps.to_string(),
                "price_per_share_human": format!("{} {}/share", pps_human, vault.asset_symbol),
                "description": vault.description
            }
        }))?
    );
    Ok(())
}
