/// `uniswap-v3 swap` — exact-input single-hop swap via SwapRouter02.exactInputSingle.
///
/// Workflow:
/// 1. dry_run guard — early return before wallet resolution if dry_run
/// 2. Resolve token metadata and quote
/// 3. Check allowance; approve SwapRouter02 if needed (with wait_for_tx)
/// 4. Execute exactInputSingle with --force

use anyhow::Result;

pub struct SwapArgs {
    pub token_in: String,
    pub token_out: String,
    pub amount: String,
    pub slippage_bps: u64,
    pub chain: u64,
    pub fee: Option<u32>,
    pub dry_run: bool,
}

pub async fn run(args: SwapArgs) -> Result<()> {
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

    // Find best fee tier and quote
    let fee_tiers: Vec<u32> = if let Some(f) = args.fee {
        vec![f]
    } else {
        vec![100, 500, 3000, 10000]
    };

    let mut best_out = 0u128;
    let mut best_fee = 0u32;

    for fee in &fee_tiers {
        match crate::rpc::get_pool_address(cfg.factory, &in_addr, &out_addr, *fee, cfg.rpc_url)
            .await
        {
            Ok(None) | Err(_) => continue,
            Ok(Some(_)) => {}
        }

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
            _ => {}
        }
    }

    if best_out == 0 {
        anyhow::bail!(
            "No liquidity found for {}/{} on chain {}. Use `get-pools` to verify pools exist.",
            symbol_in,
            symbol_out,
            args.chain
        );
    }

    // Apply slippage
    let slippage_factor = 1.0 - (args.slippage_bps as f64 / 10000.0);
    let amount_out_minimum = (best_out as f64 * slippage_factor) as u128;

    let amount_out_human = best_out as f64 / 10f64.powi(decimals_out as i32);
    let amount_out_min_human = amount_out_minimum as f64 / 10f64.powi(decimals_out as i32);
    let fee_pct = best_fee as f64 / 10000.0;

    println!("Swap (Uniswap V3, chain {}):", args.chain);
    println!("  Input:            {} {}", args.amount, symbol_in);
    println!("  Expected output:  {:.6} {}", amount_out_human, symbol_out);
    println!(
        "  Minimum output:   {:.6} {} ({} bps slippage)",
        amount_out_min_human, symbol_out, args.slippage_bps
    );
    println!("  Fee tier:         {}%", fee_pct);
    println!("  SwapRouter02:     {}", cfg.swap_router02);
    println!();
    println!("NOTE: This will submit two transactions (approve + swap). Confirm before proceeding.");

    // ── dry_run guard: resolve wallet only after this point ──────────────────

    if args.dry_run {
        let approve_data = crate::calldata::encode_approve_max(cfg.swap_router02)?;
        let swap_data = crate::calldata::encode_exact_input_single(
            &in_addr,
            &out_addr,
            best_fee,
            "0x0000000000000000000000000000000000000000",
            amount_in,
            amount_out_minimum,
        )?;
        println!("[dry-run] Step 1 — Approve {} for SwapRouter02:", symbol_in);
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, in_addr, approve_data
        );
        println!("[dry-run] Step 2 — exactInputSingle:");
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, cfg.swap_router02, swap_data
        );
        println!("\nDry-run complete. No transactions submitted.");
        return Ok(());
    }

    // Resolve real wallet address (after dry_run guard)
    let wallet_addr = crate::onchainos::resolve_wallet(args.chain)?;
    if wallet_addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address for chain {}. Is the wallet logged in?", args.chain);
    }

    // Step 1: Check allowance; approve SwapRouter02 if needed
    println!("Step 1: Checking {} allowance for SwapRouter02...", symbol_in);
    let allowance = crate::rpc::get_allowance(&in_addr, &wallet_addr, cfg.swap_router02, cfg.rpc_url)
        .await
        .unwrap_or(0);

    if allowance < amount_in {
        println!(
            "  Allowance insufficient ({}), approving SwapRouter02 for max {}...",
            allowance, symbol_in
        );
        let approve_data = crate::calldata::encode_approve_max(cfg.swap_router02)?;
        let approve_result = crate::onchainos::wallet_contract_call(
            args.chain,
            &in_addr,
            &approve_data,
            None,
            None,
            true,  // --force
            false, // not dry_run
        )
        .await?;
        let approve_hash = crate::onchainos::extract_tx_hash(&approve_result);
        println!("  Approve tx: {}", approve_hash);
        // Wait for approve receipt before submitting swap
        crate::onchainos::wait_for_tx(&approve_hash, cfg.rpc_url).await?;
        println!("  Approve confirmed.");
    } else {
        println!("  Allowance sufficient ({}), skipping approve.", allowance);
    }

    // Step 2: Execute swap
    println!("\nStep 2: Executing exactInputSingle via SwapRouter02...");
    println!("  Recipient: {}", wallet_addr);

    let swap_data = crate::calldata::encode_exact_input_single(
        &in_addr,
        &out_addr,
        best_fee,
        &wallet_addr,
        amount_in,
        amount_out_minimum,
    )?;

    let swap_result = crate::onchainos::wallet_contract_call(
        args.chain,
        cfg.swap_router02,
        &swap_data,
        None,
        None,
        true,  // --force required for all DEX writes
        false,
    )
    .await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&swap_result);
    let explorer = crate::config::explorer_url(args.chain, &tx_hash);

    println!("  Swap tx: {}", tx_hash);
    println!("\nSwap submitted successfully!");
    println!("  Swapped {} {} -> ~{:.6} {}", args.amount, symbol_in, amount_out_human, symbol_out);
    println!("  View: {}", explorer);

    Ok(())
}
