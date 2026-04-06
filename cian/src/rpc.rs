// rpc.rs — Direct eth_call utilities for CIAN vault on-chain reads

/// Perform a raw JSON-RPC eth_call
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
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

/// Decode the first ABI word as u128
pub fn decode_uint256_u128(hex: &str) -> u128 {
    let clean = hex.trim_start_matches("0x");
    let word0 = if clean.len() >= 64 { &clean[..64] } else { clean };
    let last32 = if word0.len() >= 32 { &word0[word0.len() - 32..] } else { word0 };
    u128::from_str_radix(last32, 16).unwrap_or(0)
}

/// Decode a 32-byte ABI-encoded address (last 40 hex chars)
pub fn decode_address(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        "0x0000000000000000000000000000000000000000".to_string()
    }
}

/// Pad a 20-byte address to 32 bytes (no 0x prefix)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.trim_start_matches("0x");
    format!("{:0>64}", clean)
}

/// totalAssets() → 0x01e1d114
pub async fn get_total_assets(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(vault, "0x01e1d114", rpc_url).await?;
    Ok(decode_uint256_u128(&hex))
}

/// totalSupply() → 0x18160ddd
pub async fn get_total_supply(vault: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex = eth_call(vault, "0x18160ddd", rpc_url).await?;
    Ok(decode_uint256_u128(&hex))
}

/// balanceOf(address) → 0x70a08231
pub async fn get_balance_of(vault: &str, owner: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let data = format!("0x70a08231{}", pad_address(owner));
    let hex = eth_call(vault, &data, rpc_url).await?;
    Ok(decode_uint256_u128(&hex))
}

/// convertToAssets(uint256) → 0x07a2d13a
pub async fn convert_to_assets(vault: &str, shares: u128, rpc_url: &str) -> anyhow::Result<u128> {
    let shares_hex = format!("{:064x}", shares);
    let data = format!("0x07a2d13a{}", shares_hex);
    let hex = eth_call(vault, &data, rpc_url).await?;
    Ok(decode_uint256_u128(&hex))
}

/// decimals() → 0x313ce567
pub async fn get_decimals(token: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    Ok(decode_uint256_u128(&hex) as u8)
}

/// asset() → 0x38d52e0f — underlying token address
pub async fn get_asset(vault: &str, rpc_url: &str) -> anyhow::Result<String> {
    let hex = eth_call(vault, "0x38d52e0f", rpc_url).await?;
    Ok(decode_address(&hex))
}
