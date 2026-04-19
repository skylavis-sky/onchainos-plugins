/// `uniswap-v3 get-pools` — list pools for a token pair via UniswapV3Factory.
/// Queries all four fee tiers, returns pool addresses, liquidity, and current price.
/// Read-only — no transactions required.

use anyhow::Result;

pub struct GetPoolsArgs {
    pub token_a: String,
    pub token_b: String,
    pub chain: u64,
}

pub async fn run(args: GetPoolsArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    let addr_a = crate::config::resolve_token_address(&args.token_a, args.chain)?;
    let addr_b = crate::config::resolve_token_address(&args.token_b, args.chain)?;

    let sym_a = crate::rpc::get_symbol(&addr_a, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| args.token_a.clone());
    let sym_b = crate::rpc::get_symbol(&addr_b, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| args.token_b.clone());

    println!(
        "Pools for {}/{} on chain {} (Uniswap V3):",
        sym_a, sym_b, args.chain
    );
    println!("Factory: {}", cfg.factory);
    println!();
    println!(
        "{:<8} {:<44} {:>14} {:>12}",
        "Fee", "Pool Address", "Liquidity", "sqrtPrice"
    );
    println!("{}", "-".repeat(82));

    let fee_tiers = [100u32, 500, 3000, 10000];
    let mut found = 0;

    for fee in fee_tiers {
        match crate::rpc::get_pool_address(cfg.factory, &addr_a, &addr_b, fee, cfg.rpc_url).await
        {
            Ok(Some(pool_addr)) => {
                found += 1;
                let (sqrt_price, tick) = crate::rpc::get_slot0(&pool_addr, cfg.rpc_url)
                    .await
                    .unwrap_or((0, 0));
                let liquidity = crate::rpc::get_pool_liquidity(&pool_addr, cfg.rpc_url)
                    .await
                    .unwrap_or(0);

                let price = if sqrt_price > 0 {
                    let sq = sqrt_price as f64 / 2f64.powi(96);
                    format!("{:.6}", sq * sq)
                } else {
                    "N/A".to_string()
                };

                println!(
                    "{:<8} {:<44} {:>14} {:>12}",
                    format!("{:.2}%", fee as f64 / 10000.0),
                    pool_addr,
                    liquidity,
                    price,
                );
                println!("         tick: {}", tick);
            }
            Ok(None) => {
                // Pool not deployed for this fee tier — skip silently
            }
            Err(e) => {
                eprintln!("  fee={}: factory error: {}", fee, e);
            }
        }
    }

    if found == 0 {
        println!(
            "No pools found for {}/{} on chain {}.",
            sym_a, sym_b, args.chain
        );
        println!("Verify the token addresses are correct.");
    } else {
        println!("\nFound {} pool(s).", found);
    }

    Ok(())
}
