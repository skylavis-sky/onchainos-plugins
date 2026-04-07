use serde_json::{json, Value};
use crate::calldata;
use crate::config::{get_pool_by_name, CHAIN_ID, WAD};
use crate::onchainos;
use crate::rpc;

/// Deposit LRT collateral (steps 1-3 of borrow flow, without borrowing).
///
/// Flow:
///   1. collateral.approve(gemJoin, amount)
///   2. GemJoin.join(wallet, amount)
///   3. IonPool.depositCollateral(ilkIndex, wallet, wallet, amount, [])
///
/// --pool: pool name or collateral symbol (e.g. "rsETH")
/// --amount: collateral amount in WAD (18 decimals)
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

    let approve_calldata = calldata::encode_erc20_approve(pool.gem_join, amount_wad)?;
    let join_calldata = calldata::encode_gem_join(&wallet, amount_wad)?;
    let deposit_calldata = calldata::encode_deposit_collateral(pool.ilk_index, &wallet, amount_wad)?;

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
        eprintln!("[dry-run] step 1 approve collateral to GemJoin: {}", approve_cmd);
        eprintln!("[dry-run] step 2 GemJoin.join: {}", join_cmd);
        eprintln!("[dry-run] step 3 IonPool.depositCollateral: {}", deposit_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "action": "deposit-collateral",
            "pool": pool.name,
            "ionPool": pool.ion_pool,
            "gemJoin": pool.gem_join,
            "collateral": pool.collateral,
            "collateralSymbol": pool.collateral_symbol,
            "wallet": wallet,
            "amountWad": amount_wad.to_string(),
            "amountHuman": format!("{:.6} {}", amount_human, pool.collateral_symbol),
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "description": format!("Approve {} to GemJoin", pool.collateral_symbol),
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
                }
            ]
        }));
    }

    // Step 1: Approve collateral to GemJoin
    eprintln!("[ion-protocol] Step 1/3: Approving {} {} to GemJoin {}...",
        amount_human, pool.collateral_symbol, pool.gem_join);
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
    eprintln!("[ion-protocol] Step 2/3: GemJoin.join...");
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
    eprintln!("[ion-protocol] Step 3/3: IonPool.depositCollateral...");
    let deposit_result = onchainos::wallet_contract_call(
        chain_id, pool.ion_pool, &deposit_calldata, Some(&wallet), false,
    )?;
    let deposit_tx = onchainos::extract_tx_hash_or_err(&deposit_result)?;
    eprintln!("[ion-protocol] DepositCollateral tx: {}", deposit_tx);

    Ok(json!({
        "ok": true,
        "action": "deposit-collateral",
        "pool": pool.name,
        "ionPool": pool.ion_pool,
        "collateral": pool.collateral,
        "collateralSymbol": pool.collateral_symbol,
        "wallet": wallet,
        "amountWad": amount_wad.to_string(),
        "amountHuman": format!("{:.6} {}", amount_human, pool.collateral_symbol),
        "steps": [
            {"step": 1, "action": "approve", "txHash": approve_tx},
            {"step": 2, "action": "GemJoin.join", "txHash": join_tx},
            {"step": 3, "action": "IonPool.depositCollateral", "txHash": deposit_tx}
        ],
        "note": "Collateral is now registered in your IonPool vault. Call borrow to take a loan against it."
    }))
}
