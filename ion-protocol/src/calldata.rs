/// ABI encoding for Ion Protocol contract calls.
///
/// All selectors verified via keccak256 in design.md §2.
/// Encoding done manually to avoid alloy dependency for simple fixed-layout functions.

use crate::rpc::parse_address;

/// ERC-20 approve(address spender, uint256 amount)
/// selector: 0x095ea7b3
pub fn encode_erc20_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let spender_bytes = parse_address(spender)?;
    let mut data = hex::decode("095ea7b3")?;
    // spender (32 bytes, left-padded)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&spender_bytes);
    // amount (32 bytes)
    data.extend_from_slice(&encode_u256(amount));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// ERC-20 approve with u128::MAX = unlimited
#[allow(dead_code)]
pub fn encode_erc20_approve_max(spender: &str) -> anyhow::Result<String> {
    let spender_bytes = parse_address(spender)?;
    let mut data = hex::decode("095ea7b3")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&spender_bytes);
    // uint256 max = 0xff...ff (32 bytes)
    data.extend_from_slice(&[0xffu8; 32]);
    Ok(format!("0x{}", hex::encode(&data)))
}

/// GemJoin.join(address user, uint256 amount)
/// selector: 0x3b4da69f
pub fn encode_gem_join(user: &str, amount: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("3b4da69f")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    data.extend_from_slice(&encode_u256(amount));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// GemJoin.exit(address user, uint256 amount)
/// selector: 0xef693bed
pub fn encode_gem_exit(user: &str, amount: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("ef693bed")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    data.extend_from_slice(&encode_u256(amount));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// IonPool.supply(address user, uint256 amount, bytes32[] proof=[])
/// selector: 0x7ca5643d
/// ABI: user(32) + amount(32) + offset_to_proof(32=0x60) + proof_len(32=0x00)
pub fn encode_supply(user: &str, amount: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("7ca5643d")?;
    // arg1: user (32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    // arg2: amount (32 bytes)
    data.extend_from_slice(&encode_u256(amount));
    // arg3: offset to dynamic bytes32[] array = 0x60 (96 = 3 args * 32)
    data.extend_from_slice(&encode_u256_raw(0x60));
    // arg4: array length = 0 (empty proof)
    data.extend_from_slice(&encode_u256_raw(0x00));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// IonPool.withdraw(address receiverOfUnderlying, uint256 amount)
/// selector: 0xf3fef3a3
pub fn encode_withdraw(receiver: &str, amount: u128) -> anyhow::Result<String> {
    let receiver_bytes = parse_address(receiver)?;
    let mut data = hex::decode("f3fef3a3")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&receiver_bytes);
    data.extend_from_slice(&encode_u256(amount));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// IonPool.depositCollateral(uint8 ilkIndex, address user, address depositor, uint256 amount, bytes32[] proof=[])
/// selector: 0x918a2f42
/// ABI: ilk(32) + user(32) + depositor(32) + amount(32) + offset_to_proof(32=0xa0) + proof_len(32=0x00)
pub fn encode_deposit_collateral(ilk_index: u8, user: &str, amount: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("918a2f42")?;
    // arg1: ilkIndex (uint8 padded to 32 bytes)
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    // arg2: user (32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    // arg3: depositor = user (32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    // arg4: amount (32 bytes)
    data.extend_from_slice(&encode_u256(amount));
    // arg5: offset to proof = 0xa0 (160 = 5 args * 32)
    data.extend_from_slice(&encode_u256_raw(0xa0));
    // arg6: proof length = 0
    data.extend_from_slice(&encode_u256_raw(0x00));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// IonPool.withdrawCollateral(uint8 ilkIndex, address user, address recipient, uint256 amount)
/// selector: 0x743f9c0c
pub fn encode_withdraw_collateral(ilk_index: u8, user: &str, amount: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("743f9c0c")?;
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    // recipient = user
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    data.extend_from_slice(&encode_u256(amount));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// IonPool.borrow(uint8 ilkIndex, address user, address recipient, uint256 normalizedDebt, bytes32[] proof=[])
/// selector: 0x9306f2f8
/// ABI: ilk(32) + user(32) + recipient(32) + normalizedDebt(32) + offset_to_proof(32=0xa0) + proof_len(32=0x00)
pub fn encode_borrow(ilk_index: u8, user: &str, normalized_debt: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("9306f2f8")?;
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    // recipient = user
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    data.extend_from_slice(&encode_u256(normalized_debt));
    // proof offset = 0xa0
    data.extend_from_slice(&encode_u256_raw(0xa0));
    // proof length = 0
    data.extend_from_slice(&encode_u256_raw(0x00));
    Ok(format!("0x{}", hex::encode(&data)))
}

/// IonPool.repay(uint8 ilkIndex, address user, address payer, uint256 normalizedDebt)
/// selector: 0x8459b437
/// Note: No dynamic proof array for repay.
pub fn encode_repay(ilk_index: u8, user: &str, normalized_debt: u128) -> anyhow::Result<String> {
    let user_bytes = parse_address(user)?;
    let mut data = hex::decode("8459b437")?;
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    // payer = user
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&user_bytes);
    data.extend_from_slice(&encode_u256(normalized_debt));
    Ok(format!("0x{}", hex::encode(&data)))
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// Encode u128 as a 32-byte big-endian uint256.
fn encode_u256(val: u128) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = val.to_be_bytes(); // 16 bytes
    out[16..32].copy_from_slice(&bytes);
    out
}

/// Encode a small usize/u64 as a 32-byte big-endian uint256.
fn encode_u256_raw(val: u64) -> [u8; 32] {
    let mut out = [0u8; 32];
    let bytes = val.to_be_bytes(); // 8 bytes
    out[24..32].copy_from_slice(&bytes);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_erc20_approve() {
        let result = encode_erc20_approve(
            "0x3bC3AC09d1ee05393F2848d82cb420f347954432",
            1_000_000_000_000_000_000u128,
        )
        .unwrap();
        assert!(result.starts_with("0x095ea7b3"));
        assert_eq!(result.len(), 2 + 8 + 64 + 64); // 0x + selector + 2 args
    }

    #[test]
    fn test_encode_supply() {
        let result = encode_supply(
            "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7",
            1_000_000_000_000_000_000u128,
        )
        .unwrap();
        assert!(result.starts_with("0x7ca5643d"));
        // selector(4) + user(32) + amount(32) + offset(32) + len(32) = 132 bytes = 264 hex + 2
        assert_eq!(result.len(), 2 + 8 + 64 * 4);
    }

    #[test]
    fn test_encode_borrow() {
        let result = encode_borrow(
            0,
            "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7",
            500_000_000_000_000u128,
        )
        .unwrap();
        assert!(result.starts_with("0x9306f2f8"));
        // selector(4) + ilk(32) + user(32) + recipient(32) + normalizedDebt(32) + offset(32) + len(32) = 196 bytes
        assert_eq!(result.len(), 2 + 8 + 64 * 6);
    }

    #[test]
    fn test_encode_repay() {
        let result = encode_repay(
            0,
            "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7",
            500_000_000_000_000u128,
        )
        .unwrap();
        assert!(result.starts_with("0x8459b437"));
        // selector(4) + ilk(32) + user(32) + payer(32) + normalizedDebt(32) = 132 bytes
        assert_eq!(result.len(), 2 + 8 + 64 * 4);
    }

    #[test]
    fn test_encode_deposit_collateral() {
        let result = encode_deposit_collateral(
            0,
            "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7",
            1_000_000_000_000_000_000u128,
        )
        .unwrap();
        assert!(result.starts_with("0x918a2f42"));
        // selector(4) + ilk(32) + user(32) + depositor(32) + amount(32) + offset(32) + len(32) = 196 bytes
        assert_eq!(result.len(), 2 + 8 + 64 * 6);
    }
}
