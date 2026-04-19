/// `uniswap-v3 add-liquidity` — mint a new V3 LP position via NonfungiblePositionManager.mint.
///
/// Workflow:
/// 1. dry_run guard — early return before wallet resolution if dry_run
/// 2. Sort token addresses (token0 < token1 required by protocol)
/// 3. Verify pool exists via Factory.getPool
/// 4. Check allowance for token0; approve NFPM if needed (with wait_for_tx)
/// 5. Check allowance for token1; approve NFPM if needed (with wait_for_tx)
/// 6. Execute mint with --force

use anyhow::Result;

pub struct AddLiquidityArgs {
    pub token_a: String,
    pub token_b: String,
    pub fee: u32,
    pub amount_a: String,
    pub amount_b: String,
    pub tick_lower: Option<i32>,
    pub tick_upper: Option<i32>,
    pub slippage_bps: u64,
    pub chain: u64,
    pub dry_run: bool,
}

pub async fn run(args: AddLiquidityArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    let addr_a = crate::config::resolve_token_address(&args.token_a, args.chain)?;
    let addr_b = crate::config::resolve_token_address(&args.token_b, args.chain)?;

    // Sort tokens: token0 < token1 (required by V3 NonfungiblePositionManager)
    let (token0, token1) = crate::calldata::sort_tokens(&addr_a, &addr_b)?;
    let (amount0_str, amount1_str) = if token0 == addr_a.as_str() {
        (args.amount_a.as_str(), args.amount_b.as_str())
    } else {
        (args.amount_b.as_str(), args.amount_a.as_str())
    };

    let decimals0 = crate::rpc::get_decimals(token0, cfg.rpc_url)
        .await
        .unwrap_or(18);
    let decimals1 = crate::rpc::get_decimals(token1, cfg.rpc_url)
        .await
        .unwrap_or(18);
    let sym0 = crate::rpc::get_symbol(token0, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| token0.to_string());
    let sym1 = crate::rpc::get_symbol(token1, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| token1.to_string());

    let amount0_desired = crate::config::human_to_minimal(amount0_str, decimals0)?;
    let amount1_desired = crate::config::human_to_minimal(amount1_str, decimals1)?;

    // Validate fee tier
    let spacing = crate::config::tick_spacing(args.fee)?;

    // Determine tick range (user-supplied or full-range default)
    let (tick_lower, tick_upper) = match (args.tick_lower, args.tick_upper) {
        (Some(lo), Some(hi)) => {
            // Validate tick alignment
            if lo % spacing != 0 || hi % spacing != 0 {
                anyhow::bail!(
                    "Ticks must be multiples of tickSpacing ({}) for fee tier {}. Got tickLower={}, tickUpper={}",
                    spacing, args.fee, lo, hi
                );
            }
            if lo >= hi {
                anyhow::bail!("tickLower ({}) must be less than tickUpper ({})", lo, hi);
            }
            (lo, hi)
        }
        _ => crate::config::full_range_ticks(args.fee)?,
    };

    // Apply slippage to minimums
    let slippage_factor = (1.0 - (args.slippage_bps as f64 / 10000.0)).max(0.0);
    let amount0_min = (amount0_desired as f64 * slippage_factor) as u128;
    let amount1_min = (amount1_desired as f64 * slippage_factor) as u128;

    // Deadline: 5 minutes from now
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + 300)
        .unwrap_or(9_999_999_999);

    println!("Add Liquidity (Uniswap V3, chain {}):", args.chain);
    println!("  token0:           {} {}", amount0_str, sym0);
    println!("  token1:           {} {}", amount1_str, sym1);
    println!("  Fee tier:         {}%", args.fee as f64 / 10000.0);
    println!("  Tick range:       {} to {}", tick_lower, tick_upper);
    println!(
        "  Slippage:         {} bps ({:.2}%)",
        args.slippage_bps,
        args.slippage_bps as f64 / 100.0
    );
    println!("  NFPM:             {}", cfg.nfpm);
    println!();
    println!("NOTE: This will submit up to three transactions (approve0, approve1, mint). Confirm before proceeding.");

    // ── dry_run guard: resolve wallet only after this point ──────────────────

    if args.dry_run {
        let zero_addr = "0x0000000000000000000000000000000000000000";
        let approve0_data = crate::calldata::encode_approve_max(cfg.nfpm)?;
        let approve1_data = crate::calldata::encode_approve_max(cfg.nfpm)?;
        let mint_data = crate::calldata::encode_mint(
            token0,
            token1,
            args.fee,
            tick_lower,
            tick_upper,
            amount0_desired,
            amount1_desired,
            amount0_min,
            amount1_min,
            zero_addr,
            deadline,
        )?;

        println!("[dry-run] Step 1 — Approve {} for NFPM:", sym0);
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, token0, approve0_data
        );
        println!("[dry-run] Step 2 — Approve {} for NFPM:", sym1);
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, token1, approve1_data
        );
        println!("[dry-run] Step 3 — mint:");
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, cfg.nfpm, mint_data
        );
        println!("\nDry-run complete. No transactions submitted.");
        return Ok(());
    }

    // Resolve wallet address (after dry_run guard)
    let wallet_addr = crate::onchainos::resolve_wallet(args.chain)?;
    if wallet_addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address for chain {}", args.chain);
    }

    // Verify pool exists
    match crate::rpc::get_pool_address(cfg.factory, token0, token1, args.fee, cfg.rpc_url).await? {
        None => {
            anyhow::bail!(
                "No {}/{} {}% pool exists on chain {}. Try a different fee tier.",
                sym0, sym1, args.fee as f64 / 10000.0, args.chain
            );
        }
        Some(_) => {}
    }

    // Step 1: Approve token0 for NFPM (if needed)
    println!("Step 1: Checking {} allowance for NFPM...", sym0);
    let allowance0 = crate::rpc::get_allowance(token0, &wallet_addr, cfg.nfpm, cfg.rpc_url)
        .await
        .unwrap_or(0);

    if allowance0 < amount0_desired {
        println!("  Approving NFPM for max {}...", sym0);
        let approve0_data = crate::calldata::encode_approve_max(cfg.nfpm)?;
        let r = crate::onchainos::wallet_contract_call(
            args.chain, token0, &approve0_data, None, None, true, false,
        )
        .await?;
        let hash = crate::onchainos::extract_tx_hash(&r);
        println!("  Approve {} tx: {}", sym0, hash);
        crate::onchainos::wait_for_tx(&hash, cfg.rpc_url).await?;
        println!("  Approve {} confirmed.", sym0);
    } else {
        println!("  {} allowance sufficient, skipping approve.", sym0);
    }

    // Small delay between sequential on-chain calls
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Step 2: Approve token1 for NFPM (if needed)
    println!("\nStep 2: Checking {} allowance for NFPM...", sym1);
    let allowance1 = crate::rpc::get_allowance(token1, &wallet_addr, cfg.nfpm, cfg.rpc_url)
        .await
        .unwrap_or(0);

    if allowance1 < amount1_desired {
        println!("  Approving NFPM for max {}...", sym1);
        let approve1_data = crate::calldata::encode_approve_max(cfg.nfpm)?;
        let r = crate::onchainos::wallet_contract_call(
            args.chain, token1, &approve1_data, None, None, true, false,
        )
        .await?;
        let hash = crate::onchainos::extract_tx_hash(&r);
        println!("  Approve {} tx: {}", sym1, hash);
        crate::onchainos::wait_for_tx(&hash, cfg.rpc_url).await?;
        println!("  Approve {} confirmed.", sym1);
    } else {
        println!("  {} allowance sufficient, skipping approve.", sym1);
    }

    // Step 3: Mint position
    println!("\nStep 3: Minting LP position via NonfungiblePositionManager.mint...");
    println!("  Recipient: {}", wallet_addr);
    println!("  Tick range: [{}, {}]", tick_lower, tick_upper);

    let mint_data = crate::calldata::encode_mint(
        token0,
        token1,
        args.fee,
        tick_lower,
        tick_upper,
        amount0_desired,
        amount1_desired,
        amount0_min,
        amount1_min,
        &wallet_addr,
        deadline,
    )?;

    let r = crate::onchainos::wallet_contract_call(
        args.chain, cfg.nfpm, &mint_data, None, None, true, false,
    )
    .await?;

    let tx_hash = crate::onchainos::extract_tx_hash(&r);
    let explorer = crate::config::explorer_url(args.chain, &tx_hash);

    println!("  Mint tx: {}", tx_hash);
    println!("\nLP position minted successfully!");
    println!("  View: {}", explorer);
    println!("  Use `get-positions --owner {}` to view your new position.", wallet_addr);

    Ok(())
}
