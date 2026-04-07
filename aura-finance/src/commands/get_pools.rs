use anyhow::Result;
use clap::Args;
use crate::{api, config, rpc};

#[derive(Args, Debug)]
pub struct GetPoolsArgs {
    /// Maximum number of pools to return (default: 10)
    #[arg(long, default_value = "10")]
    pub limit: usize,
}

pub async fn run(args: GetPoolsArgs, chain_id: u64) -> Result<()> {
    // Fetch Balancer pool data for APY/TVL enrichment (best-effort)
    let balancer_pools = api::fetch_balancer_pools(chain_id).await.unwrap_or_else(|e| {
        eprintln!("Warning: Balancer API unavailable ({}); APY data will be missing", e);
        Vec::new()
    });

    // On-chain: get total pool count from Booster
    let total = rpc::booster_pool_length(config::BOOSTER).await.unwrap_or(0);

    // Iterate the most-recent MAX_POOLS (highest PIDs) — avoids RPC waterfall
    let start_pid = if total > config::MAX_POOLS { total - config::MAX_POOLS } else { 0 };
    let mut summaries: Vec<api::AuraPoolSummary> = Vec::new();

    for pid in start_pid..total {
        match rpc::booster_pool_info(config::BOOSTER, pid).await {
            Ok((lp_token, crv_rewards, shutdown)) => {
                if shutdown {
                    continue;
                }
                // Try to match Balancer pool by lptoken address
                let lp_lower = lp_token.to_lowercase();
                let (tokens, tvl_usd) = balancer_pools.iter()
                    .find(|p| p.address.to_lowercase() == lp_lower)
                    .map(|p| {
                        let syms: Vec<String> = p.tokens.iter().map(|t| t.symbol.clone()).collect();
                        let tvl = p.total_liquidity.as_ref()
                            .and_then(|v| v.as_f64())
                            .or_else(|| p.total_liquidity.as_ref().and_then(|v| v.as_str()).and_then(|s| s.parse().ok()))
                            .map(|v| format!("${:.0}", v))
                            .unwrap_or_else(|| "N/A".to_string());
                        (syms, tvl)
                    })
                    .unwrap_or_else(|| (Vec::new(), "N/A".to_string()));

                summaries.push(api::AuraPoolSummary {
                    aura_pid: pid,
                    lp_token,
                    crv_rewards,
                    tokens,
                    tvl_usd,
                    shutdown,
                });
            }
            Err(e) => {
                eprintln!("Warning: failed to fetch poolInfo for pid {}: {}", pid, e);
            }
        }
    }

    // Sort by TVL descending
    summaries.sort_by(|a, b| {
        let a_val: f64 = a.tvl_usd.trim_start_matches('$').replace(',', "").parse().unwrap_or(0.0);
        let b_val: f64 = b.tvl_usd.trim_start_matches('$').replace(',', "").parse().unwrap_or(0.0);
        b_val.partial_cmp(&a_val).unwrap_or(std::cmp::Ordering::Equal)
    });

    let shown: Vec<&api::AuraPoolSummary> = summaries.iter().take(args.limit).collect();

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "total_pool_count": total,
            "scanned": summaries.len(),
            "shown": shown.len(),
            "note": "Only active (non-shutdown) pools from the most recent 50 are shown. Use --pool-id to query a specific pool.",
            "pools": shown
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
