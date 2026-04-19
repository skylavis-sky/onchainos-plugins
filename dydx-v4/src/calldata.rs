/// ABI calldata encoding for dYdX V4 EVM bridge operations.
///
/// bridge(uint256,bytes,bytes) — selector 0x1d45e29c
/// Verified: keccak256("bridge(uint256,bytes,bytes)")[:4] = 0x1d45e29c
///
/// ABI layout for bridge(uint256 amount, bytes accAddress, bytes memo):
///   [selector 4B]
///   [amount          32B  — uint256 value]
///   [offset_accAddr  32B  — offset from start of params to accAddr length word]
///   [offset_memo     32B  — offset from start of params to memo length word]
///   [accAddr_len     32B]
///   [accAddr_data    ceil(len/32)*32 B]
///   [memo_len        32B  — 0 for empty]

/// Encode the bridge calldata.
///
/// - `amount_wei`: DYDX amount in base units (18 decimals)
/// - `dydx_address`: dYdX bech32 address string ("dydx1..."), encoded as UTF-8 bytes
///
/// Returns 0x-prefixed hex calldata string.
pub fn encode_bridge(amount_wei: u128, dydx_address: &str) -> String {
    // selector
    let selector = "1d45e29c";

    // amount as 32-byte big-endian
    let amount_hex = format!("{:064x}", amount_wei);

    // accAddress = UTF-8 bytes of the dydx bech32 address
    let acc_bytes = dydx_address.as_bytes();
    let acc_len = acc_bytes.len();
    let acc_padded_len = (acc_len + 31) / 32 * 32;
    let acc_hex: String = acc_bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let acc_padding = "0".repeat((acc_padded_len - acc_len) * 2);

    // memo = empty bytes — length 0, no data
    let memo_len = 0usize;

    // ABI tuple offsets (from start of params = after selector):
    //   param 0: amount (uint256) — value type, 32B at position 0
    //   param 1: accAddress (bytes) — dynamic; offset_to_accAddr relative to start of params
    //   param 2: memo (bytes) — dynamic; offset_to_memo relative to start of params
    //
    // Layout:
    //   [0x000] amount          (32B)
    //   [0x020] offset_accAddr  (32B) = 0x60 (96 = 3 * 32: skip amount + two offsets)
    //   [0x040] offset_memo     (32B) = 0x60 + 32 + acc_padded_len
    //   [0x060] acc_len         (32B)
    //   [0x080] acc_data        (acc_padded_len B)
    //   [0x080+acc_padded_len] memo_len (32B) = 0

    let offset_acc: u128 = 0x60; // 3 × 32 bytes past the start of params
    let offset_memo: u128 = 0x60 + 32 + acc_padded_len as u128;

    format!(
        "0x{}{}{}{}{}{}{}{}",
        selector,
        amount_hex,
        format!("{:064x}", offset_acc),
        format!("{:064x}", offset_memo),
        format!("{:064x}", acc_len),
        acc_hex,
        acc_padding,
        format!("{:064x}", memo_len),
    )
}

/// Parse a human-readable float amount into wei (18 decimals).
/// Handles inputs like "100", "0.5", "1000.123456789012345678".
pub fn parse_dydx_amount(amount_str: &str) -> anyhow::Result<u128> {
    const DECIMALS: u32 = 18;
    let parts: Vec<&str> = amount_str.split('.').collect();
    match parts.len() {
        1 => {
            let whole: u128 = parts[0]
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount_str))?;
            Ok(whole * 10u128.pow(DECIMALS))
        }
        2 => {
            let whole: u128 = parts[0]
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount_str))?;
            let frac_str = parts[1];
            let frac_len = frac_str.len() as u32;
            if frac_len > DECIMALS {
                anyhow::bail!("Too many decimal places: {} (max {})", frac_len, DECIMALS);
            }
            let frac: u128 = frac_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid fractional part: {}", frac_str))?;
            let frac_scaled = frac * 10u128.pow(DECIMALS - frac_len);
            Ok(whole * 10u128.pow(DECIMALS) + frac_scaled)
        }
        _ => anyhow::bail!("Invalid amount: {}", amount_str),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dydx_amount_whole() {
        assert_eq!(
            parse_dydx_amount("100").unwrap(),
            100_000_000_000_000_000_000u128
        );
    }

    #[test]
    fn test_parse_dydx_amount_fractional() {
        assert_eq!(
            parse_dydx_amount("0.5").unwrap(),
            500_000_000_000_000_000u128
        );
    }

    #[test]
    fn test_encode_bridge_selector() {
        let calldata = encode_bridge(100_000_000_000_000_000_000u128, "dydx1abc");
        assert!(calldata.starts_with("0x1d45e29c"), "selector mismatch: {}", &calldata[..10]);
    }

    #[test]
    fn test_encode_bridge_structure() {
        // addr = "dydx1" (5 bytes), acc_padded = 32 bytes
        let addr = "dydx1";
        let amount_wei = 100_000_000_000_000_000_000u128;
        let cd = encode_bridge(amount_wei, addr);
        // Strip 0x prefix and selector (4 bytes = 8 hex chars)
        let data = &cd[2 + 8..];
        // First 32 bytes: amount
        let amount_field = &data[..64];
        let expected_amount = format!("{:064x}", amount_wei);
        assert_eq!(amount_field, expected_amount);
        // Next 32 bytes: offset_accAddr = 0x60
        let offset_acc_field = &data[64..128];
        assert_eq!(offset_acc_field, format!("{:064x}", 0x60u128));
    }
}
