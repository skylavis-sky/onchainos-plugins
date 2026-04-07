use serde_json::{json, Value};
use crate::calldata;
use crate::config::{get_pool_by_name, CHAIN_ID, WAD};
use crate::onchainos;
use crate::rpc;

/// Supply wstETH/WETH to earn interest (lender side).
///
/// Flow:
///   1. lendToken.approve(ionPool, amount)
///   2. IonPool.supply(wallet, amount, [])
///
/// --pool: pool name or collateral symbol (e.g. "rsETH/wstETH" or "rsETH")
/// --amount: amount in WAD (wei-level, 18 decimals)
pub async fn run(
    chain_id: u64,
    pool_name: &str,
    amount_wad: u128,
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

    let amount_human = amount_wad as f64 / WAD as f64;

    // Build calldatas
    let approve_calldata = calldata::encode_erc20_approve(pool.ion_pool, amount_wad)?;
    let supply_calldata = calldata::encode_supply(&wallet, amount_wad)?;

    if dry_run {
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.lend_token, approve_calldata, wallet
        );
        let supply_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.ion_pool, supply_calldata, wallet
        );
        eprintln!("[dry-run] step 1 approve lend token: {}", approve_cmd);
        eprintln!("[dry-run] step 2 supply to IonPool: {}", supply_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "action": "lend",
            "pool": pool.name,
            "ionPool": pool.ion_pool,
            "lendToken": pool.lend_token,
            "lendSymbol": pool.lend_symbol,
            "wallet": wallet,
            "amountWad": amount_wad.to_string(),
            "amountHuman": format!("{:.6} {}", amount_human, pool.lend_symbol),
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "contract": pool.lend_token,
                    "spender": pool.ion_pool,
                    "calldata": approve_calldata,
                    "simulatedCommand": approve_cmd
                },
                {
                    "step": 2,
                    "action": "supply",
                    "contract": pool.ion_pool,
                    "calldata": supply_calldata,
                    "simulatedCommand": supply_cmd
                }
            ]
        }));
    }

    // Step 1: Approve lend token to IonPool
    eprintln!("[ion-protocol] Step 1/2: Approving {} {} to IonPool {}...",
        amount_human, pool.lend_symbol, pool.ion_pool);
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        pool.lend_token,
        &approve_calldata,
        Some(&wallet),
        false,
    )?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    eprintln!("[ion-protocol] Approve tx: {}", approve_tx);

    // Wait for approve tx to be mined
    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        eprintln!("[ion-protocol] Waiting for approve to confirm...");
        rpc::wait_for_tx(crate::config::RPC_URL, &approve_tx).await
            .map_err(|e| anyhow::anyhow!("Approve tx did not confirm: {}", e))?;
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: Supply to IonPool
    eprintln!("[ion-protocol] Step 2/2: Supplying {} {} to IonPool...",
        amount_human, pool.lend_symbol);
    let supply_result = onchainos::wallet_contract_call(
        chain_id,
        pool.ion_pool,
        &supply_calldata,
        Some(&wallet),
        false,
    )?;
    let supply_tx = onchainos::extract_tx_hash_or_err(&supply_result)?;
    eprintln!("[ion-protocol] Supply tx: {}", supply_tx);

    Ok(json!({
        "ok": true,
        "action": "lend",
        "pool": pool.name,
        "ionPool": pool.ion_pool,
        "lendToken": pool.lend_token,
        "lendSymbol": pool.lend_symbol,
        "wallet": wallet,
        "amountWad": amount_wad.to_string(),
        "amountHuman": format!("{:.6} {}", amount_human, pool.lend_symbol),
        "approveTxHash": approve_tx,
        "supplyTxHash": supply_tx,
        "note": "You received ion-tokens representing your lend position. Yield accrues automatically via supplyFactor."
    }))
}
