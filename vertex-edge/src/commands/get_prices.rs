/// get-prices: Query current mark prices and index prices for Vertex Edge perp markets.
///
/// Queries the archive (indexer) for perp_prices which includes:
///   index_price_x18: oracle/index price (reference price)
///   mark_price_x18: fair mark price used for funding and liquidation
///   update_time: last update timestamp
///
/// By default queries common perp markets (BTC=2, ETH=4, ARB=6, SOL=12).
/// Use --product-ids to specify custom product IDs.

use anyhow::Context;
use serde_json::{json, Value};

use crate::api::{query_market_prices, query_perp_prices, x18_to_f64};
use crate::config::get_chain_config;

/// Default perp product IDs to query if none specified
const DEFAULT_PERP_IDS: &[u32] = &[2, 4, 6, 8, 10, 12, 14, 16, 18, 20];

pub async fn run(
    chain_id: u64,
    product_ids: Option<Vec<u32>>,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    let ids = product_ids
        .unwrap_or_else(|| DEFAULT_PERP_IDS.to_vec());

    // Query perp prices from archive (index + mark prices)
    let archive_resp = query_perp_prices(cfg.archive_url, &ids)
        .await
        .context("Failed to query perp_prices from archive")?;

    // Also query market prices from engine gateway (bid/ask)
    let engine_resp = query_market_prices(cfg.gateway_url, &ids)
        .await
        .context("Failed to query market_prices from engine")?;

    // Parse archive perp prices: map of product_id -> {index_price_x18, mark_price_x18, ...}
    let mut prices: Vec<Value> = Vec::new();

    // Archive response may be a map of string keys to price objects
    if let Some(price_map) = archive_resp["data"].as_object() {
        for (id_str, price_info) in price_map {
            let product_id: u32 = id_str.parse().unwrap_or(0);

            let index_price_x18 = price_info["index_price_x18"]
                .as_str()
                .unwrap_or("0");
            let mark_price_x18 = price_info["mark_price_x18"]
                .as_str()
                .unwrap_or("0");

            let index_price = x18_to_f64(index_price_x18);
            let mark_price = x18_to_f64(mark_price_x18);

            if index_price == 0.0 && mark_price == 0.0 {
                continue;
            }

            prices.push(json!({
                "product_id": product_id,
                "index_price_usd": format!("{:.6}", index_price),
                "mark_price_usd": format!("{:.6}", mark_price),
                "update_time": price_info["update_time"],
            }));
        }
    }

    // If archive returned array format instead of map
    if prices.is_empty() {
        if let Some(price_array) = archive_resp["data"].as_array() {
            for price_info in price_array {
                let product_id = price_info["product_id"].as_u64().unwrap_or(0) as u32;
                let index_price_x18 = price_info["index_price_x18"].as_str().unwrap_or("0");
                let mark_price_x18 = price_info["mark_price_x18"].as_str().unwrap_or("0");

                prices.push(json!({
                    "product_id": product_id,
                    "index_price_usd": format!("{:.6}", x18_to_f64(index_price_x18)),
                    "mark_price_usd": format!("{:.6}", x18_to_f64(mark_price_x18)),
                    "update_time": price_info["update_time"],
                }));
            }
        }
    }

    // Merge engine market prices (bid/ask) into the result
    let engine_prices_arr = engine_resp["data"]["market_prices"].as_array();
    if let Some(engine_prices) = engine_prices_arr {
        for ep in engine_prices {
            let product_id = ep["product_id"].as_u64().unwrap_or(0) as u32;
            if let Some(existing) = prices.iter_mut().find(|p| {
                p["product_id"].as_u64().unwrap_or(0) as u32 == product_id
            }) {
                existing["bid_price_usd"] = json!(format!(
                    "{:.6}",
                    x18_to_f64(ep["bid_x18"].as_str().unwrap_or("0"))
                ));
                existing["ask_price_usd"] = json!(format!(
                    "{:.6}",
                    x18_to_f64(ep["ask_x18"].as_str().unwrap_or("0"))
                ));
            }
        }
    }

    // Sort by product_id
    prices.sort_by_key(|p| p["product_id"].as_u64().unwrap_or(0));

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chain_id": chain_id,
        "product_ids_queried": ids,
        "prices": prices,
        "note": "index_price = oracle reference price; mark_price = fair value used for funding/liquidation"
    }))
}
