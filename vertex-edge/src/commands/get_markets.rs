/// get-markets: Query all Vertex Edge markets (spot + perp products).
///
/// Makes two calls:
/// 1. POST /query {"type": "all_products"} — returns all products with oracle prices
/// 2. POST /query {"type": "symbols"} — returns symbol names and funding rates
///
/// Merges results by product_id and outputs a combined market list.

use anyhow::Context;
use serde_json::{json, Value};

use crate::api::{query_all_products, query_symbols, x18_to_f64};
use crate::config::get_chain_config;

pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Fetch all products (prices, open interest)
    let products_resp = query_all_products(cfg.gateway_url)
        .await
        .context("Failed to query all_products")?;

    // Fetch symbols for perps (funding rates, symbol names)
    let symbols_resp = query_symbols(cfg.gateway_url, Some("perp"))
        .await
        .context("Failed to query symbols")?;

    // Build a map of product_id -> symbol info
    let empty_map = serde_json::Map::new();
    let symbols_map = symbols_resp["data"]["symbols"]
        .as_object()
        .unwrap_or(&empty_map);

    let mut markets: Vec<Value> = Vec::new();

    // Process spot products
    if let Some(spot_products) = products_resp["data"]["spot_products"].as_array() {
        for product in spot_products {
            let product_id = product["product_id"].as_u64().unwrap_or(0) as u32;
            let oracle_price_x18 = product["oracle_price_x18"]
                .as_str()
                .unwrap_or("0");
            let oracle_price = x18_to_f64(oracle_price_x18);

            markets.push(json!({
                "product_id": product_id,
                "type": "spot",
                "symbol": symbols_map
                    .get(&product_id.to_string())
                    .and_then(|s| s["symbol"].as_str())
                    .unwrap_or("UNKNOWN"),
                "oracle_price_usd": format!("{:.6}", oracle_price),
                "long_weight_initial": product["risk"]["long_weight_initial"].as_str().unwrap_or("0"),
                "short_weight_initial": product["risk"]["short_weight_initial"].as_str().unwrap_or("0"),
            }));
        }
    }

    // Process perp products
    if let Some(perp_products) = products_resp["data"]["perp_products"].as_array() {
        for product in perp_products {
            let product_id = product["product_id"].as_u64().unwrap_or(0) as u32;
            let oracle_price_x18 = product["oracle_price_x18"]
                .as_str()
                .unwrap_or("0");
            let oracle_price = x18_to_f64(oracle_price_x18);

            let open_interest_x18 = product["state"]["open_interest_x18"]
                .as_str()
                .unwrap_or("0");
            let open_interest = x18_to_f64(open_interest_x18);

            let funding_rate_x18 = product["state"]["cumulative_funding_long_x18"]
                .as_str()
                .unwrap_or("0");

            let symbol_info = symbols_map.get(&product_id.to_string());
            let symbol_name = symbol_info
                .and_then(|s| s["symbol"].as_str())
                .unwrap_or("UNKNOWN");

            markets.push(json!({
                "product_id": product_id,
                "type": "perp",
                "symbol": symbol_name,
                "oracle_price_usd": format!("{:.6}", oracle_price),
                "open_interest": format!("{:.4}", open_interest),
                "cumulative_funding_long_x18": funding_rate_x18,
                "long_weight_initial": product["risk"]["long_weight_initial"].as_str().unwrap_or("0"),
                "short_weight_initial": product["risk"]["short_weight_initial"].as_str().unwrap_or("0"),
            }));
        }
    }

    // Sort by product_id
    markets.sort_by_key(|m| m["product_id"].as_u64().unwrap_or(0));

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chain_id": chain_id,
        "market_count": markets.len(),
        "markets": markets
    }))
}
