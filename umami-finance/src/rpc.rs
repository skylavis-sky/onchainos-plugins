/// Direct eth_call helpers for reading Umami Finance vault state on Arbitrum.
/// All selectors verified via `cast sig` and live eth_call tests.

use anyhow::Result;
use serde_json::Value;

/// Send a raw eth_call to Arbitrum RPC
pub async fn eth_call(rpc: &str, to: &str, data: &str) -> Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client.post(rpc).json(&body).send().await?.json().await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a 32-byte hex result as u128
pub fn decode_u128(hex: &str) -> u128 {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 64 {
        return 0;
    }
    u128::from_str_radix(&trimmed[trimmed.len().saturating_sub(32)..], 16).unwrap_or(0)
}

/// Decode a 32-byte hex result as address (last 20 bytes)
pub fn decode_address(hex: &str) -> String {
    let trimmed = hex.trim_start_matches("0x");
    if trimmed.len() < 40 {
        return "0x".to_string();
    }
    format!("0x{}", &trimmed[trimmed.len() - 40..])
}

/// ERC-4626: totalAssets() → 0x01e1d114
pub async fn total_assets(rpc: &str, vault: &str) -> Result<u128> {
    let result = eth_call(rpc, vault, "0x01e1d114").await?;
    Ok(decode_u128(&result))
}

/// ERC-4626: totalSupply() → 0x18160ddd
pub async fn total_supply(rpc: &str, vault: &str) -> Result<u128> {
    let result = eth_call(rpc, vault, "0x18160ddd").await?;
    Ok(decode_u128(&result))
}

/// ERC-4626: convertToAssets(uint256 shares) → 0x07a2d13a
/// Returns assets corresponding to given shares (price per share when shares = 10^decimals)
pub async fn convert_to_assets(rpc: &str, vault: &str, shares: u128) -> Result<u128> {
    let data = format!("0x07a2d13a{:064x}", shares);
    let result = eth_call(rpc, vault, &data).await?;
    Ok(decode_u128(&result))
}

/// ERC-4626: previewDeposit(uint256 assets) → 0xef8b30f7
pub async fn preview_deposit(rpc: &str, vault: &str, assets: u128) -> Result<u128> {
    let data = format!("0xef8b30f7{:064x}", assets);
    let result = eth_call(rpc, vault, &data).await?;
    Ok(decode_u128(&result))
}

/// ERC-4626: previewRedeem(uint256 shares) → 0x4cdad506
pub async fn preview_redeem(rpc: &str, vault: &str, shares: u128) -> Result<u128> {
    let data = format!("0x4cdad506{:064x}", shares);
    let result = eth_call(rpc, vault, &data).await?;
    Ok(decode_u128(&result))
}

/// ERC-4626: maxDeposit(address) → 0x402d267d
pub async fn max_deposit(rpc: &str, vault: &str, user: &str) -> Result<u128> {
    let addr = user.trim_start_matches("0x");
    let data = format!("0x402d267d{:0>64}", addr);
    let result = eth_call(rpc, vault, &data).await?;
    Ok(decode_u128(&result))
}

/// ERC-20: balanceOf(address) → 0x70a08231
pub async fn balance_of(rpc: &str, token: &str, user: &str) -> Result<u128> {
    let addr = user.trim_start_matches("0x");
    let data = format!("0x70a08231{:0>64}", addr);
    let result = eth_call(rpc, token, &data).await?;
    Ok(decode_u128(&result))
}

/// ERC-20: allowance(address owner, address spender) → 0xdd62ed3e
pub async fn allowance(rpc: &str, token: &str, owner: &str, spender: &str) -> Result<u128> {
    let owner_hex = owner.trim_start_matches("0x");
    let spender_hex = spender.trim_start_matches("0x");
    let data = format!("0xdd62ed3e{:0>64}{:0>64}", owner_hex, spender_hex);
    let result = eth_call(rpc, token, &data).await?;
    Ok(decode_u128(&result))
}
