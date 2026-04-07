use serde_json::{json, Value};
use crate::calldata;
use crate::config::{get_pool_by_name, CHAIN_ID, RAY, WAD};
use crate::onchainos;
use crate::rpc;

/// Full 4-step borrow flow: approve → GemJoin.join → depositCollateral → borrow.
///
/// --pool: pool name or collateral symbol (e.g. "rsETH")
/// --collateral-amount: collateral to deposit in WAD (18 decimals)
/// --borrow-amount: loan token amount to borrow in WAD (18 decimals)
///
/// CRITICAL: borrow() takes normalizedDebt = actualAmount * RAY / rate(ilkIndex)
pub async fn run(
    chain_id: u64,
    pool_name: &str,
    collateral_amount_wad: u128,
    borrow_amount_wad: u128,
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

    // Get current accumulated rate to compute normalizedDebt
    let rate = rpc::get_rate(pool.ion_pool, pool.ilk_index).await
        .unwrap_or(RAY); // fallback to RAY (1:1) if call fails

    // normalizedDebt = borrow_amount * RAY / rate
    let normalized_debt = rpc::to_normalized(borrow_amount_wad, rate);

    let collateral_human = collateral_amount_wad as f64 / WAD as f64;
    let borrow_human = borrow_amount_wad as f64 / WAD as f64;
    let rate_human = rate as f64 / RAY as f64;

    // Build calldatas
    let approve_calldata = calldata::encode_erc20_approve(pool.gem_join, collateral_amount_wad)?;
    let join_calldata = calldata::encode_gem_join(&wallet, collateral_amount_wad)?;
    let deposit_calldata = calldata::encode_deposit_collateral(pool.ilk_index, &wallet, collateral_amount_wad)?;
    let borrow_calldata = calldata::encode_borrow(pool.ilk_index, &wallet, normalized_debt)?;

    if dry_run {
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.collateral, approve_calldata, wallet
        );
        let join_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.gem_join, join_calldata, wallet
        );
        let deposit_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.ion_pool, deposit_calldata, wallet
        );
        let borrow_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.ion_pool, borrow_calldata, wallet
        );
        eprintln!("[dry-run] step 1 approve {} to GemJoin: {}", pool.collateral_symbol, approve_cmd);
        eprintln!("[dry-run] step 2 GemJoin.join: {}", join_cmd);
        eprintln!("[dry-run] step 3 IonPool.depositCollateral: {}", deposit_cmd);
        eprintln!("[dry-run] step 4 IonPool.borrow (normalizedDebt={}): {}", normalized_debt, borrow_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "action": "borrow",
            "pool": pool.name,
            "ionPool": pool.ion_pool,
            "gemJoin": pool.gem_join,
            "collateral": pool.collateral,
            "collateralSymbol": pool.collateral_symbol,
            "lendToken": pool.lend_token,
            "lendSymbol": pool.lend_symbol,
            "wallet": wallet,
            "collateralAmountWad": collateral_amount_wad.to_string(),
            "collateralAmountHuman": format!("{:.6} {}", collateral_human, pool.collateral_symbol),
            "borrowAmountWad": borrow_amount_wad.to_string(),
            "borrowAmountHuman": format!("{:.6} {}", borrow_human, pool.lend_symbol),
            "normalizedDebt": normalized_debt.to_string(),
            "rateRay": rate.to_string(),
            "rateHuman": format!("{:.6}", rate_human),
            "normalizedDebtCalc": format!("{} * RAY / {} = {}", borrow_amount_wad, rate, normalized_debt),
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "description": format!("Approve {} {} to GemJoin", collateral_human, pool.collateral_symbol),
                    "contract": pool.collateral,
                    "calldata": approve_calldata,
                    "simulatedCommand": approve_cmd
                },
                {
                    "step": 2,
                    "action": "GemJoin.join",
                    "description": "Transfer collateral to GemJoin",
                    "contract": pool.gem_join,
                    "calldata": join_calldata,
                    "simulatedCommand": join_cmd
                },
                {
                    "step": 3,
                    "action": "IonPool.depositCollateral",
                    "description": "Register collateral in IonPool vault",
                    "contract": pool.ion_pool,
                    "calldata": deposit_calldata,
                    "simulatedCommand": deposit_cmd
                },
                {
                    "step": 4,
                    "action": "IonPool.borrow",
                    "description": format!("Borrow {} {} (normalized: {})", borrow_human, pool.lend_symbol, normalized_debt),
                    "contract": pool.ion_pool,
                    "calldata": borrow_calldata,
                    "simulatedCommand": borrow_cmd
                }
            ]
        }));
    }

    // Step 1: Approve collateral to GemJoin
    eprintln!("[ion-protocol] Step 1/4: Approving {} {} to GemJoin {}...",
        collateral_human, pool.collateral_symbol, pool.gem_join);
    let approve_result = onchainos::wallet_contract_call(
        chain_id, pool.collateral, &approve_calldata, Some(&wallet), false,
    )?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    eprintln!("[ion-protocol] Approve tx: {}", approve_tx);

    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        eprintln!("[ion-protocol] Waiting for approve to confirm...");
        rpc::wait_for_tx(crate::config::RPC_URL, &approve_tx).await
            .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: GemJoin.join
    eprintln!("[ion-protocol] Step 2/4: GemJoin.join...");
    let join_result = onchainos::wallet_contract_call(
        chain_id, pool.gem_join, &join_calldata, Some(&wallet), false,
    )?;
    let join_tx = onchainos::extract_tx_hash_or_err(&join_result)?;
    eprintln!("[ion-protocol] Join tx: {}", join_tx);

    if join_tx.starts_with("0x") && join_tx.len() == 66 {
        eprintln!("[ion-protocol] Waiting for join to confirm...");
        rpc::wait_for_tx(crate::config::RPC_URL, &join_tx).await
            .map_err(|e| anyhow::anyhow!("GemJoin.join tx did not confirm: {}", e))?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 3: IonPool.depositCollateral
    eprintln!("[ion-protocol] Step 3/4: IonPool.depositCollateral...");
    let deposit_result = onchainos::wallet_contract_call(
        chain_id, pool.ion_pool, &deposit_calldata, Some(&wallet), false,
    )?;
    let deposit_tx = onchainos::extract_tx_hash_or_err(&deposit_result)?;
    eprintln!("[ion-protocol] DepositCollateral tx: {}", deposit_tx);

    if deposit_tx.starts_with("0x") && deposit_tx.len() == 66 {
        eprintln!("[ion-protocol] Waiting for depositCollateral to confirm...");
        rpc::wait_for_tx(crate::config::RPC_URL, &deposit_tx).await
            .map_err(|e| anyhow::anyhow!("depositCollateral tx did not confirm: {}", e))?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 4: IonPool.borrow
    eprintln!("[ion-protocol] Step 4/4: IonPool.borrow (normalizedDebt={})...", normalized_debt);
    let borrow_result = onchainos::wallet_contract_call(
        chain_id, pool.ion_pool, &borrow_calldata, Some(&wallet), false,
    )?;
    let borrow_tx = onchainos::extract_tx_hash_or_err(&borrow_result)?;
    eprintln!("[ion-protocol] Borrow tx: {}", borrow_tx);

    Ok(json!({
        "ok": true,
        "action": "borrow",
        "pool": pool.name,
        "ionPool": pool.ion_pool,
        "collateralSymbol": pool.collateral_symbol,
        "lendSymbol": pool.lend_symbol,
        "wallet": wallet,
        "collateralAmountWad": collateral_amount_wad.to_string(),
        "collateralAmountHuman": format!("{:.6} {}", collateral_human, pool.collateral_symbol),
        "borrowAmountWad": borrow_amount_wad.to_string(),
        "borrowAmountHuman": format!("{:.6} {}", borrow_human, pool.lend_symbol),
        "normalizedDebt": normalized_debt.to_string(),
        "rateRay": rate.to_string(),
        "steps": [
            {"step": 1, "action": "approve", "txHash": approve_tx},
            {"step": 2, "action": "GemJoin.join", "txHash": join_tx},
            {"step": 3, "action": "IonPool.depositCollateral", "txHash": deposit_tx},
            {"step": 4, "action": "IonPool.borrow", "txHash": borrow_tx}
        ]
    }))
}
