// list-lsts: enumerate tracked validator LSTs with APY, TVL, and SOL value.
//
// Fetches from Sanctum Extra API in parallel; falls back to hardcoded registry
// if the API is unavailable (403 from some environments).

use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::api;
use crate::config::{self, LST_DECIMALS};

#[derive(Args)]
pub struct ListLstsArgs {
    /// Show all LSTs including INF and wSOL (default: hide meta-tokens)
    #[arg(long, default_value_t = false)]
    pub all: bool,
}

pub async fn run(args: ListLstsArgs) -> Result<Value> {
    let client = reqwest::Client::new();

    // Decide which mints to display
    let registry_mints: Vec<&str> = if args.all {
        config::LSTS.iter().map(|l| l.mint).collect()
    } else {
        // Exclude wSOL and INF (covered by other plugins)
        config::LSTS
            .iter()
            .filter(|l| l.symbol != "wSOL" && l.symbol != "INF")
            .map(|l| l.mint)
            .collect()
    };

    let registry_symbols: Vec<&str> = if args.all {
        config::LSTS.iter().map(|l| l.symbol).collect()
    } else {
        config::LSTS
            .iter()
            .filter(|l| l.symbol != "wSOL" && l.symbol != "INF")
            .map(|l| l.symbol)
            .collect()
    };

    // Fetch APY, TVL, and SOL value in parallel (all non-fatal)
    let (apy_result, tvl_result, sol_value_result) = tokio::join!(
        api::get_apy(&client, &registry_mints),
        api::get_tvl(&client, &registry_mints),
        api::get_sol_value(&client, &registry_mints),
    );

    let apy_map = apy_result.map(|r| r.apys).unwrap_or_default();
    let tvl_map = tvl_result.map(|r| r.tvls).unwrap_or_default();
    let sol_value_map = sol_value_result.map(|r| r.sol_values).unwrap_or_default();

    let mut entries = Vec::new();
    for (symbol, mint) in registry_symbols.iter().zip(registry_mints.iter()) {
        let apy_pct = apy_map.get(*mint).copied();
        let tvl_lamports = tvl_map.get(*mint).and_then(|s| s.parse::<u64>().ok());
        let sol_value_lamports = sol_value_map.get(*mint).and_then(|s| s.parse::<u64>().ok());

        let tvl_sol = tvl_lamports.map(|l| api::atomics_to_ui(l, LST_DECIMALS));
        let sol_per_lst = sol_value_lamports.map(|l| api::atomics_to_ui(l, LST_DECIMALS));

        entries.push(serde_json::json!({
            "symbol": symbol,
            "mint": mint,
            "apy_pct": apy_pct.map(|a| format!("{:.2}%", a * 100.0)).unwrap_or_else(|| "N/A".to_string()),
            "tvl_sol": tvl_sol.map(|t| format!("{:.2}", t)).unwrap_or_else(|| "N/A".to_string()),
            "sol_per_lst": sol_per_lst.map(|v| format!("{:.9}", v)).unwrap_or_else(|| "N/A".to_string()),
        }));
    }

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "lsts": entries,
            "count": entries.len(),
            "note": "mSOL uses Marinade's custom program — use the 'marinade' plugin to stake SOL for mSOL. INF is covered by the 'sanctum-infinity' plugin."
        }
    }))
}
