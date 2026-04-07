use serde_json::{json, Value};

use crate::config::{get_chain_config, get_known_markets};
use crate::onchainos;
use crate::rpc;

/// Get user positions across all known TermMax markets.
///
/// For each known market:
///   - Check FT balance (lend position)
///   - Check GT NFT balance (borrow position) via viewer
///   - Use TermMaxViewer.getPositionDetail for full detail
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let markets = get_known_markets(chain_id);

    // Resolve wallet address — must be after early returns
    let wallet = match from {
        Some(addr) => addr.to_string(),
        None => onchainos::resolve_wallet(chain_id)
            .map_err(|e| anyhow::anyhow!("Could not resolve wallet: {}", e))?,
    };

    if markets.is_empty() {
        return Ok(json!({
            "ok": true,
            "wallet": wallet,
            "chain_id": chain_id,
            "positions": [],
            "note": "No known markets on this chain. Try --chain 42161 for Arbitrum."
        }));
    }

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut positions: Vec<Value> = Vec::new();

    for market in &markets {
        // Fetch tokens for this market
        let tokens = match rpc::market_tokens(market.address, cfg.rpc_url).await {
            Ok(t) => t,
            Err(e) => {
                eprintln!("Warning: could not fetch tokens for {}: {}", market.address, e);
                continue;
            }
        };

        // Check FT balance (lend position)
        let ft_balance = rpc::erc20_balance(&tokens.ft, &wallet, cfg.rpc_url)
            .await
            .unwrap_or(0);

        // Try viewer for detailed position (includes GT/borrow info)
        let position_detail = rpc::viewer_get_position(
            cfg.termmax_viewer,
            market.address,
            &wallet,
            cfg.rpc_url,
        )
        .await
        .unwrap_or_default();

        let underlying_decimals =
            crate::config::token_decimals_by_symbol(market.underlying_symbol);
        let collateral_decimals =
            crate::config::token_decimals_by_symbol(market.collateral_symbol);

        let ft_human = ft_balance as f64 / 10f64.powi(underlying_decimals as i32);
        let underlying_human =
            position_detail.underlying_balance as f64 / 10f64.powi(underlying_decimals as i32);
        let collateral_human =
            position_detail.collateral_balance as f64 / 10f64.powi(collateral_decimals as i32);

        // Only include markets where user has a position
        if ft_balance == 0 && position_detail.collateral_balance == 0 {
            continue;
        }

        let is_matured = market.maturity_ts < now_ts;

        let position_type = if ft_balance > 0 {
            "lend"
        } else if position_detail.collateral_balance > 0 {
            "borrow"
        } else {
            "unknown"
        };

        let mut pos = json!({
            "market": market.address,
            "collateral_symbol": market.collateral_symbol,
            "underlying_symbol": market.underlying_symbol,
            "maturity_date": market.maturity_label,
            "maturity_ts": market.maturity_ts,
            "is_matured": is_matured,
            "position_type": position_type,
        });

        if ft_balance > 0 {
            pos["ft_balance"] = json!(format!("{:.6} {} (FT)", ft_human, market.underlying_symbol));
            pos["ft_balance_raw"] = json!(ft_balance.to_string());
            pos["ft_address"] = json!(tokens.ft);
            if is_matured {
                pos["action_available"] = json!("redeem");
                pos["redeem_cmd"] = json!(format!(
                    "term-structure redeem --chain {} --market {} --all",
                    chain_id, market.address
                ));
            } else {
                pos["action_available"] = json!("hold_until_maturity or early_exit");
            }
        }

        if position_detail.collateral_balance > 0 {
            pos["collateral_balance"] = json!(format!(
                "{:.6} {}",
                collateral_human, market.collateral_symbol
            ));
            pos["underlying_balance"] = json!(format!(
                "{:.6} {}",
                underlying_human, market.underlying_symbol
            ));
            pos["action_available"] = json!("repay");
        }

        positions.push(pos);
    }

    Ok(json!({
        "ok": true,
        "wallet": wallet,
        "chain_id": chain_id,
        "chain_name": cfg.name,
        "positions": positions,
        "total_positions": positions.len(),
        "note": "FT tokens = lend positions (redeem at maturity). GT NFT = borrow positions (repay before maturity to avoid liquidation)."
    }))
}
