use serde_json::{json, Value};
use crate::config::POOLS;
use crate::rpc;

/// List all 4 Ion Protocol pools with current borrow APY and total supply (TVL).
///
/// For each pool:
///   - getCurrentBorrowRate(0) -> per-second RAY rate -> annualized APY %
///   - totalSupply() -> total lend tokens supplied (WAD)
///   - rate(0) -> accumulated rate for debt calculation
pub async fn run() -> anyhow::Result<Value> {
    let mut pools_data: Vec<Value> = Vec::new();

    for pool in POOLS {
        let pool_info = fetch_pool_info(pool).await;
        pools_data.push(pool_info);
    }

    Ok(json!({
        "ok": true,
        "chain": "Ethereum Mainnet",
        "chainId": 1,
        "protocol": "Ion Protocol",
        "description": "CDP-style lending for LRT/LST collateral. Supply wstETH/WETH to earn yield, or deposit LRT collateral to borrow.",
        "poolCount": pools_data.len(),
        "pools": pools_data
    }))
}

async fn fetch_pool_info(pool: &crate::config::PoolConfig) -> Value {
    let rpc_url = crate::config::RPC_URL;

    // Get borrow rate
    let borrow_apy_pct = match rpc::get_current_borrow_rate(pool.ion_pool, pool.ilk_index).await {
        Ok((borrow_rate, _reserve_factor)) => {
            rpc::borrow_rate_to_apy_pct(borrow_rate)
        }
        Err(e) => {
            eprintln!("Warning: could not get borrow rate for {}: {}", pool.name, e);
            -1.0
        }
    };

    // Get accumulated rate
    let rate_ray = match rpc::get_rate(pool.ion_pool, pool.ilk_index).await {
        Ok(r) => r,
        Err(_) => crate::config::RAY,
    };

    // Get total lender supply (TVL)
    let total_supply_wad = match rpc::get_total_supply(pool.ion_pool).await {
        Ok(s) => s,
        Err(_) => 0,
    };

    let total_supply_human = total_supply_wad as f64 / 1e18;
    let borrow_apy_str = if borrow_apy_pct >= 0.0 {
        format!("{:.4}%", borrow_apy_pct)
    } else {
        "unavailable".to_string()
    };

    // Lend APY is not directly returned; borrow APY is what lenders earn (minus reserve factor)
    // For display purposes, show borrow APY as the rate lenders receive
    let _ = rpc_url;
    let _ = rate_ray;

    json!({
        "name": pool.name,
        "ionPool": pool.ion_pool,
        "gemJoin": pool.gem_join,
        "collateral": {
            "symbol": pool.collateral_symbol,
            "address": pool.collateral
        },
        "lendToken": {
            "symbol": pool.lend_symbol,
            "address": pool.lend_token
        },
        "ilkIndex": pool.ilk_index,
        "borrowApy": borrow_apy_str,
        "totalLendSupply": format!("{:.6} {}", total_supply_human, pool.lend_symbol),
        "totalLendSupplyWad": total_supply_wad.to_string(),
        "note": "borrowApy shown is the per-second borrow rate annualized (linear approximation). Lend APY is slightly lower after reserve factor."
    })
}
