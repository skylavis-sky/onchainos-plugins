#![allow(dead_code)]

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize)]
struct RpcRequest<'a> {
    jsonrpc: &'a str,
    method: &'a str,
    params: Value,
    id: u64,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<String>,
    error: Option<Value>,
}

/// Perform a raw eth_call against the given RPC endpoint.
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = build_client()?;
    let req = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_call",
        params: json!([
            { "to": to, "data": data },
            "latest"
        ]),
        id: 1,
    };
    let resp: RpcResponse = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call response parse failed")?;

    if let Some(err) = resp.error {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    resp.result
        .ok_or_else(|| anyhow::anyhow!("eth_call returned null result"))
}

/// Build reqwest client, respecting HTTPS_PROXY / HTTP_PROXY environment variables.
fn build_client() -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("HTTP_PROXY")) {
        if !proxy_url.is_empty() {
            builder = builder.proxy(
                reqwest::Proxy::all(&proxy_url)
                    .context("Failed to build proxy from HTTPS_PROXY")?,
            );
        }
    }
    builder.build().context("Failed to build reqwest client")
}

/// Poll eth_getTransactionReceipt until the tx is mined (or timeout).
/// Returns true if the tx succeeded (status=0x1), false if reverted, error if timed out.
pub async fn wait_for_tx(rpc_url: &str, tx_hash: &str) -> anyhow::Result<bool> {
    use std::time::{Duration, Instant};
    let client = build_client()?;
    let deadline = Instant::now() + Duration::from_secs(60);

    loop {
        if Instant::now() > deadline {
            anyhow::bail!("Timeout waiting for tx {} to be mined", tx_hash);
        }

        let req = json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });

        if let Ok(resp) = client.post(rpc_url).json(&req).send().await {
            if let Ok(body) = resp.json::<Value>().await {
                let receipt = &body["result"];
                if !receipt.is_null() {
                    let status = receipt["status"].as_str().unwrap_or("0x1");
                    return Ok(status == "0x1");
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

// ── ABI helpers ──────────────────────────────────────────────────────────────

pub fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

pub fn parse_address(addr: &str) -> anyhow::Result<[u8; 20]> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address (must be 20 bytes / 40 hex chars): {}", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Decode a uint128 at the given 32-byte slot index from raw hex (no 0x prefix).
pub fn decode_u128_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!(
            "decode_u128_at: slot {} out of range (raw len {})",
            slot,
            raw.len()
        );
    }
    let slot_hex = &raw[start..end];
    // u256 may exceed u128 — take lower 32 hex chars (16 bytes)
    let low32 = &slot_hex[32..64];
    u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))
}

/// Decode a uint256 at the given slot as u128 (saturating at u128::MAX).
#[allow(dead_code)]
pub fn decode_u256_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    decode_u128_at(raw, slot)
}

/// Decode an address at the given 32-byte slot index (last 20 bytes of 32).
pub fn decode_address_at(raw: &str, slot: usize) -> anyhow::Result<String> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!(
            "decode_address_at: slot {} out of range (raw len {})",
            slot,
            raw.len()
        );
    }
    let slot_hex = &raw[start..end];
    let addr_hex = &slot_hex[24..64]; // last 40 chars = 20 bytes
    Ok(format!("0x{}", addr_hex))
}

