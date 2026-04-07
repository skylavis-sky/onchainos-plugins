/// get-positions: Query subaccount positions and margin health on Vertex Edge.
///
/// Calls POST /query {"type": "subaccount_info", "subaccount": "<bytes32_hex>"}
///
/// The subaccount is 32 bytes: 20-byte address + 12-byte name (right-padded "default" with nulls).
/// Returns perp_balances, spot_balances, and health metrics.

use anyhow::Context;
use serde_json::{json, Value};

use crate::api::{query_subaccount_info, x18_to_f64};
use crate::config::{build_subaccount_hex, get_chain_config, DEFAULT_SUBACCOUNT_NAME};
use crate::onchainos::resolve_wallet;

pub async fn run(chain_id: u64, address: Option<&str>) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve wallet address: use provided address or fetch from onchainos
    let wallet_addr = match address {
        Some(addr) => addr.to_string(),
        None => resolve_wallet(chain_id).context("Failed to resolve wallet address")?,
    };

    let subaccount = build_subaccount_hex(&wallet_addr, DEFAULT_SUBACCOUNT_NAME)
        .context("Failed to build subaccount hex")?;

    let resp = query_subaccount_info(cfg.gateway_url, &subaccount)
        .await
        .context("Failed to query subaccount_info")?;

    let data = &resp["data"];

    // Parse perp balances
    let mut perp_positions: Vec<Value> = Vec::new();
    if let Some(perp_balances) = data["perp_balances"].as_array() {
        for balance in perp_balances {
            let product_id = balance["product_id"].as_u64().unwrap_or(0) as u32;
            let amount_x18 = balance["balance"]["amount_x18"]
                .as_str()
                .unwrap_or("0");
            let v_quote_x18 = balance["balance"]["v_quote_balance_x18"]
                .as_str()
                .unwrap_or("0");
            let amount = x18_to_f64(amount_x18);
            let v_quote = x18_to_f64(v_quote_x18);

            // Skip zero positions
            if amount == 0.0 && v_quote == 0.0 {
                continue;
            }

            let side = if amount > 0.0 { "long" } else { "short" };

            perp_positions.push(json!({
                "product_id": product_id,
                "type": "perp",
                "side": side,
                "size": format!("{:.6}", amount.abs()),
                "v_quote_balance": format!("{:.6}", v_quote),
                "unrealized_pnl_approx": format!("{:.6}", v_quote),
            }));
        }
    }

    // Parse spot balances
    let mut spot_balances: Vec<Value> = Vec::new();
    if let Some(spot_bals) = data["spot_balances"].as_array() {
        for balance in spot_bals {
            let product_id = balance["product_id"].as_u64().unwrap_or(0) as u32;
            let amount_x18 = balance["balance"]["amount_x18"]
                .as_str()
                .unwrap_or("0");
            let amount = x18_to_f64(amount_x18);

            // Skip zero balances
            if amount == 0.0 {
                continue;
            }

            // product_id 0 = USDC collateral
            let symbol = if product_id == 0 { "USDC" } else { "UNKNOWN" };

            spot_balances.push(json!({
                "product_id": product_id,
                "type": "spot",
                "symbol": symbol,
                "balance": format!("{:.6}", amount),
            }));
        }
    }

    // Parse health (index 0 = initial health, index 1 = maintenance health)
    let mut health_info = json!({});
    if let Some(healths) = data["healths"].as_array() {
        if let Some(initial) = healths.get(0) {
            let assets_x18 = initial["assets"].as_str().unwrap_or("0");
            let liabilities_x18 = initial["liabilities"].as_str().unwrap_or("0");
            health_info = json!({
                "initial_health_assets": format!("{:.6}", x18_to_f64(assets_x18)),
                "initial_health_liabilities": format!("{:.6}", x18_to_f64(liabilities_x18)),
                "maintenance_health_assets": healths.get(1)
                    .and_then(|h| h["assets"].as_str())
                    .map(|v| format!("{:.6}", x18_to_f64(v)))
                    .unwrap_or_else(|| "0.000000".to_string()),
                "maintenance_health_liabilities": healths.get(1)
                    .and_then(|h| h["liabilities"].as_str())
                    .map(|v| format!("{:.6}", x18_to_f64(v)))
                    .unwrap_or_else(|| "0.000000".to_string()),
            });
        }
    }

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chain_id": chain_id,
        "address": wallet_addr,
        "subaccount": subaccount,
        "perp_positions": perp_positions,
        "spot_balances": spot_balances,
        "health": health_info,
        "note": "unrealized_pnl_approx is v_quote_balance (approximate, mark-price adjusted PnL requires current mark price)"
    }))
}
