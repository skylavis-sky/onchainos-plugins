use crate::config::{resolve_token_address, token_decimals, FACTORY_V2, POLYGON_RPC, CHAIN_ID};
use crate::rpc::{factory_get_pair, get_reserves, get_token0};

/// Get the price of tokenA in terms of tokenB, derived from on-chain reserves.
///
/// Accounts for different token decimals (e.g. USDC/USDT = 6 decimals, WMATIC/WETH = 18 decimals).
/// price = (reserveB / 10^decimalsB) / (reserveA / 10^decimalsA)
pub async fn run(token_a: &str, token_b: &str) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = POLYGON_RPC;

    let addr_a = resolve_token_address(token_a, chain_id);
    let addr_b = resolve_token_address(token_b, chain_id);

    let pair = factory_get_pair(FACTORY_V2, &addr_a, &addr_b, rpc).await?;
    if pair == "0x0000000000000000000000000000000000000000" {
        anyhow::bail!("Pair does not exist for {} / {}", token_a, token_b);
    }

    let (r0, r1) = get_reserves(&pair, rpc).await?;
    let token0 = get_token0(&pair, rpc).await?;

    // Determine ordering
    let (reserve_a, reserve_b) = if token0.to_lowercase() == addr_a.to_lowercase() {
        (r0, r1)
    } else {
        (r1, r0)
    };

    if reserve_a == 0 {
        anyhow::bail!("Reserve for {} is zero — pool may be empty", token_a);
    }

    // Get decimals for each token (critical for Polygon where USDC/USDT have 6 decimals)
    let dec_a = token_decimals(&addr_a) as i32;
    let dec_b = token_decimals(&addr_b) as i32;

    // Normalize reserves to human-readable units then compute price
    // price = (reserveB / 10^dec_b) / (reserveA / 10^dec_a)
    //       = reserveB * 10^dec_a / (reserveA * 10^dec_b)   [rearranged to avoid floats where possible]
    let price = (reserve_b as f64 / 10f64.powi(dec_b))
        / (reserve_a as f64 / 10f64.powi(dec_a));

    println!("QuickSwap V2 Price");
    println!("  pair:    {}", pair);
    println!("  {} reserve: {} (raw, {} decimals)", token_a.to_uppercase(), reserve_a, dec_a);
    println!("  {} reserve: {} (raw, {} decimals)", token_b.to_uppercase(), reserve_b, dec_b);
    println!("  1 {} = {:.6} {} (from on-chain reserves)", token_a.to_uppercase(), price, token_b.to_uppercase());

    Ok(())
}
