/// positions: show user's vault positions

use crate::{config, onchainos, rpc};
use anyhow::Result;
use serde::Serialize;

#[derive(Serialize)]
pub struct PositionEntry {
    pub vault_name: String,
    pub vault_address: String,
    pub asset_symbol: String,
    pub shares_raw: String,
    pub shares_human: String,
    pub assets_value_raw: String,
    pub assets_value_human: String,
}

pub async fn execute(chain_id: u64, from: Option<&str>) -> Result<()> {
    let rpc = config::ARBITRUM_RPC;

    // Resolve wallet — resolve here since this is a read-only op with no dry_run path
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        let w = onchainos::resolve_wallet(chain_id)?;
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet address. Pass --from <address> or ensure onchainos is logged in.");
        }
        w
    };

    let mut positions = Vec::new();

    for v in config::VAULTS {
        let shares = rpc::balance_of(rpc, v.address, &wallet).await.unwrap_or(0);
        if shares > 0 {
            let assets_value = rpc::preview_redeem(rpc, v.address, shares).await.unwrap_or(0);
            let shares_human = format!(
                "{:.8}",
                shares as f64 / 10f64.powi(v.asset_decimals as i32)
            );
            let assets_human = format!(
                "{:.6} {}",
                assets_value as f64 / 10f64.powi(v.asset_decimals as i32),
                v.asset_symbol
            );
            positions.push(PositionEntry {
                vault_name: v.name.to_string(),
                vault_address: v.address.to_string(),
                asset_symbol: v.asset_symbol.to_string(),
                shares_raw: shares.to_string(),
                shares_human,
                assets_value_raw: assets_value.to_string(),
                assets_value_human: assets_human,
            });
        }
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "chain_id": chain_id,
            "wallet": wallet,
            "positions": positions
        }))?
    );
    Ok(())
}