/// Encode: 4-byte selector + address (32 bytes, left-padded)
pub fn encode_selector_address(selector_hex: &str, addr: &str) -> anyhow::Result<String> {
    let addr_bytes = parse_address(addr)?;
    let mut data = hex::decode(selector_hex).context("Invalid selector hex")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Encode: 4-byte selector + uint256 (32 bytes)
pub fn encode_selector_uint256(selector_hex: &str, val: u128) -> anyhow::Result<String> {
    let mut data = hex::decode(selector_hex).context("Invalid selector hex")?;
    let val_hex = format!("{:064x}", val);
    let val_bytes = hex::decode(&val_hex)?;
    data.extend_from_slice(&val_bytes);
    Ok(format!("0x{}", hex::encode(&data)))
}

/// Decode a dynamic byte string from ABI encoding.
/// Used for ERC-20 symbol / name which may use bytes32 or dynamic string.
pub fn decode_string_result(hex_result: &str) -> String {
    let raw = strip_0x(hex_result);
    if raw.len() < 64 {
        return "UNKNOWN".to_string();
    }
    // Try fixed bytes32 first (common for old tokens like MKR, SNX)
    // If it starts with non-zero and the second word has data length, it's dynamic
    let first_word = &raw[0..64];
    // Check if this is an offset pointer (0x0000...0020 = dynamic string)
    if first_word.ends_with("20") || first_word == "0000000000000000000000000000000000000000000000000000000000000020" {
        // Dynamic string: offset=32, length=next word, data follows
        if raw.len() < 128 {
            return "UNKNOWN".to_string();
        }
        let len_hex = &raw[64..128];
        let len = usize::from_str_radix(len_hex.trim_start_matches('0'), 16).unwrap_or(0);
        if len == 0 || raw.len() < 128 + len * 2 {
            return "UNKNOWN".to_string();
        }
        let data_hex = &raw[128..128 + len * 2];
        if let Ok(bytes) = hex::decode(data_hex) {
            return String::from_utf8_lossy(&bytes).trim_end_matches('\0').to_string();
        }
    }
    // Fixed bytes32 — interpret as string
    if let Ok(bytes) = hex::decode(first_word) {
        let s = String::from_utf8_lossy(&bytes)
            .trim_end_matches('\0')
            .to_string();
        if !s.is_empty() {
            return s;
        }
    }
    "UNKNOWN".to_string()
}

/// Get ERC-20 token symbol
pub async fn erc20_symbol(token_addr: &str, rpc_url: &str) -> anyhow::Result<String> {
    // symbol() selector: 0x95d89b41
    let hex_result = eth_call(rpc_url, token_addr, "0x95d89b41").await?;
    Ok(decode_string_result(&hex_result))
}

/// Get ERC-20 token decimals
pub async fn erc20_decimals(token_addr: &str, rpc_url: &str) -> anyhow::Result<u8> {
    // decimals() selector: 0x313ce567
    let hex_result = eth_call(rpc_url, token_addr, "0x313ce567").await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        return Ok(18);
    }
    let val = u8::from_str_radix(&raw[62..64], 16).unwrap_or(18);
    Ok(val)
}

/// Get ERC-20 balance: balanceOf(address)
/// selector: 0x70a08231
pub async fn erc20_balance(token_addr: &str, account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let addr_bytes = parse_address(account)?;
    let mut data = hex::decode("70a08231")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);
    let hex_result = eth_call(rpc_url, token_addr, &format!("0x{}", hex::encode(&data))).await?;
    let raw = strip_0x(&hex_result);
    decode_u128_at(raw, 0)
}

/// Decode an ABI-encoded dynamic array of uint256 values.
/// Returns up to `max` values starting from offset.
pub fn decode_uint256_array(hex_result: &str, max: usize) -> Vec<u128> {
    let raw = strip_0x(hex_result);
    if raw.len() < 128 {
        return vec![];
    }
    // Slot 0: offset pointer (0x20)
    // Slot 1: array length
    let len_hex = &raw[64..128];
    let len = usize::from_str_radix(len_hex.trim_start_matches('0'), 16).unwrap_or(0);
    let actual_len = len.min(max);
    let mut values = Vec::with_capacity(actual_len);
    let data_start = 128;
    for i in 0..actual_len {
        let slot_start = data_start + i * 64;
        if raw.len() < slot_start + 64 {
            break;
        }
        let val = decode_u128_at(raw, 4 + i).unwrap_or(0); // slot offset = 4 (after 2-word header at start)
        values.push(val);
    }
    // Re-read properly
    let mut out = Vec::with_capacity(actual_len);
    for i in 0..actual_len {
        let slot_start = data_start + i * 64;
        if raw.len() < slot_start + 64 {
            break;
        }
        let word = &raw[slot_start..slot_start + 64];
        let low = &word[32..64];
        let val = u128::from_str_radix(low, 16).unwrap_or(0);
        out.push(val);
    }
    out
}
