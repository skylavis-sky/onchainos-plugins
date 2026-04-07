// stake: Stake SOL into a specific validator LST pool via SPL Stake Pool DepositSol.
//
// Flow:
//   1. Validate LST is stakeable (not mSOL, not INF, not wSOL)
//   2. Resolve wallet via onchainos
//   3. Fetch stake pool state via getAccountInfo
//   4. Derive withdraw authority PDA and user token account (getTokenAccountsByOwner → ATA)
//   5. Build DepositSol v0 versioned transaction
//   6. Submit via onchainos wallet contract-call --unsigned-tx <base58> --force
//
// Important: Do NOT create ATA in the same tx — pre-check existence instead.
// After staking, LST tokens appear at the next epoch boundary (~2-3 days).

use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::config::{self, PoolProgram, LAMPORTS_PER_SOL, STAKE_POOL_PROGRAM};
use crate::instructions::{build_deposit_sol_transaction, derive_ata};
use crate::onchainos;
use crate::rpc;

#[derive(Args)]
pub struct StakeArgs {
    /// LST symbol to stake into (e.g. jitoSOL, bSOL). NOT mSOL (use marinade plugin).
    #[arg(long)]
    pub lst: String,

    /// Amount of SOL to stake (UI units, e.g. 0.002)
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (must be 501)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,

    /// Preview without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: StakeArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("sanctum-validator-lst only supports Solana (chain 501)");
    }
    if args.amount <= 0.0 {
        anyhow::bail!("Amount must be positive");
    }
    let lamports = (args.amount * LAMPORTS_PER_SOL as f64) as u64;
    if lamports < 100_000 {
        anyhow::bail!("Minimum stake amount is 0.0001 SOL");
    }

    // Validate LST
    let lst_cfg = config::find_lst(&args.lst)
        .ok_or_else(|| anyhow::anyhow!("Unknown LST '{}'. Run 'list-lsts' to see supported tokens.", args.lst))?;

    match lst_cfg.pool_program {
        PoolProgram::Marinade => {
            anyhow::bail!(
                "mSOL uses Marinade's custom program — use the 'marinade' plugin to stake SOL for mSOL."
            );
        }
        PoolProgram::Infinity => {
            anyhow::bail!(
                "INF is the Sanctum Infinity pool token — use the 'sanctum-infinity' plugin to deposit into the Infinity pool."
            );
        }
        PoolProgram::WrappedSol => {
            anyhow::bail!("wSOL is Wrapped SOL and is not a stakeable LST.");
        }
        PoolProgram::SplStakePool | PoolProgram::SanctumSpl => {
            // Supported — proceed
        }
    }

    // Resolve wallet
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet. Make sure onchainos is logged in.");
    }

    // For jitoSOL we have a known stake pool address; others need runtime resolution.
    // If stake_pool is empty we cannot build the transaction without an RPC lookup.
    // Strategy: if stake_pool is empty, we look it up by scanning the LST mint's
    // mintAuthority (which is the withdraw authority PDA). For now, require non-empty
    // stake_pool or fail with a helpful message.
    let stake_pool_addr = if !lst_cfg.stake_pool.is_empty() {
        lst_cfg.stake_pool.to_string()
    } else {
        // For SanctumSpl LSTs without a hardcoded pool address, we cannot derive the
        // stake pool address without additional API calls. Return a clear error for now.
        anyhow::bail!(
            "Stake pool address for {} is not hardcoded in this version. \
             Only jitoSOL is fully supported for the 'stake' command. \
             For other LSTs, use the Sanctum web UI or the 'swap-lst' command \
             to swap wSOL (native SOL) → {} via the Router instead.",
            lst_cfg.symbol, lst_cfg.symbol
        );
    };

    // Fetch stake pool state
    let pool_info = rpc::fetch_stake_pool(&stake_pool_addr).await?;

    let pool_mint_b58 = bs58::encode(&pool_info.pool_mint).into_string();
    // Sanity check: pool mint should match our registry
    if pool_mint_b58 != lst_cfg.mint {
        anyhow::bail!(
            "Pool mint mismatch for {}: expected {} got {}",
            lst_cfg.symbol, lst_cfg.mint, pool_mint_b58
        );
    }

    // Calculate expected LST amount
    let sol_per_lst = if pool_info.pool_token_supply > 0 {
        pool_info.total_lamports as f64 / pool_info.pool_token_supply as f64
    } else {
        1.0
    };
    let expected_lst = args.amount / sol_per_lst * 1e9; // in atomics

    // Resolve user token account: try existing accounts first, fall back to ATA
    let (user_token_account_b58, user_token_account_bytes) =
        resolve_user_token_account(&wallet, &pool_mint_b58).await?;

    // Get latest blockhash
    let blockhash = rpc::get_latest_blockhash().await?;
    let blockhash_bytes = bs58::decode(&blockhash)
        .into_vec()
        .map_err(|e| anyhow::anyhow!("Invalid blockhash: {}", e))?;

    let preview = serde_json::json!({
        "operation": "stake",
        "lst": lst_cfg.symbol,
        "wallet": wallet,
        "sol_amount": args.amount,
        "lamports": lamports.to_string(),
        "expected_lst_atomics": format!("{:.0}", expected_lst),
        "sol_per_lst_rate": format!("{:.8}", sol_per_lst),
        "user_token_account": user_token_account_b58,
        "stake_pool": stake_pool_addr,
        "pool_mint": pool_mint_b58,
        "note": "Ask user to confirm before broadcasting. LST tokens are credited at the next epoch boundary (~2-3 days)."
    });

    if args.dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": preview
        }));
    }

    // Build the DepositSol transaction
    let tx_b64 = build_deposit_sol_transaction(
        &wallet,
        &stake_pool_addr,
        &pool_info,
        &user_token_account_bytes,
        &blockhash_bytes,
        lamports,
    )?;

    // Submit
    let result = onchainos::wallet_contract_call_solana(STAKE_POOL_PROGRAM, &tx_b64, false).await?;
    let tx_hash = onchainos::extract_tx_hash(&result)?;

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "operation": "stake",
            "lst": lst_cfg.symbol,
            "sol_staked": args.amount,
            "expected_lst_atomics": format!("{:.0}", expected_lst),
            "wallet": wallet,
            "solscan": format!("https://solscan.io/tx/{}", tx_hash),
            "epoch_delay_note": "LST tokens are credited at the next epoch boundary (~2-3 days on Solana mainnet).",
            "preview": preview
        }
    }))
}

/// Resolve user's LST token account.
/// Tries getTokenAccountsByOwner first (handles non-ATA accounts),
/// then falls back to the canonical ATA address.
async fn resolve_user_token_account(wallet: &str, mint: &str) -> Result<(String, Vec<u8>)> {
    if let Ok((_ui, _raw, addr)) = rpc::get_token_accounts_by_owner(wallet, mint).await {
        if !addr.is_empty() {
            let bytes = bs58::decode(&addr)
                .into_vec()
                .map_err(|e| anyhow::anyhow!("Invalid token account address: {}", e))?;
            return Ok((addr, bytes));
        }
    }

    // Fall back to canonical ATA derivation
    let ata_bytes = derive_ata(wallet, mint)?;
    let ata_addr = bs58::encode(&ata_bytes).into_string();
    Ok((ata_addr, ata_bytes))
}
