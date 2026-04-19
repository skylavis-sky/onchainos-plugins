/// `uniswap-v3 get-quote` — get swap quote via QuoterV2 eth_call (read-only).
/// Iterates all four fee tiers, validates pools exist via Factory, returns best output.

use anyhow::Result;

pub struct GetQuoteArgs {
    pub token_in: String,
    pub token_out: String,
    pub amount: String,
    pub chain: u64,
    pub fee: Option<u32>,
}

pub async fn run(args: GetQuoteArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    let in_addr = crate::config::resolve_token_address(&args.token_in, args.chain)?;
    let out_addr = crate::config::resolve_token_address(&args.token_out, args.chain)?;

    let decimals_in = crate::rpc::get_decimals(&in_addr, cfg.rpc_url)
        .await
        .unwrap_or(18);
    let decimals_out = crate::rpc::get_decimals(&out_addr, cfg.rpc_url)
        .await
        .unwrap_or(18);
    let symbol_in = crate::rpc::get_symbol(&in_addr, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| args.token_in.clone());
    let symbol_out = crate::rpc::get_symbol(&out_addr, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| args.token_out.clone());

    let amount_in = crate::config::human_to_minimal(&args.amount, decimals_in)?;

    // Determine fee tiers to try
    let fee_tiers: Vec<u32> = if let Some(f) = args.fee {
        vec![f]
    } else {
        vec![100, 500, 3000, 10000]
    };

    let mut best_out = 0u128;
    let mut best_fee = 0u32;
    let mut errors: Vec<String> = Vec::new();

    for fee in &fee_tiers {
        // Step 1: verify pool exists via Factory.getPool — skip fee tiers with no pool
        match crate::rpc::get_pool_address(cfg.factory, &in_addr, &out_addr, *fee, cfg.rpc_url)
            .await
        {
            Ok(None) => {
                // Pool not deployed for this fee tier
                continue;
            }
            Err(e) => {
                errors.push(format!("fee={}: factory error: {}", fee, e));
                continue;
            }
            Ok(Some(_)) => {}
        }

        // Step 2: get quote from QuoterV2
        match crate::rpc::quote_exact_input_single(
            cfg.quoter_v2,
            &in_addr,
            &out_addr,
            amount_in,
            *fee,
            cfg.rpc_url,
        )
        .await
        {
            Ok(out) if out > best_out => {
                best_out = out;
                best_fee = *fee;
            }
            Ok(_) => {}
            Err(e) => {
                errors.push(format!("fee={}: quote error: {}", fee, e));
            }
        }
    }

    if best_out == 0 {
        eprintln!("No quote found. Errors per fee tier:");
        for e in &errors {
            eprintln!("  {}", e);
        }
        anyhow::bail!(
            "Could not get a quote for {}/{} on chain {}. No pools with liquidity found.",
            symbol_in,
            symbol_out,
            args.chain
        );
    }

    let amount_out_human = best_out as f64 / 10f64.powi(decimals_out as i32);
    let amount_in_human: f64 = args.amount.parse().unwrap_or(0.0);
    let fee_pct = best_fee as f64 / 10000.0;

    println!("Quote (Uniswap V3, chain {}):", args.chain);
    println!("  Input:      {} {}", args.amount, symbol_in);
    println!("  Output:     {:.6} {}", amount_out_human, symbol_out);
    println!("  Fee tier:   {}%", fee_pct);
    if amount_in_human > 0.0 {
        println!(
            "  Rate:       1 {} = {:.6} {}",
            symbol_in,
            amount_out_human / amount_in_human,
            symbol_out
        );
    }
    println!("  QuoterV2:   {}", cfg.quoter_v2);
    println!(
        "\nThis is a read-only quote. No transaction was submitted."
    );

    Ok(())
}
