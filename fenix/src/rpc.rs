// rpc.rs — Direct eth_call utilities for Blast RPC

/// Perform a raw JSON-RPC eth_call
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {"to": to, "data": data},
            "latest"
        ],
        "id": 1
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// allowance(address owner, address spender) selector = 0xdd62ed3e
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner_padded = pad_address(owner);
    let spender_padded = pad_address(spender);
    let data = format!("0xdd62ed3e{}{}", owner_padded, spender_padded);
    let hex = eth_call(token, &data, rpc_url).await?;
    Ok(decode_uint256_u128(&hex))
}

/// decimals() selector = 0x313ce567
pub async fn get_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    Ok(decode_uint256_u128(&hex) as u8)
}

/// Pad a 20-byte address to 32 bytes (no 0x prefix)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// Decode the first ABI word (word 0) of an eth_call response as u128.
/// QuoterV2 and other multi-word responses return amountOut in word 0.
pub fn decode_uint256_u128(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    // Take first 64 hex chars (32 bytes = one ABI word), then last 32 chars of that word
    let word0 = if clean.len() >= 64 {
        &clean[..64]
    } else {
        clean
    };
    let last32 = if word0.len() >= 32 {
        &word0[word0.len() - 32..]
    } else {
        word0
    };
    u128::from_str_radix(last32, 16).unwrap_or(0)
}

/// Decode a 32-byte ABI-encoded address result (last 40 hex chars)
pub fn decode_address(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    }
}
