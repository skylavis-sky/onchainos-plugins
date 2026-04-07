use anyhow::Context;
use serde_json::json;

/// Perform a raw eth_call against the given RPC endpoint.
/// `to` and `data` are hex strings (0x-prefixed).
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let req = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": to, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp: serde_json::Value = client
        .post(rpc_url)
        .json(&req)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json::<serde_json::Value>()
        .await
        .context("eth_call response parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    resp["result"]
        .as_str()
        .map(|s: &str| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("eth_call returned null result"))
}

/// Poll eth_getTransactionReceipt until the tx is mined (or timeout).
pub async fn wait_for_tx(rpc_url: &str, tx_hash: &str) -> anyhow::Result<bool> {
    use std::time::{Duration, Instant};
    let client = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(90);

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

        if let Ok(http_resp) = client.post(rpc_url).json(&req).send().await {
            if let Ok(body) = http_resp.json::<serde_json::Value>().await {
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

#[allow(dead_code)]
/// Get ERC-20 token balance: token.balanceOf(account)
/// Selector: 0x70a08231
pub async fn get_erc20_balance(
    token_addr: &str,
    account: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner = parse_address(account)?;
    let mut data = hex::decode("70a08231")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&owner);
    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        anyhow::bail!("balanceOf: short response");
    }
    decode_u128_at(raw, 0)
}

#[allow(dead_code)]
/// Get ERC-20 allowance: token.allowance(owner, spender)
/// Selector: 0xdd62ed3e
pub async fn get_allowance(
    token_addr: &str,
    owner_addr: &str,
    spender_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let owner = parse_address(owner_addr)?;
    let spender = parse_address(spender_addr)?;
    let mut data = hex::decode("dd62ed3e")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&owner);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&spender);
    let data_hex = format!("0x{}", hex::encode(&data));
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        anyhow::bail!("allowance: short response");
    }
    decode_u128_at(raw, 0)
}

/// Call CreditFacadeV3.debtLimits() → (uint128 minDebt, uint128 maxDebt)
/// Selector: 0x166bf9d9
pub async fn get_debt_limits(
    facade_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<(u128, u128)> {
    let hex_result = eth_call(rpc_url, facade_addr, "0x166bf9d9").await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 128 {
        anyhow::bail!("debtLimits: short response ({} chars)", raw.len());
    }
    let min_debt = decode_u128_at(raw, 0)?;
    let max_debt = decode_u128_at(raw, 1)?;
    Ok((min_debt, max_debt))
}

// ── helpers ─────────────────────────────────────────────────────────────────

pub fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

pub fn parse_address(addr: &str) -> anyhow::Result<[u8; 20]> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address length for '{}': need 40 hex chars", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

pub fn decode_u128_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u128_at: slot {} out of range (raw len {})", slot, raw.len());
    }
    let slot_hex = &raw[start..end];
    // Take lower 32 hex chars (u128 = 16 bytes)
    let low32 = &slot_hex[32..64];
    let val = u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))?;
    Ok(val)
}

#[allow(dead_code)]
pub fn decode_u256_at_raw(raw: &str, slot: usize) -> anyhow::Result<String> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u256_at_raw: slot {} out of range", slot);
    }
    Ok(raw[start..end].to_string())
}

#[allow(dead_code)]
pub fn decode_address_at(raw: &str, slot: usize) -> anyhow::Result<String> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_address_at: slot {} out of range", slot);
    }
    let slot_hex = &raw[start..end];
    // Address is last 40 chars of 64-char slot
    let addr_hex = &slot_hex[24..64];
    Ok(format!("0x{}", addr_hex))
}
