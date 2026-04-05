// Direct eth_call helpers (no onchainos needed for reads)
// RPC: https://ethereum.publicnode.com

use anyhow::Result;
use serde_json::{json, Value};

/// Low-level eth_call
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client
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

/// Decode a uint256 from a 0x-prefixed 32-byte hex result
pub fn decode_uint256(hex: &str) -> u128 {
    let h = hex.trim_start_matches("0x");
    if h.is_empty() || h == "0" {
        return 0;
    }
    // Take last 32 bytes (64 hex chars) to be safe
    let trimmed = if h.len() > 32 {
        &h[h.len() - 32..]
    } else {
        h
    };
    u128::from_str_radix(trimmed, 16).unwrap_or(0)
}

/// balanceOf(address) → 0x70a08231
pub async fn balance_of(rpc_url: &str, token: &str, owner: &str) -> Result<u128> {
    let owner_padded = format!("{:0>64}", &owner.trim_start_matches("0x").to_lowercase());
    let data = format!("0x70a08231{}", owner_padded);
    let result = eth_call(rpc_url, token, &data).await?;
    Ok(decode_uint256(&result))
}

/// totalAssets() → 0x01e1d114
pub async fn total_assets(rpc_url: &str, pool: &str) -> Result<u128> {
    let result = eth_call(rpc_url, pool, "0x01e1d114").await?;
    Ok(decode_uint256(&result))
}

/// totalSupply() → 0x18160ddd
pub async fn total_supply(rpc_url: &str, pool: &str) -> Result<u128> {
    let result = eth_call(rpc_url, pool, "0x18160ddd").await?;
    Ok(decode_uint256(&result))
}

/// convertToExitAssets(uint256 shares) → 0x50496cbd
/// Returns asset value accounting for unrealized losses
pub async fn convert_to_exit_assets(rpc_url: &str, pool: &str, shares: u128) -> Result<u128> {
    let shares_hex = format!("{:064x}", shares);
    let data = format!("0x50496cbd{}", shares_hex);
    let result = eth_call(rpc_url, pool, &data).await?;
    Ok(decode_uint256(&result))
}

/// allowance(address owner, address spender) → 0xdd62ed3e
pub async fn allowance(rpc_url: &str, token: &str, owner: &str, spender: &str) -> Result<u128> {
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x").to_lowercase());
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x").to_lowercase());
    let data = format!("0xdd62ed3e{}{}", owner_padded, spender_padded);
    let result = eth_call(rpc_url, token, &data).await?;
    Ok(decode_uint256(&result))
}

/// Convert raw token amount to human-readable (divides by 10^decimals)
pub fn format_amount(raw: u128, decimals: u32) -> f64 {
    raw as f64 / 10f64.powi(decimals as i32)
}
