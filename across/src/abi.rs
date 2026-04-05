/// ABI encoding helpers for Across Protocol SpokePool calls.
///
/// All encoding follows Ethereum ABI specification (EIP-712 / Solidity ABI).

fn pad_address(addr: &str) -> String {
    // Strip 0x prefix, left-pad to 32 bytes (64 hex chars)
    let clean = addr.trim_start_matches("0x").trim_start_matches("0X");
    format!("{:0>64}", clean)
}

fn pad_u256(val: u128) -> String {
    format!("{:064x}", val)
}

fn pad_u256_str(val: &str) -> String {
    // val may be decimal string
    let n: u128 = val.parse().unwrap_or(0);
    format!("{:064x}", n)
}

fn pad_u32(val: u32) -> String {
    format!("{:064x}", val)
}

fn pad_u64(val: u64) -> String {
    format!("{:064x}", val)
}

/// Encode ERC-20 approve(spender, amount) calldata.
/// selector = 0x095ea7b3
/// amount = u128::MAX for unlimited approval
pub fn encode_approve(spender: &str, amount: u128) -> String {
    format!(
        "0x095ea7b3{}{}",
        pad_address(spender),
        pad_u256(amount)
    )
}

/// Encode SpokePool.depositV3 calldata.
/// selector = 0x7b939232
///
/// Parameters (in order):
///   depositor          address
///   recipient          address
///   input_token        address
///   output_token       address
///   input_amount       uint256  (decimal string)
///   output_amount      uint256  (decimal string)
///   destination_chain  uint256
///   exclusive_relayer  address
///   quote_timestamp    uint32
///   fill_deadline      uint32
///   exclusivity_deadline uint32
///   message            bytes    (empty = 0x)
///
/// ABI encoding for dynamic `bytes` (empty):
///   offset  = 0x180 (384 decimal = 12 static words * 32 bytes each)
///   length  = 0x00
///   (no data words needed for empty bytes)
pub fn encode_deposit_v3(
    depositor: &str,
    recipient: &str,
    input_token: &str,
    output_token: &str,
    input_amount: &str,
    output_amount: &str,
    destination_chain_id: u64,
    exclusive_relayer: &str,
    quote_timestamp: u32,
    fill_deadline: u32,
    exclusivity_deadline: u32,
) -> String {
    // Offset for `bytes message`: 12 static params * 32 bytes = 384 = 0x180
    let message_offset = pad_u256(384u128);
    // Empty bytes: length = 0, no data
    let message_length = pad_u256(0u128);

    format!(
        "0x7b939232{}{}{}{}{}{}{}{}{}{}{}{}{}",
        pad_address(depositor),
        pad_address(recipient),
        pad_address(input_token),
        pad_address(output_token),
        pad_u256_str(input_amount),
        pad_u256_str(output_amount),
        pad_u64(destination_chain_id),
        pad_address(exclusive_relayer),
        pad_u32(quote_timestamp),
        pad_u32(fill_deadline),
        pad_u32(exclusivity_deadline),
        message_offset,
        message_length,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_approve_length() {
        let data = encode_approve("0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5", u128::MAX);
        // 0x + 8 (selector) + 64 (spender) + 64 (amount) = 136 + 2 = 138
        assert_eq!(data.len(), 2 + 8 + 64 + 64);
    }

    #[test]
    fn test_encode_deposit_v3_length() {
        let data = encode_deposit_v3(
            "0x1111111111111111111111111111111111111111",
            "0x2222222222222222222222222222222222222222",
            "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
            "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
            "100000000",
            "99968700",
            10,
            "0x0000000000000000000000000000000000000000",
            1775384651,
            1775391851,
            0,
        );
        // 0x + 8 (selector) + 13 * 64 (params) = 2 + 8 + 832 = 842
        assert_eq!(data.len(), 2 + 8 + 13 * 64);
    }
}
