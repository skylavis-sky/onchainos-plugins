/// Resolve a token symbol or hex address to a checksummed hex address.
/// If the input looks like a hex address (starts with 0x and >= 40 chars), return as-is.
pub fn resolve_token_address(symbol: &str, chain_id: u64) -> String {
    match (symbol.to_uppercase().as_str(), chain_id) {
        // Polygon (137) — MATIC/POL/WMATIC/WPOL all map to Wrapped MATIC (WPOL)
        ("MATIC", 137) | ("WMATIC", 137) | ("POL", 137) | ("WPOL", 137)
            => "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270".to_string(),
        // USDC (native Polygon USDC, 6 decimals)
        ("USDC", 137) => "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359".to_string(),
        // USDC.e (bridged USDC, 6 decimals)
        ("USDC.E", 137) | ("USDCE", 137)
            => "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174".to_string(),
        // USDT (PoS bridged, 6 decimals)
        ("USDT", 137) => "0xc2132D05D31c914a87C6611C10748AEb04B58e8f".to_string(),
        // WETH (PoS bridged, 18 decimals)
        ("WETH", 137) | ("ETH", 137)
            => "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619".to_string(),
        // QUICK (new governance token, 18 decimals)
        ("QUICK", 137) => "0xB5C064F955D8e7F38fE0460C556a72987494eE17".to_string(),
        // Pass-through: assume already a hex address
        _ => symbol.to_string(),
    }
}

/// Returns true if the given symbol represents native MATIC (not WMATIC ERC-20).
/// "MATIC" and "POL" are treated as native for swap purposes.
pub fn is_native_matic(symbol: &str) -> bool {
    matches!(symbol.to_uppercase().as_str(), "MATIC" | "POL")
}

/// Returns the number of decimals for a known token address on Polygon.
/// Falls back to 18 for unknown tokens.
pub fn token_decimals(addr: &str) -> u32 {
    match addr.to_lowercase().as_str() {
        // 6-decimal tokens on Polygon
        "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359" => 6, // USDC (native)
        "0x2791bca1f2de4661ed88a30c99a7a9449aa84174" => 6, // USDC.e
        "0xc2132d05d31c914a87c6611c10748aeb04b58e8f" => 6, // USDT
        // 18-decimal tokens
        _ => 18,
    }
}

pub const ROUTER_V2: &str = "0xa5E0829CaCEd8fFDD4De3c43696c57F7D7A678ff";
pub const FACTORY_V2: &str = "0x5757371414417b8C6CAad45bAeF941aBc7d3Ab32";
pub const WMATIC: &str = "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270";
pub const POLYGON_RPC: &str = "https://polygon-bor-rpc.publicnode.com";
pub const CHAIN_ID: u64 = 137;

/// Deadline: current unix timestamp + 20 minutes.
pub fn deadline() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 1200
}

/// Apply 0.5% slippage (995/1000).
pub fn apply_slippage(amount: u128) -> u128 {
    amount * 995 / 1000
}

/// Pad an address (with or without 0x) to 32 bytes hex (no 0x prefix).
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Pad a u128 to 32 bytes hex (no 0x prefix).
pub fn pad_u256(val: u128) -> String {
    format!("{:0>64x}", val)
}

/// Encode an address[] dynamic array for ABI encoding.
/// Returns the raw hex (no 0x) for the array portion: length + elements.
pub fn encode_address_array(addrs: &[&str]) -> String {
    let mut out = String::new();
    // length
    out.push_str(&format!("{:0>64x}", addrs.len()));
    // elements
    for addr in addrs {
        out.push_str(&pad_address(addr));
    }
    out
}

/// Build ERC-20 approve calldata: approve(address spender, uint256 amount).
/// Selector: 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_padded = pad_address(spender);
    let amount_hex = pad_u256(amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}
