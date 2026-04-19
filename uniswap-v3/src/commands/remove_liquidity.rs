/// `uniswap-v3 remove-liquidity` — three-step remove via NonfungiblePositionManager.
///
/// Step 1: decreaseLiquidity — withdraw liquidity; tokens become owed.
/// Step 2: collect — transfer owed tokens + fees to recipient wallet.
/// Step 3: burn — (only if all liquidity removed) destroy the NFT.
///
/// Workflow:
/// 1. dry_run guard — early return before wallet resolution if dry_run
/// 2. Fetch position data via positions(tokenId)
/// 3. Verify ownership via ownerOf(tokenId)
/// 4. Execute decreaseLiquidity with --force; wait_for_tx
/// 5. Execute collect with --force; wait_for_tx
/// 6. Execute burn with --force if full removal

use anyhow::Result;

pub struct RemoveLiquidityArgs {
    pub token_id: u128,
    pub liquidity_pct: f64, // 0–100, percentage of position liquidity to remove
    pub chain: u64,
    pub dry_run: bool,
}

pub async fn run(args: RemoveLiquidityArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    if args.liquidity_pct <= 0.0 || args.liquidity_pct > 100.0 {
        anyhow::bail!("liquidity_pct must be between 0 (exclusive) and 100 (inclusive)");
    }

    // ── dry_run guard: resolve wallet only after this point ──────────────────

    if args.dry_run {
        // Use synthetic values to preview calldata
        let deadline = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() + 300)
            .unwrap_or(9_999_999_999);

        let synthetic_liquidity = 1_000_000u128;
        let liquidity_to_remove =
            (synthetic_liquidity as f64 * args.liquidity_pct / 100.0) as u128;
        let zero_addr = "0x0000000000000000000000000000000000000000";

        let decrease_data = crate::calldata::encode_decrease_liquidity(
            args.token_id,
            liquidity_to_remove,
            0,
            0,
            deadline,
        )?;
        let collect_data = crate::calldata::encode_collect(args.token_id, zero_addr)?;
        let burn_data = crate::calldata::encode_burn(args.token_id)?;

        println!("[dry-run] Remove Liquidity (chain {}, position #{}):", args.chain, args.token_id);
        println!("[dry-run] Step 1 — decreaseLiquidity:");
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, cfg.nfpm, decrease_data
        );
        println!("[dry-run] Step 2 — collect:");
        println!(
            "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
            args.chain, cfg.nfpm, collect_data
        );
        if (args.liquidity_pct - 100.0).abs() < f64::EPSILON {
            println!("[dry-run] Step 3 — burn (full removal):");
            println!(
                "  onchainos wallet contract-call --chain {} --to {} --input-data {} --force",
                args.chain, cfg.nfpm, burn_data
            );
        }
        println!("\nDry-run complete. No transactions submitted.");
        return Ok(());
    }

    // Fetch position data
    println!(
        "Fetching position #{} on chain {}...",
        args.token_id, args.chain
    );
    let pos = crate::rpc::get_position(cfg.nfpm, args.token_id, cfg.rpc_url).await?;

    if pos.liquidity == 0 {
        anyhow::bail!(
            "Position #{} has zero liquidity. Nothing to remove.",
            args.token_id
        );
    }

    let sym0 = crate::rpc::get_symbol(&pos.token0, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| pos.token0.clone());
    let sym1 = crate::rpc::get_symbol(&pos.token1, cfg.rpc_url)
        .await
        .unwrap_or_else(|_| pos.token1.clone());

    let decimals0 = crate::rpc::get_decimals(&pos.token0, cfg.rpc_url)
        .await
        .unwrap_or(18);
    let decimals1 = crate::rpc::get_decimals(&pos.token1, cfg.rpc_url)
        .await
        .unwrap_or(18);

    let owed0_human = pos.tokens_owed0 as f64 / 10f64.powi(decimals0 as i32);
    let owed1_human = pos.tokens_owed1 as f64 / 10f64.powi(decimals1 as i32);

    let liquidity_to_remove =
        (pos.liquidity as f64 * args.liquidity_pct / 100.0) as u128;
    let is_full_removal = (args.liquidity_pct - 100.0).abs() < f64::EPSILON;

    // Resolve wallet address
    let wallet_addr = crate::onchainos::resolve_wallet(args.chain)?;
    if wallet_addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address for chain {}", args.chain);
    }

    // Verify ownership
    let owner = crate::rpc::get_owner_of(cfg.nfpm, args.token_id, cfg.rpc_url).await?;
    if owner.to_lowercase() != wallet_addr.to_lowercase() {
        anyhow::bail!(
            "You do not own position #{} (owner: {}, wallet: {})",
            args.token_id,
            owner,
            wallet_addr
        );
    }

    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() + 300)
        .unwrap_or(9_999_999_999);

    println!("Remove Liquidity (Uniswap V3, chain {}):", args.chain);
    println!("  Position:     #{}", args.token_id);
    println!("  Pair:         {}/{}", sym0, sym1);
    println!(
        "  Fee tier:     {}%",
        pos.fee as f64 / 10000.0
    );
    println!("  Tick range:   {} to {}", pos.tick_lower, pos.tick_upper);
    println!("  Total liq:    {}", pos.liquidity);
    println!("  Remove:       {}% = {}", args.liquidity_pct, liquidity_to_remove);
    println!(
        "  Owed fees:    {:.6} {} / {:.6} {}",
        owed0_human, sym0, owed1_human, sym1
    );
    println!("  Recipient:    {}", wallet_addr);
    println!("  NFPM:         {}", cfg.nfpm);
    println!();
    println!("NOTE: This will submit {} transactions. Confirm before proceeding.",
        if is_full_removal { "three (decreaseLiquidity + collect + burn)" } else { "two (decreaseLiquidity + collect)" }
    );

    // Step 1: decreaseLiquidity
    println!("\nStep 1: Calling decreaseLiquidity (liquidity={})...", liquidity_to_remove);
    let decrease_data = crate::calldata::encode_decrease_liquidity(
        args.token_id,
        liquidity_to_remove,
        0, // amount0Min = 0 (accept any — user wants full removal)
        0, // amount1Min = 0
        deadline,
    )?;

    let r1 = crate::onchainos::wallet_contract_call(
        args.chain, cfg.nfpm, &decrease_data, None, None, true, false,
    )
    .await?;
    let hash1 = crate::onchainos::extract_tx_hash(&r1);
    println!("  decreaseLiquidity tx: {}", hash1);
    crate::onchainos::wait_for_tx(&hash1, cfg.rpc_url).await?;
    println!("  Step 1 confirmed.");

    // Wait between decreaseLiquidity and collect
    tokio::time::sleep(std::time::Duration::from_secs(5)).await;

    // Step 2: collect — transfer all owed tokens to recipient
    println!("\nStep 2: Calling collect (recipient={})...", wallet_addr);
    let collect_data = crate::calldata::encode_collect(args.token_id, &wallet_addr)?;

    let r2 = crate::onchainos::wallet_contract_call(
        args.chain, cfg.nfpm, &collect_data, None, None, true, false,
    )
    .await?;
    let hash2 = crate::onchainos::extract_tx_hash(&r2);
    println!("  collect tx: {}", hash2);
    crate::onchainos::wait_for_tx(&hash2, cfg.rpc_url).await?;
    println!("  Step 2 confirmed.");

    // Step 3: burn (only if fully removing all liquidity)
    if is_full_removal {
        println!("\nStep 3: Burning position NFT #{} (all liquidity removed)...", args.token_id);
        let burn_data = crate::calldata::encode_burn(args.token_id)?;

        let r3 = crate::onchainos::wallet_contract_call(
            args.chain, cfg.nfpm, &burn_data, None, None, true, false,
        )
        .await?;
        let hash3 = crate::onchainos::extract_tx_hash(&r3);
        println!("  burn tx: {}", hash3);
        let explorer = crate::config::explorer_url(args.chain, &hash3);
        println!("\nPosition #{} fully closed!", args.token_id);
        println!("  Tokens returned to your wallet.");
        println!("  View: {}", explorer);
    } else {
        let explorer = crate::config::explorer_url(args.chain, &hash2);
        println!("\nPartial removal complete for position #{}!", args.token_id);
        println!("  {}% liquidity removed. Position still active.", args.liquidity_pct);
        println!("  View: {}", explorer);
    }

    Ok(())
}
