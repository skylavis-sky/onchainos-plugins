// deposit — deposit USDC or USDT into a Maple Finance syrup pool
// Flow: ERC-20 approve(router, amount) → 3s delay → SyrupRouter.deposit(amount, bytes32(0))
//
// Contracts:
//   syrupUSDC SyrupRouter: 0x134cCaaA4F1e4552eC8aEcb9E4A2360dDcF8df76
//   syrupUSDT SyrupRouter: 0xF007476Bb27430795138C511F18F821e8D1e5Ee2
//
// Selectors (verified with `cast sig`):
//   approve(address,uint256)   → 0x095ea7b3
//   deposit(uint256,bytes32)   → 0xc9630cb0  [SyrupRouter]
//
// IMPORTANT: Ask user to confirm before executing on-chain transactions.

use crate::{config, onchainos, rpc};
use anyhow::Result;

pub async fn run(
    pool_name: &str,
    amount: f64,
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

    // Convert human-readable amount to raw token units
    let raw_amount: u128 = (amount * 10f64.powi(pool_cfg.decimals as i32)) as u128;
    if raw_amount == 0 {
        anyhow::bail!("Amount too small. Minimum is 0.000001 {}.", pool_cfg.token_symbol);
    }

    // Resolve wallet address (move after dry_run check since wallet not needed for dry_run)
    let wallet_addr = if dry_run {
        // Use zero address as placeholder for dry-run
        "0x0000000000000000000000000000000000000000".to_string()
    } else {
        if let Some(w) = from {
            w
        } else {
            onchainos::resolve_wallet()?
        }
    };

    // Step 1: Check current allowance (skip for dry-run)
    if !dry_run {
        let current_allowance = rpc::allowance(rpc_url, pool_cfg.token, &wallet_addr, pool_cfg.router)
            .await
            .unwrap_or(0);

        if current_allowance < raw_amount {
            // For USDT: must set allowance to 0 first if non-zero (USDT allowance race)
            if pool_cfg.token_symbol == "USDT" && current_allowance > 0 {
                eprintln!("Resetting USDT allowance to 0 first...");
                let reset_result = onchainos::erc20_approve(
                    config::CHAIN_ID,
                    pool_cfg.token,
                    pool_cfg.router,
                    0,
                    Some(&wallet_addr),
                    false,
                )
                .await?;
                if !onchainos::is_ok(&reset_result) {
                    eprintln!("Warning: USDT allowance reset may have failed: {:?}", reset_result);
                }
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            }

            // Step 1: ERC-20 approve
            // Ask user to confirm before executing on-chain transactions
            eprintln!(
                "Step 1/2: Approving {} {} for SyrupRouter {}",
                amount, pool_cfg.token_symbol, pool_cfg.router
            );
            let approve_result = onchainos::erc20_approve(
                config::CHAIN_ID,
                pool_cfg.token,
                pool_cfg.router,
                raw_amount,
                Some(&wallet_addr),
                false,
            )
            .await?;

            if !onchainos::is_ok(&approve_result) {
                anyhow::bail!("ERC-20 approve failed: {}", serde_json::to_string(&approve_result)?);
            }
            eprintln!("Approve tx: {}", onchainos::extract_tx_hash(&approve_result));

            // Wait 3 seconds between approve and deposit
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    // Step 2: SyrupRouter.deposit(amount, bytes32(0))
    // selector: 0xc9630cb0 (cast sig "deposit(uint256,bytes32)")
    // Ask user to confirm before executing on-chain transactions
    let amount_hex = format!("{:064x}", raw_amount);
    let deposit_data_zero = "0".repeat(64); // bytes32(0)
    let calldata = format!("0xc9630cb0{}{}", amount_hex, deposit_data_zero);

    eprintln!(
        "Step 2/2: Depositing {} {} into {} pool...",
        amount, pool_cfg.token_symbol, pool_cfg.name
    );

    let result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        pool_cfg.router,
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
                "router": pool_cfg.router,
                "token": pool_cfg.token_symbol,
                "amount": amount,
                "amount_raw": raw_amount.to_string(),
                "calldata": calldata,
                "txHash": tx_hash
            }
        }))?
    );
    Ok(())
}
