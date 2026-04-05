// withdraw (requestRedeem) — initiate withdrawal from Maple Finance syrup pool
// This calls Pool.requestRedeem(shares, owner) which enqueues the redemption
// Actual fund release happens after the withdrawal queue processes.
//
// Contracts:
//   syrupUSDC Pool: 0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b
//   syrupUSDT Pool: 0x356B8d89c1e1239Cbbb9dE4815c39A1474d5BA7D
//
// Selector (verified with `cast sig`):
//   requestRedeem(uint256,address) → 0x107703ab
//
// IMPORTANT: Ask user to confirm before executing on-chain transactions.

use crate::{config, onchainos, rpc};
use anyhow::Result;

pub async fn run(
    pool_name: &str,
    shares: Option<f64>,  // None = withdraw all shares
    rpc_url: &str,
    from: Option<String>,
    dry_run: bool,
) -> Result<()> {
    let pool_cfg = config::resolve_pool(pool_name).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown pool '{}'. Valid options: syrupUSDC, syrupUSDT, usdc, usdt",
            pool_name
        )
    })?;

    // Resolve wallet address
    let wallet_addr = if dry_run {
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        if let Some(w) = from {
            w
        } else {
            onchainos::resolve_wallet()?
        }
    };

    // Determine shares to redeem
    let raw_shares: u128 = if let Some(s) = shares {
        (s * 10f64.powi(pool_cfg.decimals as i32)) as u128
    } else {
        // Withdraw all: fetch current balance
        if dry_run {
            // Use placeholder for dry-run
            1_000_000u128
        } else {
            let bal = rpc::balance_of(rpc_url, pool_cfg.pool, &wallet_addr).await?;
            if bal == 0 {
                anyhow::bail!(
                    "No {} shares found for wallet {}",
                    pool_cfg.name,
                    wallet_addr
                );
            }
            bal
        }
    };

    if raw_shares == 0 {
        anyhow::bail!("Shares amount is 0. Nothing to withdraw.");
    }

    // Build requestRedeem(uint256 shares, address owner) calldata
    // selector: 0x107703ab (cast sig "requestRedeem(uint256,address)")
    // Ask user to confirm before executing on-chain transactions
    let shares_hex = format!("{:064x}", raw_shares);
    let owner_padded = format!("{:0>64}", wallet_addr.trim_start_matches("0x").to_lowercase());
    let calldata = format!("0x107703ab{}{}", shares_hex, owner_padded);

    eprintln!(
        "Requesting redeem of {} shares from {} pool...",
        rpc::format_amount(raw_shares, pool_cfg.decimals),
        pool_cfg.name
    );

    let result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        pool_cfg.pool,
        &calldata,
        if dry_run {
            None
        } else {
            Some(wallet_addr.as_str())
        },
        dry_run,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "dry_run": dry_run,
            "data": {
                "pool": pool_cfg.name,
                "pool_address": pool_cfg.pool,
                "shares_raw": raw_shares.to_string(),
                "shares_formatted": format!("{:.6}", rpc::format_amount(raw_shares, pool_cfg.decimals)),
                "owner": wallet_addr,
                "calldata": calldata,
                "txHash": tx_hash,
                "note": "requestRedeem enqueues your shares for withdrawal. Funds will be available after the withdrawal queue processes (timing depends on pool liquidity)."
            }
        }))?
    );
    Ok(())
}
