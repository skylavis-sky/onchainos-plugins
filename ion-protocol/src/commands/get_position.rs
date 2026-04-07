use serde_json::{json, Value};
use crate::config::{POOLS, RAY, WAD};
use crate::rpc;
use crate::onchainos;

/// Show collateral deposited and debt for a wallet address across all pools.
///
/// Uses vault(ilkIndex, address) -> (collateral_wad, normalizedDebt_wad)
/// actual_debt = normalizedDebt * rate / RAY
pub async fn run(chain_id: u64, from: Option<&str>) -> anyhow::Result<Value> {
    let wallet = match from {
        Some(addr) => addr.to_string(),
        None => onchainos::resolve_wallet(chain_id)
            .map_err(|e| anyhow::anyhow!("Could not resolve wallet: {}", e))?,
    };

    let mut positions: Vec<Value> = Vec::new();
    let mut has_any_position = false;

    for pool in POOLS {
        let pos = fetch_position(pool, &wallet).await?;
        let has_position = pos["collateralWad"].as_str().map(|s| s != "0").unwrap_or(false)
            || pos["normalizedDebtWad"].as_str().map(|s| s != "0").unwrap_or(false)
            || pos["lendBalanceWad"].as_str().map(|s| s != "0").unwrap_or(false);
        if has_position {
            has_any_position = true;
        }
        positions.push(pos);
    }

    Ok(json!({
        "ok": true,
        "wallet": wallet,
        "chain": "Ethereum Mainnet",
        "chainId": 1,
        "hasPositions": has_any_position,
        "positions": positions
    }))
}

async fn fetch_position(pool: &crate::config::PoolConfig, wallet: &str) -> anyhow::Result<Value> {
    // Get vault (collateral + normalized debt)
    let (collateral_wad, normalized_debt_wad) =
        rpc::get_vault(pool.ion_pool, pool.ilk_index, wallet).await
            .unwrap_or((0, 0));

    // Get current accumulated rate to compute actual debt
    let rate = rpc::get_rate(pool.ion_pool, pool.ilk_index).await
        .unwrap_or(RAY);

    // Get lender supply token balance
    let lend_balance_wad = rpc::get_ion_balance(pool.ion_pool, wallet).await
        .unwrap_or(0);

    // actual_debt = normalizedDebt * rate / RAY
    let actual_debt_wad: u128 = if normalized_debt_wad == 0 || rate == 0 {
        0
    } else {
        // Use f64 to avoid overflow: normalizedDebt * rate / RAY
        let actual_f64 = (normalized_debt_wad as f64) * (rate as f64) / (RAY as f64);
        actual_f64 as u128
    };

    let collateral_human = collateral_wad as f64 / WAD as f64;
    let actual_debt_human = actual_debt_wad as f64 / WAD as f64;
    let lend_balance_human = lend_balance_wad as f64 / WAD as f64;

    Ok(json!({
        "pool": pool.name,
        "ionPool": pool.ion_pool,
        "collateral": {
            "symbol": pool.collateral_symbol,
            "wad": collateral_wad.to_string(),
            "human": format!("{:.6} {}", collateral_human, pool.collateral_symbol)
        },
        "debt": {
            "normalizedDebtWad": normalized_debt_wad.to_string(),
            "actualDebtWad": actual_debt_wad.to_string(),
            "human": format!("{:.6} {}", actual_debt_human, pool.lend_symbol),
            "lendToken": pool.lend_symbol,
            "rateRay": rate.to_string()
        },
        "lendPosition": {
            "balanceWad": lend_balance_wad.to_string(),
            "human": format!("{:.6} ion-{}", lend_balance_human, pool.lend_symbol),
            "description": "ion-token supply balance (earns yield automatically)"
        },
        "collateralWad": collateral_wad.to_string(),
        "normalizedDebtWad": normalized_debt_wad.to_string(),
        "lendBalanceWad": lend_balance_wad.to_string()
    }))
}
