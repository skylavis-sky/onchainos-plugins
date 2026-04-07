use serde_json::{json, Value};
use crate::calldata;
use crate::config::{get_pool_by_name, CHAIN_ID, WAD};
use crate::onchainos;

/// Withdraw lent wstETH/WETH from IonPool.
///
/// Calls IonPool.withdraw(receiverOfUnderlying, amount_wad).
/// selector: 0xf3fef3a3
///
/// --pool: pool name or collateral symbol
/// --amount: amount in WAD (18 decimals)
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
    let withdraw_calldata = calldata::encode_withdraw(&wallet, amount_wad)?;

    if dry_run {
        let cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force --from {}",
            chain_id, pool.ion_pool, withdraw_calldata, wallet
        );
        eprintln!("[dry-run] withdraw from IonPool: {}", cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "action": "withdraw-lend",
            "pool": pool.name,
            "ionPool": pool.ion_pool,
            "lendSymbol": pool.lend_symbol,
            "wallet": wallet,
            "amountWad": amount_wad.to_string(),
            "amountHuman": format!("{:.6} {}", amount_human, pool.lend_symbol),
            "calldata": withdraw_calldata,
            "simulatedCommand": cmd
        }));
    }

    eprintln!("[ion-protocol] Withdrawing {} {} from IonPool {}...",
        amount_human, pool.lend_symbol, pool.ion_pool);
    let result = onchainos::wallet_contract_call(
        chain_id,
        pool.ion_pool,
        &withdraw_calldata,
        Some(&wallet),
        false,
    )?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;
    eprintln!("[ion-protocol] Withdraw tx: {}", tx_hash);

    Ok(json!({
        "ok": true,
        "action": "withdraw-lend",
        "pool": pool.name,
        "ionPool": pool.ion_pool,
        "lendSymbol": pool.lend_symbol,
        "wallet": wallet,
        "amountWad": amount_wad.to_string(),
        "amountHuman": format!("{:.6} {}", amount_human, pool.lend_symbol),
        "txHash": tx_hash
    }))
}
