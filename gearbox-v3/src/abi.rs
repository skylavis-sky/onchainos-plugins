/// ABI encoding helpers for Gearbox V3 MultiCall structures.
///
/// Gearbox V3 uses a MultiCall[] parameter for all write operations:
///   struct MultiCall { address target; bytes callData; }
///
/// ABI encoding for (address target, bytes[] callData) tuples in a dynamic array.

use anyhow::Context;
use crate::rpc::parse_address;

/// Encode a single inner call: (address target, bytes callData)
/// Returns the ABI-encoded bytes for one MultiCall element (without array wrapper).
///
/// ABI encoding of a struct (address, bytes) as a tuple:
///   [0x00..0x13] = address (padded to 32 bytes)
///   [0x20..0x3f] = offset to bytes data (relative to start of tuple = 64 = 0x40)
///   [0x40..0x5f] = length of bytes
///   [0x60....]   = bytes data (padded to 32-byte boundary)
fn encode_multicall_element(target: &str, call_data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let addr_bytes = parse_address(target)?;

    let mut head = Vec::new();
    let mut tail = Vec::new();

    // head[0]: address padded to 32 bytes
    head.extend_from_slice(&[0u8; 12]);
    head.extend_from_slice(&addr_bytes);

    // head[1]: offset to bytes data = 64 (0x40) — offset from start of this tuple
    let bytes_offset: u64 = 64;
    head.extend_from_slice(&[0u8; 24]);
    head.extend_from_slice(&bytes_offset.to_be_bytes());

    // tail: length of callData
    let len = call_data.len() as u64;
    tail.extend_from_slice(&[0u8; 24]);
    tail.extend_from_slice(&len.to_be_bytes());

    // tail: callData padded to 32-byte boundary
    tail.extend_from_slice(call_data);
    let pad = (32 - (call_data.len() % 32)) % 32;
    tail.extend(std::iter::repeat(0u8).take(pad));

    head.extend(tail);
    Ok(head)
}

/// Encode a MultiCall[] dynamic array for use in CreditFacadeV3 calls.
///
/// ABI encoding for dynamic array of tuples:
///   [0x00..0x1f] = length of array (number of elements)
///   For each element: head pointer (offset from start of array data body)
///   Then each element's encoded data
///
/// This follows ABI encoding rules for `(address,bytes)[]`.
pub fn encode_multicall_array(calls: &[(&str, Vec<u8>)]) -> anyhow::Result<Vec<u8>> {
    let n = calls.len();

    // Encode each element individually
    let mut encoded_elements: Vec<Vec<u8>> = Vec::new();
    for (target, call_data) in calls {
        let encoded = encode_multicall_element(target, call_data)?;
        encoded_elements.push(encoded);
    }

    let mut result = Vec::new();

    // Array length
    result.extend_from_slice(&[0u8; 24]);
    result.extend_from_slice(&(n as u64).to_be_bytes());

    // Head: offsets for each element (relative to start of array body, which is after the n offsets)
    // The array body starts at offset n * 32 (right after the n head pointers)
    let head_size = n * 32;
    let mut current_offset = head_size;

    for elem in &encoded_elements {
        result.extend_from_slice(&[0u8; 24]);
        result.extend_from_slice(&(current_offset as u64).to_be_bytes());
        current_offset += elem.len();
    }

    // Body: each encoded element
    for elem in &encoded_elements {
        result.extend_from_slice(elem);
    }

    Ok(result)
}

// ── Inner call encoders ───────────────────────────────────────────────────────

/// Encode addCollateral(address token, uint256 amount)
/// Selector: 0x6d75b9ee
pub fn encode_add_collateral(token_addr: &str, amount: u128) -> anyhow::Result<Vec<u8>> {
    let token = parse_address(token_addr)?;
    let mut data = hex::decode("6d75b9ee")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&token);
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&amount.to_be_bytes());
    Ok(data)
}

/// Encode increaseDebt(uint256 amount)
/// Selector: 0x2b7c7b11
pub fn encode_increase_debt(amount: u128) -> anyhow::Result<Vec<u8>> {
    let mut data = hex::decode("2b7c7b11")?;
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&amount.to_be_bytes());
    Ok(data)
}

/// Encode decreaseDebt(uint256 amount)
/// Selector: 0x2a7ba1f7
/// Use u128::MAX for repay-all (maps to type(uint256).max in practice; Gearbox treats any
/// value >= total debt as full repayment via its own capping logic).
pub fn encode_decrease_debt(amount: u128) -> anyhow::Result<Vec<u8>> {
    let mut data = hex::decode("2a7ba1f7")?;
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&amount.to_be_bytes());
    Ok(data)
}

