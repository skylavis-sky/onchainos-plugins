/// get-position: Show user's current positions across all Exactly Protocol markets.
///
/// Calls Previewer.exactly(walletAddress) to get user-specific position data.
/// Displays: floating deposits, floating borrows, isCollateral status, health info.

use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos;
#[allow(unused_imports)]
use crate::previewer;

pub async fn run(
    chain_id: u64,
    from: Option<&str>,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve wallet address
    let wallet = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    eprintln!("Fetching positions for {} on chain {}...", wallet, cfg.name);

    let result = previewer::get_markets(cfg.previewer, cfg.rpc_url, cfg, Some(&wallet)).await?;

    // Filter to markets where user has positions
    let markets = result["markets"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut positions: Vec<Value> = Vec::new();

    for m in &markets {
        let has_deposit = m["userHasFloatingDeposit"].as_bool().unwrap_or(false);
        let has_borrow = m["userHasFloatingBorrow"].as_bool().unwrap_or(false);
        let is_collateral = m["isCollateral"].as_bool().unwrap_or(false);

        if has_deposit || has_borrow || is_collateral {
            positions.push(m.clone());
        }
    }

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "wallet": wallet,
        "positionCount": positions.len(),
        "positions": positions,
        "allMarkets": markets,
        "note": "isCollateral=true means asset counts toward borrowing power. Call enter-market to enable collateral."
    }))
}
