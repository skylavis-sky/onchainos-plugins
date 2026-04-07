/// get-markets: List all Exactly Protocol markets with rates and liquidity.
///
/// Calls Previewer.exactly(address(0)) to get all market data without user-specific positions.
/// Displays: market address, asset symbol, total supply, total borrow, utilization.

use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::previewer;

pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let zero_addr = "0x0000000000000000000000000000000000000000";

    eprintln!("Fetching markets from Previewer on chain {}...", cfg.name);

    let result = previewer::get_markets(cfg.previewer, cfg.rpc_url, cfg, Some(zero_addr)).await?;

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chainId": chain_id,
        "previewer": cfg.previewer,
        "markets": result["markets"],
        "marketCount": result["marketCount"],
        "note": "Fixed-rate pools (maturities) available via get-position with a wallet address."
    }))
}