/// Encode withdrawCollateral(address token, uint256 amount, address to)
/// Selector: 0x1f1088a0
pub fn encode_withdraw_collateral(
    token_addr: &str,
    amount: u128,
    to_addr: &str,
) -> anyhow::Result<Vec<u8>> {
    let token = parse_address(token_addr)?;
    let to = parse_address(to_addr)?;
    let mut data = hex::decode("1f1088a0")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&token);
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&amount.to_be_bytes());
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&to);
    Ok(data)
}

// ── Outer call encoders ───────────────────────────────────────────────────────

/// Encode openCreditAccount(address onBehalfOf, MultiCall[] calls, uint256 referralCode)
/// Selector: 0x92beab1d
///
/// ABI layout for function with (address, (address,bytes)[], uint256):
///   [0..3]   selector
///   [4..35]  onBehalfOf (address, padded 32 bytes)
///   [36..67] offset to calls array (from start of params = 4+32+32+32... → 96 = 0x60)
///   [68..99] referralCode (uint256)
///   [100+]   calls array data
pub fn encode_open_credit_account(
    on_behalf_of: &str,
    calls: &[(&str, Vec<u8>)],
    referral_code: u64,
) -> anyhow::Result<Vec<u8>> {
    let addr_bytes = parse_address(on_behalf_of)
        .with_context(|| format!("Invalid onBehalfOf address: {}", on_behalf_of))?;

    let calls_encoded = encode_multicall_array(calls)?;

    let mut data = hex::decode("92beab1d")?;

    // Param 1: onBehalfOf address (32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);

    // Param 2: offset to calls array — 3 params * 32 bytes = 96 (0x60)
    // (address = 32, offset to array = 32, referralCode = 32; array data follows)
    let array_offset: u64 = 96;
    data.extend_from_slice(&[0u8; 24]);
    data.extend_from_slice(&array_offset.to_be_bytes());

    // Param 3: referralCode (uint256)
    data.extend_from_slice(&[0u8; 24]);
    data.extend_from_slice(&referral_code.to_be_bytes());

    // Calls array (inline after fixed params)
    data.extend_from_slice(&calls_encoded);

    Ok(data)
}

/// Encode closeCreditAccount(address creditAccount, MultiCall[] calls)
/// Selector: 0x36b2ced3
pub fn encode_close_credit_account(
    credit_account: &str,
    calls: &[(&str, Vec<u8>)],
) -> anyhow::Result<Vec<u8>> {
    let addr_bytes = parse_address(credit_account)
        .with_context(|| format!("Invalid creditAccount address: {}", credit_account))?;

    let calls_encoded = encode_multicall_array(calls)?;

    let mut data = hex::decode("36b2ced3")?;

    // Param 1: creditAccount address (32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);

    // Param 2: offset to calls array — 2 params * 32 = 64 (0x40)
    let array_offset: u64 = 64;
    data.extend_from_slice(&[0u8; 24]);
    data.extend_from_slice(&array_offset.to_be_bytes());

    // Calls array
    data.extend_from_slice(&calls_encoded);

    Ok(data)
}

/// Encode multicall(address creditAccount, MultiCall[] calls)
/// Selector: 0xebe4107c
pub fn encode_multicall(
    credit_account: &str,
    calls: &[(&str, Vec<u8>)],
) -> anyhow::Result<Vec<u8>> {
    let addr_bytes = parse_address(credit_account)
        .with_context(|| format!("Invalid creditAccount address: {}", credit_account))?;

    let calls_encoded = encode_multicall_array(calls)?;

    let mut data = hex::decode("ebe4107c")?;

    // Param 1: creditAccount address (32 bytes)
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);

    // Param 2: offset to calls array — 2 params * 32 = 64 (0x40)
    let array_offset: u64 = 64;
    data.extend_from_slice(&[0u8; 24]);
    data.extend_from_slice(&array_offset.to_be_bytes());

    // Calls array
    data.extend_from_slice(&calls_encoded);

    Ok(data)
}

/// Encode ERC-20 approve(address spender, uint256 amount)
/// Selector: 0x095ea7b3
pub fn encode_erc20_approve(spender: &str, amount: u128) -> anyhow::Result<Vec<u8>> {
    let spender_bytes = parse_address(spender)?;
    let mut data = hex::decode("095ea7b3")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&spender_bytes);
    data.extend_from_slice(&[0u8; 16]);
    data.extend_from_slice(&amount.to_be_bytes());
    Ok(data)
}

/// Convert token amount from human-readable float to minimal units (u128).
pub fn human_to_minimal(amount: f64, decimals: u8) -> u128 {
    let factor = 10u128.pow(decimals as u32);
    (amount * factor as f64) as u128
}

/// Infer token decimals from well-known symbols.
pub fn infer_decimals(symbol: &str) -> u8 {
    match symbol.to_uppercase().as_str() {
        "USDC" | "USDT" | "USDC.E" | "USDCE" => 6,
        "WBTC" | "CBBTC" => 8,
        "WETH" | "ETH" => 18,
        _ => 18,
    }
}
