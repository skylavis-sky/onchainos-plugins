/// ABI encoding helpers for Archimedes Finance contract calls.
///
/// All selectors verified with pycryptodome keccak256 using ERC-20 approve
/// (0x095ea7b3) as a known-correct cross-check.

use crate::rpc::{encode_address, encode_bool, encode_u256, encode_u256_u64};

/// Encode ERC-20 approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3 (verified ERC-20 standard)
pub fn encode_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let mut data = hex::decode("095ea7b3").unwrap();
    data.extend_from_slice(&encode_address(spender)?);
    data.extend_from_slice(&encode_u256(amount));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Encode ERC-20 approve with uint256.max (unlimited approval)
#[allow(dead_code)]
pub fn encode_approve_max(spender: &str) -> anyhow::Result<String> {
    let mut data = hex::decode("095ea7b3").unwrap();
    data.extend_from_slice(&encode_address(spender)?);
    // uint256.max = 32 bytes of 0xff
    data.extend_from_slice(&[0xff_u8; 32]);
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Encode ERC-721 setApprovalForAll(address operator, bool approved)
/// Selector: 0xa22cb465 (verified ERC-721 standard)
pub fn encode_set_approval_for_all(operator: &str, approved: bool) -> anyhow::Result<String> {
    let mut data = hex::decode("a22cb465").unwrap();
    data.extend_from_slice(&encode_address(operator)?);
    data.extend_from_slice(&encode_bool(approved));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Encode Zapper.zapIn(
///   uint256 stableCoinAmount,
///   uint256 cycles,
///   uint256 archMinAmount,
///   uint256 ousdMinAmount,
///   uint16  maxSlippageAllowed,
///   address addressBaseStable,
///   bool    useUserArch
/// )
/// Selector: 0x657d81f7 (verified)
pub fn encode_zap_in(
    stable_coin_amount: u128,
    cycles: u64,
    arch_min_amount: u128,
    ousd_min_amount: u128,
    max_slippage_bps: u16,
    address_base_stable: &str,
    use_user_arch: bool,
) -> anyhow::Result<String> {
    let mut data = hex::decode("657d81f7").unwrap();
    data.extend_from_slice(&encode_u256(stable_coin_amount));
    data.extend_from_slice(&encode_u256_u64(cycles));
    data.extend_from_slice(&encode_u256(arch_min_amount));
    data.extend_from_slice(&encode_u256(ousd_min_amount));
    // uint16 padded to 32 bytes
    data.extend_from_slice(&encode_u256_u64(max_slippage_bps as u64));
    data.extend_from_slice(&encode_address(address_base_stable)?);
    data.extend_from_slice(&encode_bool(use_user_arch));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Encode LeverageEngine.unwindLeveragedPosition(uint256 positionTokenId, uint256 minReturnedOUSD)
/// Selector: 0xdafccdd9 (verified)
pub fn encode_unwind_leveraged_position(
    position_token_id: u128,
    min_returned_ousd: u128,
) -> anyhow::Result<String> {
    let mut data = hex::decode("dafccdd9").unwrap();
    data.extend_from_slice(&encode_u256(position_token_id));
    data.extend_from_slice(&encode_u256(min_returned_ousd));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Convert a human-readable amount to on-chain minimal units.
/// USDC/USDT: 6 decimals — 1.0 → 1_000_000
/// DAI/OUSD:  18 decimals — 1.0 → 1_000_000_000_000_000_000
pub fn human_to_minimal(amount: f64, decimals: u8) -> u128 {
    let factor = 10u128.pow(decimals as u32);
    (amount * factor as f64) as u128
}

/// Format a u128 on-chain value as a human-readable decimal string.
pub fn format_18(val: u128) -> String {
    let integer = val / 1_000_000_000_000_000_000u128;
    let frac = (val % 1_000_000_000_000_000_000u128) / 1_000_000_000_000u128; // 6 decimal places
    format!("{}.{:06}", integer, frac)
}
