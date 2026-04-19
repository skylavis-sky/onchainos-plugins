/// `uniswap-v3 get-positions` — view Uniswap V3 LP positions for a wallet.
/// Read-only — no transactions required.
///
/// Supports two modes:
/// - `--token-id <ID>`: fetch a single position by NFT token ID
/// - `--owner <address>`: enumerate all positions for a wallet via balanceOf + tokenOfOwnerByIndex

use anyhow::Result;

pub struct GetPositionsArgs {
    pub owner: Option<String>,
    pub token_id: Option<u128>,
    pub chain: u64,
}

pub async fn run(args: GetPositionsArgs) -> Result<()> {
    let cfg = crate::config::get_chain_config(args.chain)?;

    if let Some(token_id) = args.token_id {
        // Single position lookup
        return show_position(cfg, token_id).await;
    }

    // Owner enumeration mode
    let owner = match args.owner {
        Some(ref o) => crate::config::resolve_token_address(o, args.chain)
            .unwrap_or_else(|_| o.clone()),
        None => {
            // Resolve from onchainos wallet
            crate::onchainos::resolve_wallet(args.chain).map_err(|e| {
                anyhow::anyhow!(
                    "No --owner specified and wallet resolution failed: {}. Use --owner <address>.",
                    e
                )
            })?
        }
    };

    if owner.is_empty() {
        anyhow::bail!("Could not determine wallet address. Use --owner <address>.");
    }

    println!(
        "Uniswap V3 positions for {} on chain {}:",
        owner, args.chain
    );
    println!("NonfungiblePositionManager: {}", cfg.nfpm);
    println!();

    let token_ids = crate::rpc::get_token_ids_for_owner(cfg.nfpm, &owner, cfg.rpc_url).await?;

    if token_ids.is_empty() {
        println!("No Uniswap V3 LP positions found for {} on chain {}.", owner, args.chain);
        return Ok(());
    }

    println!("Found {} position(s):\n", token_ids.len());

    for tid in token_ids {
        show_position(cfg, tid).await.unwrap_or_else(|e| {
            eprintln!("  Error fetching position #{}: {}", tid, e);
        });
    }

    Ok(())
}

async fn show_position(cfg: &crate::config::ChainConfig, token_id: u128) -> Result<()> {
    let pos = crate::rpc::get_position(cfg.nfpm, token_id, cfg.rpc_url).await?;

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
    let fee_pct = pos.fee as f64 / 10000.0;

    println!("  Position #{}", token_id);
    println!("    Pair:         {}/{}", sym0, sym1);
    println!("    Fee tier:     {}%", fee_pct);
    println!("    Tick range:   {} to {}", pos.tick_lower, pos.tick_upper);
    println!("    Liquidity:    {}", pos.liquidity);
    println!(
        "    Owed fees:    {:.6} {} / {:.6} {}",
        owed0_human, sym0, owed1_human, sym1
    );
    println!("    token0:       {}", pos.token0);
    println!("    token1:       {}", pos.token1);
    if pos.liquidity == 0 {
        println!("    Status:       [CLOSED — zero liquidity]");
    }
    println!();

    Ok(())
}
