// ABI encoding helpers for CIAN Yield Layer contract calls

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

/// ERC-20 approve(address spender, uint256 amount)
pub fn encode_approve(spender: &str, amount: u128) -> String {
    format!("0x{}{}{}", SEL_APPROVE, pad_address(spender), pad_u128(amount))
}

/// optionalDeposit(address _token, uint256 _assets, address _receiver, address _referral)
/// selector: 0x32507a5f
pub fn encode_optional_deposit(
    token: &str,
    assets: u128,
    receiver: &str,
    referral: &str,
) -> String {
    format!(
        "0x{}{}{}{}{}",
        SEL_OPTIONAL_DEPOSIT,
        pad_address(token),
        pad_u128(assets),
        pad_address(receiver),
        pad_address(referral),
    )
}

/// deposit(uint256 _assets, address _receiver)
/// selector: 0x6e553f65
#[allow(dead_code)]
pub fn encode_deposit(assets: u128, receiver: &str) -> String {
    format!(
        "0x{}{}{}",
        SEL_DEPOSIT,
        pad_u128(assets),
        pad_address(receiver),
    )
}

/// requestRedeem(uint256 _shares, address _token)
/// ETH-class vaults (stETH, rsETH, ezETH, BTCLST, FBTC, uniBTC)
/// selector: 0x107703ab
pub fn encode_request_redeem_eth(shares: u128, token: &str) -> String {
    format!(
        "0x{}{}{}",
        SEL_REQUEST_REDEEM_ETH,
        pad_u128(shares),
        pad_address(token),
    )
}

/// requestRedeem(uint256 _shares)
/// BTC-class vaults (pumpBTC)
/// selector: 0xaa2f892d
pub fn encode_request_redeem_btc(shares: u128) -> String {
    format!("0x{}{}", SEL_REQUEST_REDEEM_BTC, pad_u128(shares))
}

/// ERC-20 balanceOf(address)
#[allow(dead_code)]
pub fn encode_balance_of(wallet: &str) -> String {
    format!("0x{}{}", SEL_BALANCE_OF, pad_address(wallet))
}

/// asset() — read vault underlying token address
#[allow(dead_code)]
pub fn encode_asset() -> String {
    format!("0x{}", SEL_ASSET)
}

/// exchangePrice() — read vault price per share
#[allow(dead_code)]
pub fn encode_exchange_price() -> String {
    format!("0x{}", SEL_EXCHANGE_PRICE)
}

/// maxDeposit(address)
#[allow(dead_code)]
pub fn encode_max_deposit(addr: &str) -> String {
    format!("0x{}{}", SEL_MAX_DEPOSIT, pad_address(addr))
}

/// Convert human-readable token amount with given decimals to raw u128.
pub fn to_raw(human: f64, decimals: u32) -> u128 {
    let scale = 10f64.powi(decimals as i32);
    (human * scale).round() as u128
}

/// Format raw token amount with given decimals to human-readable string.
#[allow(dead_code)]
pub fn from_raw(raw: u128, decimals: u32) -> String {
    let denom = 10u128.pow(decimals);
    let whole = raw / denom;
    let frac = raw % denom;
    format!("{}.{:0>width$}", whole, frac, width = decimals as usize)
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_string()
}

/// Decode a hex-encoded uint256 from an eth_call response (0x-prefixed).
#[allow(dead_code)]
pub fn decode_uint256_from_hex(hex_str: &str) -> anyhow::Result<u128> {
    let cleaned = hex_str.trim_start_matches("0x");
    if cleaned.len() > 32 {
        let trimmed = &cleaned[cleaned.len() - 32..];
        Ok(u128::from_str_radix(trimmed, 16)?)
    } else {
        Ok(u128::from_str_radix(cleaned, 16)?)
    }
}

/// Decode a hex-encoded address from an eth_call response (0x-prefixed, last 20 bytes).
#[allow(dead_code)]
pub fn decode_address_from_hex(hex_str: &str) -> String {
    let cleaned = hex_str.trim_start_matches("0x");
    if cleaned.len() >= 40 {
        format!("0x{}", &cleaned[cleaned.len() - 40..])
    } else {
        format!("0x{}", cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_address() {
        let addr = "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d";
        let padded = pad_address(addr);
        assert_eq!(padded.len(), 64);
        assert!(padded.starts_with("000000000000000000000000"));
    }

    #[test]
    fn test_encode_approve_length() {
        let cd = encode_approve("0xB13aa2d0345b0439b064f26B82D8dCf3f508775d", MAX_UINT256);
        // 0x + 8 (sel) + 64 (addr) + 64 (amount) = 138 chars
        assert_eq!(cd.len(), 138);
    }

    #[test]
    fn test_encode_optional_deposit_length() {
        let cd = encode_optional_deposit(
            "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d",
            1_000_000_000_000_000_000u128,
            "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d",
            ZERO_ADDRESS,
        );
        // 0x + 8 + 64 + 64 + 64 + 64 = 266 chars
        assert_eq!(cd.len(), 266);
    }

    #[test]
    fn test_encode_request_redeem_eth_length() {
        let cd = encode_request_redeem_eth(
            1_000_000_000_000_000_000u128,
            "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d",
        );
        // 0x + 8 + 64 + 64 = 138 chars
        assert_eq!(cd.len(), 138);
    }

    #[test]
    fn test_encode_request_redeem_btc_length() {
        let cd = encode_request_redeem_btc(1_000_000_000_000_000_000u128);
        // 0x + 8 + 64 = 74 chars
        assert_eq!(cd.len(), 74);
    }
}
