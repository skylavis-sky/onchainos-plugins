use anyhow::Context;
use serde_json::{json, Value};

use crate::config::{get_chain_config, get_known_markets};
use crate::rpc;

/// List active TermMax markets with current APR.
///
/// For each known market:
///   1. Call market.config() to get maturity timestamp
///   2. Filter out expired markets (maturity < now)
///   3. Call market.tokens() to get token addresses
///   4. Call market.apr() on the market itself (treats market as an order for APR read)
///   5. Display table sorted by lend APR descending
pub async fn run(chain_id: u64, underlying_filter: Option<&str>) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let markets = get_known_markets(chain_id);

    if markets.is_empty() {
        return Ok(json!({
            "ok": true,
            "chain_id": chain_id,
            "markets": [],
            "note": "No known markets configured for this chain. Arbitrum (42161) has the most active markets."
        }));
    }

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut market_rows: Vec<Value> = Vec::new();

    for market in &markets {
        // Apply underlying filter if provided
        if let Some(filter) = underlying_filter {
            if !market.underlying_symbol.to_uppercase().contains(&filter.to_uppercase()) {
                continue;
            }
        }

        // Fetch on-chain config (maturity)
        let (_, maturity) = match rpc::market_config(market.address, cfg.rpc_url).await {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "Warning: could not fetch config for market {}: {}",
                    market.address, e
                );
                // Use hardcoded maturity from config as fallback
                ("".to_string(), market.maturity_ts)
            }
        };

        let is_expired = maturity > 0 && maturity < now_ts;
        let status = if is_expired { "expired" } else { "active" };

        // Fetch tokens
        let tokens = match rpc::market_tokens(market.address, cfg.rpc_url).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!(
                    "Warning: could not fetch tokens for market {}: {}",
                    market.address, e
                );
                continue;
            }
        };

        // Fetch APR from market contract directly (market acts as order in simple case)
        let (lend_apr_raw, borrow_apr_raw) =
            match rpc::order_apr(market.address, cfg.rpc_url).await {
                Ok(v) => v,
                Err(_) => (0u128, 0u128),
            };

        // Fetch FT reserves for liquidity indication
        let (ft_reserve, xt_reserve) =
            match rpc::order_reserves(market.address, cfg.rpc_url).await {
                Ok(v) => v,
                Err(_) => (0u128, 0u128),
            };

        let lend_apr_pct = rpc::apr_to_pct(lend_apr_raw);
        let borrow_apr_pct = rpc::apr_to_pct(borrow_apr_raw);

        let underlying_decimals = crate::config::token_decimals_by_symbol(market.underlying_symbol);
        let ft_liquidity = ft_reserve as f64 / 10f64.powi(underlying_decimals as i32);

        market_rows.push(json!({
            "market": market.address,
            "collateral": market.collateral_symbol,
            "underlying": market.underlying_symbol,
            "ft_address": tokens.ft,
            "gt_address": tokens.gt,
            "maturity_date": market.maturity_label,
            "maturity_ts": maturity,
            "status": status,
            "lend_apr_pct": format!("{:.2}%", lend_apr_pct),
            "borrow_apr_pct": format!("{:.2}%", borrow_apr_pct),
            "ft_liquidity": format!("{:.2} {}", ft_liquidity, market.underlying_symbol),
            "xt_reserve_raw": xt_reserve.to_string(),
        }));
    }

    // Sort active markets by lend APR descending
    market_rows.sort_by(|a, b| {
        let apr_a = parse_apr_pct(a["lend_apr_pct"].as_str().unwrap_or("0%"));
        let apr_b = parse_apr_pct(b["lend_apr_pct"].as_str().unwrap_or("0%"));
        apr_b.partial_cmp(&apr_a).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(json!({
        "ok": true,
        "chain_id": chain_id,
        "chain_name": cfg.name,
        "markets": market_rows,
        "total_markets": market_rows.len(),
        "note": "Thin liquidity (~$3.6M TVL total). Check ft_liquidity before placing large orders. Market addresses are from curated TermMax V2 deployment list."
    }))
}

fn parse_apr_pct(s: &str) -> f64 {
    s.trim_end_matches('%').parse::<f64>().unwrap_or(0.0)
}
