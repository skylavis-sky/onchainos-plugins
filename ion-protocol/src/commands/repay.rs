use serde_json::{json, Value};
use crate::calldata;
use crate::config::{get_pool_by_name, CHAIN_ID, RAY, WAD};
use crate::onchainos;
use crate::rpc;

/// Repay borrowed wstETH/WETH and optionally withdraw collateral.
///
/// Full repay+withdraw flow:
///   1. lendToken.approve(ionPool, repay_amount)
///   2. IonPool.repay(ilkIndex, wallet, wallet, normalizedDebt)
///   3. IonPool.withdrawCollateral(ilkIndex, wallet, wallet, collateral_amount)   [if --withdraw-collateral]
///   4. GemJoin.exit(wallet, collateral_amount)                                    [if --withdraw-collateral]
///
/// --pool: pool name or collateral symbol
/// --amount: amount of lend token to repay in WAD (or use --all to repay full debt)
/// --withdraw-collateral: also withdraw collateral after repay (optional)
/// --collateral-amount: collateral to withdraw in WAD (required if --withdraw-collateral)
pub async fn run(
    chain_id: u64,
    pool_name: &str,
    amount_wad: Option<u128>,
    repay_all: bool,
    withdraw_collateral: bool,
    collateral_amount_wad: Option<u128>,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if chain_id != CHAIN_ID {
        anyhow::bail!(
            "Ion Protocol only supports Ethereum Mainnet (chain 1). Got chain {}",
            chain_id
        );
    }

    let pool = get_pool_by_name(pool_name)?;

    let wallet = match from {
        Some(addr) => addr.to_string(),
        None => onchainos::resolve_wallet(chain_id)
            .map_err(|e| anyhow::anyhow!("Could not resolve wallet: {}", e))?,
    };

    // Determine normalizedDebt to repay
    let normalized_debt = if repay_all {
        // Read current normalizedDebt(ilk, user) and add 0.1% buffer to avoid dust
        let nd = rpc::get_normalized_debt(pool.ion_pool, pool.ilk_index, &wallet).await?;
        if nd == 0 {
            anyhow::bail!("No outstanding debt found for wallet {} in pool {}", wallet, pool.name);
        }
        // Add 0.1% buffer: nd + nd/1000
        nd + nd / 1000
    } else {
        let borrow_amount = amount_wad.ok_or_else(|| {
            anyhow::anyhow!("Must specify --amount or --all for repay")
        })?;
        // Get current rate to compute normalizedDebt
        let rate = rpc::get_rate(pool.ion_pool, pool.ilk_index).await
            .unwrap_or(RAY);
        // normalizedDebt = amount * RAY / rate; add 0.1% buffer
        let nd = rpc::to_normalized(borrow_amount, rate);
        nd + nd / 1000
    };

    // Determine repay amount for approve (use actual amount + buffer)
    let rate = rpc::get_rate(pool.ion_pool, pool.ilk_index).await
        .unwrap_or(RAY);
    // actual repay amount = normalizedDebt * rate / RAY
    let repay_amount_wad: u128 = {
        let actual_f64 = (normalized_debt as f64) * (rate as f64) / (RAY as f64);
        actual_f64 as u128
    };
    let repay_human = repay_amount_wad as f64 / WAD as f64;

    // Build calldatas
    let approve_calldata = calldata::encode_erc20_approve(pool.ion_pool, repay_amount_wad)?;
    let repay_calldata = calldata::encode_repay(pool.ilk_index, &wallet, normalized_debt)?;

    // Collateral withdrawal calldatas (optional)
    let col_amount = collateral_amount_wad.unwrap_or(0);
    let withdraw_col_calldata = if withdraw_collateral && col_amount > 0 {
        Some(calldata::encode_withdraw_collateral(pool.ilk_index, &wallet, col_amount)?)
    } else {
        None
    };
    let gem_exit_calldata = if withdraw_collateral && col_amount > 0 {
        Some(calldata::encode_gem_exit(&wallet, col_amount)?)
    } else {
        None
    };

    let col_human = col_amount as f64 / WAD as f64;

    if dry_run {
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.lend_token, approve_calldata, wallet
        );
        let repay_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.ion_pool, repay_calldata, wallet
        );
        eprintln!("[dry-run] step 1 approve lend token: {}", approve_cmd);
        eprintln!("[dry-run] step 2 IonPool.repay (normalizedDebt={}): {}", normalized_debt, repay_cmd);

        let mut steps = vec![
            json!({
                "step": 1,
                "action": "approve",
                "description": format!("Approve ~{:.6} {} to IonPool for repay", repay_human, pool.lend_symbol),
                "contract": pool.lend_token,
                "calldata": approve_calldata,
                "simulatedCommand": approve_cmd
            }),
            json!({
                "step": 2,
                "action": "IonPool.repay",
                "description": format!("Repay normalizedDebt={}", normalized_debt),
                "contract": pool.ion_pool,
                "calldata": repay_calldata,
                "simulatedCommand": repay_cmd
            }),
        ];

        if let (Some(wc), Some(ge)) = (&withdraw_col_calldata, &gem_exit_calldata) {
            let wc_cmd = format!(
                "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
                chain_id, pool.ion_pool, wc, wallet
            );
            let ge_cmd = format!(
                "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
                chain_id, pool.gem_join, ge, wallet
            );
            eprintln!("[dry-run] step 3 IonPool.withdrawCollateral: {}", wc_cmd);
            eprintln!("[dry-run] step 4 GemJoin.exit: {}", ge_cmd);
            steps.push(json!({
                "step": 3,
                "action": "IonPool.withdrawCollateral",
                "description": format!("Withdraw {:.6} {} from vault", col_human, pool.collateral_symbol),
                "contract": pool.ion_pool,
                "calldata": wc,
                "simulatedCommand": wc_cmd
            }));
            steps.push(json!({
                "step": 4,
                "action": "GemJoin.exit",
                "description": format!("Transfer {:.6} {} to wallet", col_human, pool.collateral_symbol),
                "contract": pool.gem_join,
                "calldata": ge,
                "simulatedCommand": ge_cmd
            }));
        }

        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "action": "repay",
            "pool": pool.name,
            "ionPool": pool.ion_pool,
            "lendToken": pool.lend_token,
            "lendSymbol": pool.lend_symbol,
            "wallet": wallet,
            "repayAmountWad": repay_amount_wad.to_string(),
            "repayAmountHuman": format!("~{:.6} {}", repay_human, pool.lend_symbol),
            "normalizedDebt": normalized_debt.to_string(),
            "rateRay": rate.to_string(),
            "withdrawCollateral": withdraw_collateral,
            "collateralAmountWad": col_amount.to_string(),
            "steps": steps
        }));
    }

    // Step 1: Approve lend token to IonPool
    eprintln!("[ion-protocol] Step 1: Approving ~{:.6} {} to IonPool for repay...",
        repay_human, pool.lend_symbol);
    let approve_result = onchainos::wallet_contract_call(
        chain_id, pool.lend_token, &approve_calldata, Some(&wallet), false,
    )?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    eprintln!("[ion-protocol] Approve tx: {}", approve_tx);

    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        eprintln!("[ion-protocol] Waiting for approve to confirm...");
        rpc::wait_for_tx(crate::config::RPC_URL, &approve_tx).await
            .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: IonPool.repay
    eprintln!("[ion-protocol] Step 2: IonPool.repay (normalizedDebt={})...", normalized_debt);
    let repay_result = onchainos::wallet_contract_call(
        chain_id, pool.ion_pool, &repay_calldata, Some(&wallet), false,
    )?;
    let repay_tx = onchainos::extract_tx_hash_or_err(&repay_result)?;
    eprintln!("[ion-protocol] Repay tx: {}", repay_tx);

    let mut result_steps = vec![
        json!({"step": 1, "action": "approve", "txHash": approve_tx}),
        json!({"step": 2, "action": "repay", "txHash": repay_tx}),
    ];

    // Optional: withdraw collateral
    if let (Some(wc), Some(ge)) = (&withdraw_col_calldata, &gem_exit_calldata) {
        if repay_tx.starts_with("0x") && repay_tx.len() == 66 {
            eprintln!("[ion-protocol] Waiting for repay to confirm...");
            rpc::wait_for_tx(crate::config::RPC_URL, &repay_tx).await
                .map_err(|e| anyhow::anyhow!("Repay tx did not confirm: {}", e))?;
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        eprintln!("[ion-protocol] Step 3: IonPool.withdrawCollateral...");
        let wc_result = onchainos::wallet_contract_call(
            chain_id, pool.ion_pool, wc, Some(&wallet), false,
        )?;
        let wc_tx = onchainos::extract_tx_hash_or_err(&wc_result)?;
        eprintln!("[ion-protocol] WithdrawCollateral tx: {}", wc_tx);

        if wc_tx.starts_with("0x") && wc_tx.len() == 66 {
            rpc::wait_for_tx(crate::config::RPC_URL, &wc_tx).await
                .map_err(|e| anyhow::anyhow!("WithdrawCollateral tx did not confirm: {}", e))?;
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }

        eprintln!("[ion-protocol] Step 4: GemJoin.exit...");
        let ge_result = onchainos::wallet_contract_call(
            chain_id, pool.gem_join, ge, Some(&wallet), false,
        )?;
        let ge_tx = onchainos::extract_tx_hash_or_err(&ge_result)?;
        eprintln!("[ion-protocol] Exit tx: {}", ge_tx);

        result_steps.push(json!({"step": 3, "action": "withdrawCollateral", "txHash": wc_tx}));
        result_steps.push(json!({"step": 4, "action": "GemJoin.exit", "txHash": ge_tx}));
    }

    Ok(json!({
        "ok": true,
        "action": "repay",
        "pool": pool.name,
        "ionPool": pool.ion_pool,
        "lendSymbol": pool.lend_symbol,
        "wallet": wallet,
        "repayAmountWad": repay_amount_wad.to_string(),
        "repayAmountHuman": format!("~{:.6} {}", repay_human, pool.lend_symbol),
        "normalizedDebt": normalized_debt.to_string(),
        "withdrawCollateral": withdraw_collateral,
        "steps": result_steps
    }))
}
