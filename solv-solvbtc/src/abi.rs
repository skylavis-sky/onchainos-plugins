// ABI encoding helpers for all Solv SolvBTC contract calls

use crate::config::*;

/// Pad an address (strip 0x, left-pad to 32 bytes).
pub fn pad_address(addr: &str) -> String {
    let cleaned = addr.trim_start_matches("0x").to_lowercase();
    format!("{:0>64}", cleaned)
}

/// Pad a u128 to 32 bytes.
pub fn pad_u128(val: u128) -> String {
    format!("{:064x}", val)
}

/// Pad a u64 to 32 bytes.
pub fn pad_u64(val: u64) -> String {
    format!("{:064x}", val)
}

/// ERC-20 approve(address spender, uint256 amount)
/// Returns full hex calldata (with 0x prefix).
pub fn encode_approve(spender: &str, amount: u128) -> String {
    format!("0x{}{}{}", SEL_APPROVE, pad_address(spender), pad_u128(amount))
}

/// RouterV2.deposit(address targetToken_, address currency_, uint256 currencyAmount_, uint256 minimumTargetTokenAmount_, uint64 expireTime_)
pub fn encode_router_deposit(
    target_token: &str,
    currency: &str,
    currency_amount: u128,
    min_target_amount: u128,
    expire_time: u64,
) -> String {
    format!(
        "0x{}{}{}{}{}{}",
        SEL_ROUTER_DEPOSIT,
        pad_address(target_token),
        pad_address(currency),
        pad_u128(currency_amount),
        pad_u128(min_target_amount),
        pad_u64(expire_time),
    )
}

/// RouterV2.withdrawRequest(address targetToken_, address currency_, uint256 withdrawAmount_)
pub fn encode_withdraw_request(
    target_token: &str,
    currency: &str,
    withdraw_amount: u128,
) -> String {
    format!(
        "0x{}{}{}{}",
        SEL_ROUTER_WITHDRAW_REQUEST,
        pad_address(target_token),
        pad_address(currency),
        pad_u128(withdraw_amount),
    )
}

/// RouterV2.cancelWithdrawRequest(address targetToken_, address redemption_, uint256 redemptionId_)
pub fn encode_cancel_withdraw_request(
    target_token: &str,
    redemption: &str,
    redemption_id: u128,
) -> String {
    format!(
        "0x{}{}{}{}",
        SEL_ROUTER_CANCEL_WITHDRAW,
        pad_address(target_token),
        pad_address(redemption),
        pad_u128(redemption_id),
    )
}

/// XSolvBTCPool.deposit(uint256 solvBtcAmount_)
pub fn encode_xpool_deposit(solv_btc_amount: u128) -> String {
    format!("0x{}{}", SEL_XPOOL_DEPOSIT, pad_u128(solv_btc_amount))
}

/// XSolvBTCPool.withdraw(uint256 xSolvBtcAmount_)
pub fn encode_xpool_withdraw(xsolv_btc_amount: u128) -> String {
    format!("0x{}{}", SEL_XPOOL_WITHDRAW, pad_u128(xsolv_btc_amount))
}

/// ERC-20 balanceOf(address)
pub fn encode_balance_of(wallet: &str) -> String {
    format!("0x{}{}", SEL_BALANCE_OF, pad_address(wallet))
}

/// Convert human-readable WBTC (e.g. "0.001") to raw u128 (8 decimals).
pub fn wbtc_to_raw(human: f64) -> u128 {
    (human * 1e8_f64).round() as u128
}

/// Convert human-readable SolvBTC / xSolvBTC (18 decimals) to raw u128.
pub fn solvbtc_to_raw(human: f64) -> u128 {
    // Use integer arithmetic to avoid float precision issues for large values
    let whole = human.floor() as u128;
    let frac = ((human - human.floor()) * 1e18_f64).round() as u128;
    whole * 10u128.pow(SOLVBTC_DECIMALS) + frac
}

/// Format raw SolvBTC (18 decimals) to human-readable string.
pub fn raw_to_solvbtc(raw: u128) -> String {
    let whole = raw / 10u128.pow(SOLVBTC_DECIMALS);
    let frac = raw % 10u128.pow(SOLVBTC_DECIMALS);
    format!("{}.{:018}", whole, frac)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Format raw WBTC (8 decimals) to human-readable string.
#[allow(dead_code)]
pub fn raw_to_wbtc(raw: u128) -> String {
    let whole = raw / 10u128.pow(WBTC_DECIMALS);
    let frac = raw % 10u128.pow(WBTC_DECIMALS);
    format!("{}.{:08}", whole, frac)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Decode a hex-encoded uint256 from an eth_call response (0x-prefixed, 64 hex chars).
pub fn decode_uint256_from_hex(hex_str: &str) -> anyhow::Result<u128> {
    let cleaned = hex_str.trim_start_matches("0x");
    // Take the last 32 bytes (64 hex chars) in case it's longer
    let trimmed = if cleaned.len() > 32 {
        &cleaned[cleaned.len() - 32..]
    } else {
        cleaned
    };
    Ok(u128::from_str_radix(trimmed, 16)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_address() {
        let addr = "0x3647c54c4c2C65bC7a2D63c0Da2809B399DBBDC0";
        let padded = pad_address(addr);
        assert_eq!(padded.len(), 64);
        assert!(padded.starts_with("000000000000000000000000"));
    }

    #[test]
    fn test_encode_approve_length() {
        let cd = encode_approve("0x92E8A4407FD1ae7a53a32f1f832184edF071080A", 100_000);
        // 0x + 8 (sel) + 64 (addr) + 64 (amount) = 138 chars
        assert_eq!(cd.len(), 138);
    }

    #[test]
    fn test_wbtc_raw() {
        assert_eq!(wbtc_to_raw(0.001), 100_000);
        assert_eq!(wbtc_to_raw(1.0), 100_000_000);
    }
}
