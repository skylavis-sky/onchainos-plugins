use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::format_18;
use crate::config::{CD_POSITION, COORDINATOR, PARAMETER_STORE, RPC_URL, ZAPPER};
use crate::rpc;

/// Query Archimedes protocol parameters from on-chain state.
///
/// Calls (all read-only eth_call):
/// - Coordinator.getAvailableLeverage()
/// - ParameterStore.getArchToLevRatio()
/// - ParameterStore.getMaxNumberOfCycles()
/// - ParameterStore.getMinPositionCollateral()
/// - ParameterStore.getOriginationFeeRate()
pub async fn run() -> anyhow::Result<Value> {
    // Parallel async calls
    let available_leverage_fut = rpc::get_available_leverage(COORDINATOR, RPC_URL);
    let arch_ratio_fut = rpc::get_arch_to_lev_ratio(PARAMETER_STORE, RPC_URL);
    let max_cycles_fut = rpc::get_max_number_of_cycles(PARAMETER_STORE, RPC_URL);
    let min_collateral_fut = rpc::get_min_position_collateral(PARAMETER_STORE, RPC_URL);
    let fee_rate_fut = rpc::get_origination_fee_rate(PARAMETER_STORE, RPC_URL);

    let (available_leverage, arch_ratio, max_cycles, min_collateral, fee_rate) = tokio::join!(
        available_leverage_fut,
        arch_ratio_fut,
        max_cycles_fut,
        min_collateral_fut,
        fee_rate_fut,
    );

    let available_leverage =
        available_leverage.context("Failed to fetch available leverage from Coordinator")?;
    let arch_ratio = arch_ratio.context("Failed to fetch ARCH:lvUSD ratio from ParameterStore")?;
    let max_cycles = max_cycles.context("Failed to fetch max cycles from ParameterStore")?;
    let min_collateral =
        min_collateral.context("Failed to fetch min collateral from ParameterStore")?;
    let fee_rate = fee_rate.unwrap_or(0); // non-critical — don't fail if absent

    Ok(json!({
        "ok": true,
        "chain": "Ethereum Mainnet",
        "chainId": 1,
        "availableLvUSD": format_18(available_leverage),
        "availableLvUSDRaw": available_leverage.to_string(),
        "archToLevRatio": arch_ratio.to_string(),
        "maxCycles": max_cycles,
        "minPositionCollateralOUSD": format_18(min_collateral),
        "minPositionCollateralRaw": min_collateral.to_string(),
        "originationFeeRate": fee_rate.to_string(),
        "contracts": {
            "coordinator": COORDINATOR,
            "parameterStore": PARAMETER_STORE,
            "cdPosition": CD_POSITION,
            "zapper": ZAPPER,
        }
    }))
}
