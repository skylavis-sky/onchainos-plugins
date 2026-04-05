// abi.rs — ABI encoding helpers for Fenix Finance contracts

use crate::rpc::pad_address;

/// Zero address constant
pub const ZERO_ADDR: &str = "0x0000000000000000000000000000000000000000";

/// Encode a u128 as a 32-byte padded hex string (no 0x prefix)
pub fn encode_u256(val: u128) -> String {
    format!("{:064x}", val)
}

/// Encode a i32 tick value as 32-byte two's complement hex (no 0x prefix)
pub fn encode_i24(val: i32) -> String {
    // int24 as int256: sign-extend
    if val >= 0 {
        format!("{:064x}", val as u64)
    } else {
        // two's complement for negative: cast to u128 via i128
        format!("{:064x}", (val as i128) as u128)
    }
}

/// Encode approve(address,uint256) calldata
/// Selector: 0x095ea7b3
pub fn encode_approve(spender: &str, amount: u128) -> String {
    format!("0x095ea7b3{}{}", pad_address(spender), encode_u256(amount))
}

/// Encode quoteExactInputSingle calldata for Fenix QuoterV2
/// Selector: 0x5e5e6e0f
/// Struct fields (4): tokenIn, tokenOut, amountIn, limitSqrtPrice
pub fn encode_quote_exact_input_single(
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    limit_sqrt_price: u128,
) -> String {
    // The struct is ABI-encoded as a tuple:
    // offset to struct data (0x20) + struct fields
    // Actually for eth_call with a struct param, the encoding is:
    // selector + offset(32) + [tokenIn, tokenOut, amountIn, limitSqrtPrice]
    // But since it's a direct struct (not dynamic), it's inline:
    // selector + 4 * 32 bytes
    format!(
        "0x5e5e6e0f{}{}{}{}",
        pad_address(token_in),
        pad_address(token_out),
        encode_u256(amount_in),
        encode_u256(limit_sqrt_price),
    )
}

/// Encode exactInputSingle calldata for Fenix SwapRouter (Algebra V4)
/// Selector: 0x1679c792
/// Struct fields (8): tokenIn, tokenOut, deployer(=0), recipient, deadline, amountIn, amountOutMinimum, limitSqrtPrice(=0)
pub fn encode_exact_input_single(
    token_in: &str,
    token_out: &str,
    recipient: &str,
    deadline: u128,
    amount_in: u128,
    amount_out_minimum: u128,
) -> String {
    format!(
        "0x1679c792{}{}{}{}{}{}{}{}",
        pad_address(token_in),
        pad_address(token_out),
        pad_address(ZERO_ADDR), // deployer = address(0)
        pad_address(recipient),
        encode_u256(deadline),
        encode_u256(amount_in),
        encode_u256(amount_out_minimum),
        encode_u256(0), // limitSqrtPrice = 0
    )
}

/// Encode poolByPair(address,address) calldata for Algebra Factory
/// Selector: 0xd9a641e1
pub fn encode_pool_by_pair(token0: &str, token1: &str) -> String {
    format!(
        "0xd9a641e1{}{}",
        pad_address(token0),
        pad_address(token1),
    )
}

/// Encode mint calldata for NFPM
/// Selector: 0x9cc1a283
/// Struct fields (10): token0, token1, tickLower, tickUpper,
///   amount0Desired, amount1Desired, amount0Min, amount1Min, recipient, deadline
pub fn encode_nfpm_mint(
    token0: &str,
    token1: &str,
    tick_lower: i32,
    tick_upper: i32,
    amount0_desired: u128,
    amount1_desired: u128,
    amount0_min: u128,
    amount1_min: u128,
    recipient: &str,
    deadline: u128,
) -> String {
    format!(
        "0x9cc1a283{}{}{}{}{}{}{}{}{}{}",
        pad_address(token0),
        pad_address(token1),
        encode_i24(tick_lower),
        encode_i24(tick_upper),
        encode_u256(amount0_desired),
        encode_u256(amount1_desired),
        encode_u256(amount0_min),
        encode_u256(amount1_min),
        pad_address(recipient),
        encode_u256(deadline),
    )
}
