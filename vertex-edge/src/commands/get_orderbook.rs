/// get-orderbook: Query orderbook depth for a Vertex Edge market.
///
/// Uses the engine gateway's market_liquidity query:
///   POST /query {"type": "market_liquidity", "product_id": N, "depth": D}
///
/// Also supports the v2 CoinGecko-format endpoint for named markets:
///   GET /v2/orderbook?ticker_id=BTC-PERP_USDC&depth=10
///
/// The --product-id flag takes a numeric product ID directly.
/// The --market flag takes a symbol name like "BTC-PERP" (looks up product_id via symbols query).

use anyhow::Context;
use serde_json::{json, Value};

use crate::api::{engine_query, gateway_v2_get, query_symbols, x18_to_f64};
use crate::config::{get_chain_config, DEFAULT_ORDERBOOK_DEPTH};

/// Resolve a market symbol name to a product_id by querying the symbols endpoint.
async fn resolve_product_id(gateway_url: &str, market: &str) -> anyhow::Result<u32> {
    // Try both "perp" and "spot" symbol types
    for product_type in &["perp", "spot"] {
        let resp = query_symbols(gateway_url, Some(product_type))
            .await
            .context("Failed to query symbols")?;

        let empty_map = serde_json::Map::new();
        let symbols_map = resp["data"]["symbols"]
            .as_object()
            .unwrap_or(&empty_map);

        for (id_str, sym_info) in symbols_map {
            let symbol = sym_info["symbol"].as_str().unwrap_or("");
            // Match case-insensitively, also try without -PERP/_USDC suffix variants
            if symbol.eq_ignore_ascii_case(market) {
                if let Ok(id) = id_str.parse::<u32>() {
                    return Ok(id);
                }
            }
        }
    }
    anyhow::bail!(
        "Market '{}' not found. Use --product-id for a numeric product ID, or check get-markets for valid symbols.",
        market
    )
}

pub async fn run(
    chain_id: u64,
    market: Option<&str>,
    product_id: Option<u32>,
    depth: Option<u32>,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let depth = depth.unwrap_or(DEFAULT_ORDERBOOK_DEPTH);

    // Resolve product_id
    let pid = match (product_id, market) {
        (Some(id), _) => id,
        (None, Some(m)) => resolve_product_id(cfg.gateway_url, m)
            .await
            .with_context(|| format!("Could not resolve market '{}'", m))?,
        (None, None) => anyhow::bail!("Either --market or --product-id must be specified"),
    };

    // Query market liquidity via engine query endpoint
    let resp = engine_query(
        cfg.gateway_url,
        json!({
            "type": "market_liquidity",
            "product_id": pid,
            "depth": depth
        }),
    )
    .await
    .context("Failed to query market_liquidity")?;

    let data = &resp["data"];

    // Parse bids and asks from x18 format
    let parse_levels = |levels: &Value| -> Vec<Value> {
        levels
            .as_array()
            .unwrap_or(&vec![])
            .iter()
            .map(|level| {
                let price_x18 = level.get(0).and_then(|v| v.as_str()).unwrap_or("0");
                let qty_x18 = level.get(1).and_then(|v| v.as_str()).unwrap_or("0");
                json!({
                    "price": format!("{:.6}", x18_to_f64(price_x18)),
                    "quantity": format!("{:.6}", x18_to_f64(qty_x18)),
                    "price_x18": price_x18,
                    "quantity_x18": qty_x18,
                })
            })
            .collect()
    };

    let bids = parse_levels(&data["bids"]);
    let asks = parse_levels(&data["asks"]);

    // Best bid/ask spread
    let best_bid = bids
        .first()
        .and_then(|b| b["price"].as_str())
        .unwrap_or("N/A");
    let best_ask = asks
        .first()
        .and_then(|a| a["price"].as_str())
        .unwrap_or("N/A");

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chain_id": chain_id,
        "product_id": pid,
        "market": market.unwrap_or("unknown"),
        "depth": depth,
        "best_bid": best_bid,
        "best_ask": best_ask,
        "timestamp": data["timestamp"],
        "bids": bids,
        "asks": asks,
    }))
}

/// Fetch orderbook in v2 CoinGecko format by ticker_id (e.g. "BTC-PERP_USDC").
#[allow(dead_code)]
pub async fn run_v2(
    chain_id: u64,
    ticker_id: &str,
    depth: Option<u32>,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let depth = depth.unwrap_or(DEFAULT_ORDERBOOK_DEPTH);
    let depth_str = depth.to_string();

    let resp = gateway_v2_get(
        cfg.gateway_url,
        "/orderbook",
        &[("ticker_id", ticker_id), ("depth", &depth_str)],
    )
    .await
    .with_context(|| format!("Failed to fetch v2 orderbook for {}", ticker_id))?;

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chain_id": chain_id,
        "ticker_id": ticker_id,
        "depth": depth,
        "orderbook": resp
    }))
}
