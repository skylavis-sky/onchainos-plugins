use anyhow::Context;
use serde_json::Value;

/// Perform a raw eth_call against the given RPC endpoint.
/// `to` and `data` are hex strings (0x-prefixed).
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .context("eth_call HTTP request failed")?
        .json()
        .await
        .context("eth_call response parse failed")?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }
    let result = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call returned null result"))?
        .to_string();
    Ok(result)
}

/// Poll eth_getTransactionReceipt until mined (or timeout 90s).
pub async fn wait_for_tx(rpc_url: &str, tx_hash: &str) -> anyhow::Result<bool> {
    use std::time::{Duration, Instant};
    let client = reqwest::Client::new();
    let deadline = Instant::now() + Duration::from_secs(90);

    loop {
        if Instant::now() > deadline {
            anyhow::bail!("Timeout waiting for tx {} to be mined", tx_hash);
        }

        let req = serde_json::json!({
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

// ── ABI decode helpers ───────────────────────────────────────────────────────

pub fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

/// Decode a 32-byte slot as u128 (takes lower 16 bytes to avoid overflow).
pub fn decode_u128_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u128_at: slot {} out of range (len {})", slot, raw.len());
    }
    let low32 = &raw[start + 32..end];
    u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))
}

/// Decode a 32-byte slot as u256 stored in two u128 (hi, lo).
/// For large RAY values, we compute via f64 instead.
#[allow(dead_code)]
pub fn decode_u256_as_f64_at(raw: &str, slot: usize) -> anyhow::Result<f64> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        return Ok(0.0);
    }
    let hi_hex = &raw[start..start + 32];
    let lo_hex = &raw[start + 32..end];
    let hi = u128::from_str_radix(hi_hex, 16).unwrap_or(0);
    let lo = u128::from_str_radix(lo_hex, 16).unwrap_or(0);
    // val = hi * 2^64 + lo (treated as approximate f64)
    let val = (hi as f64) * (2u128.pow(64) as f64) + (lo as f64);
    Ok(val)
}

/// Decode an address from a 32-byte slot (last 20 bytes).
#[allow(dead_code)]
pub fn decode_address_at(raw: &str, slot: usize) -> anyhow::Result<String> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_address_at: slot {} out of range", slot);
    }
    let addr_hex = &raw[end - 40..end];
    Ok(format!("0x{}", addr_hex))
}

/// Parse a 20-byte address string to bytes.
pub fn parse_address(addr: &str) -> anyhow::Result<[u8; 20]> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address (expected 40 hex chars): {}", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

// ── Ion Protocol specific calls ──────────────────────────────────────────────

/// IonPool.getCurrentBorrowRate(uint8 ilkIndex) -> (uint256 borrowRate, uint256 reserveFactor)
/// selector: 0x6908d3df
/// borrowRate is per-second in RAY (1e27)
pub async fn get_current_borrow_rate(ion_pool: &str, ilk_index: u8) -> anyhow::Result<(u128, u128)> {
    let mut data = hex::decode("6908d3df")?;
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    let data_hex = format!("0x{}", hex::encode(&data));
    let result = eth_call(crate::config::RPC_URL, ion_pool, &data_hex).await?;
    let raw = strip_0x(&result);
    if raw.len() < 128 {
        anyhow::bail!("getCurrentBorrowRate: short response");
    }
    let borrow_rate = decode_u128_at(raw, 0)?;
    let reserve_factor = decode_u128_at(raw, 1)?;
    Ok((borrow_rate, reserve_factor))
}

/// IonPool.rate(uint8 ilkIndex) -> uint256 (RAY = 1e27)
/// selector: 0x3c04b547
pub async fn get_rate(ion_pool: &str, ilk_index: u8) -> anyhow::Result<u128> {
    let mut data = hex::decode("3c04b547")?;
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    let data_hex = format!("0x{}", hex::encode(&data));
    let result = eth_call(crate::config::RPC_URL, ion_pool, &data_hex).await?;
    let raw = strip_0x(&result);
    if raw.len() < 64 {
        anyhow::bail!("rate: short response");
    }
    decode_u128_at(raw, 0)
}

/// IonPool.totalSupply() -> uint256 (WAD)
/// selector: 0x18160ddd
pub async fn get_total_supply(ion_pool: &str) -> anyhow::Result<u128> {
    let data_hex = "0x18160ddd";
    let result = eth_call(crate::config::RPC_URL, ion_pool, data_hex).await?;
    let raw = strip_0x(&result);
    if raw.len() < 64 {
        return Ok(0);
    }
    decode_u128_at(raw, 0)
}

/// IonPool.vault(uint8 ilkIndex, address user) -> (uint256 collateral, uint256 normalizedDebt)
/// selector: 0x9a3db79b
pub async fn get_vault(ion_pool: &str, ilk_index: u8, user: &str) -> anyhow::Result<(u128, u128)> {
    let addr_bytes = parse_address(user)?;
    let mut data = hex::decode("9a3db79b")?;
    // ilkIndex as uint8 padded to 32 bytes
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    // user address padded to 32 bytes
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);
    let data_hex = format!("0x{}", hex::encode(&data));
    let result = eth_call(crate::config::RPC_URL, ion_pool, &data_hex).await?;
    let raw = strip_0x(&result);
    if raw.len() < 128 {
        // No vault = zero position
        return Ok((0, 0));
    }
    let collateral = decode_u128_at(raw, 0)?;
    let normalized_debt = decode_u128_at(raw, 1)?;
    Ok((collateral, normalized_debt))
}

/// IonPool.normalizedDebt(uint8 ilkIndex, address user) -> uint256
/// selector: 0x57fc90b2
pub async fn get_normalized_debt(ion_pool: &str, ilk_index: u8, user: &str) -> anyhow::Result<u128> {
    let addr_bytes = parse_address(user)?;
    let mut data = hex::decode("57fc90b2")?;
    data.extend_from_slice(&[0u8; 31]);
    data.push(ilk_index);
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);
    let data_hex = format!("0x{}", hex::encode(&data));
    let result = eth_call(crate::config::RPC_URL, ion_pool, &data_hex).await?;
    let raw = strip_0x(&result);
    if raw.len() < 64 {
        return Ok(0);
    }
    decode_u128_at(raw, 0)
}

/// IonPool.balanceOf(address) -> uint256 (WAD) — lender supply token balance
/// selector: 0x70a08231
pub async fn get_ion_balance(ion_pool: &str, user: &str) -> anyhow::Result<u128> {
    let addr_bytes = parse_address(user)?;
    let mut data = hex::decode("70a08231")?;
    data.extend_from_slice(&[0u8; 12]);
    data.extend_from_slice(&addr_bytes);
    let data_hex = format!("0x{}", hex::encode(&data));
    let result = eth_call(crate::config::RPC_URL, ion_pool, &data_hex).await?;
    let raw = strip_0x(&result);
    if raw.len() < 64 {
        return Ok(0);
    }
    decode_u128_at(raw, 0)
}

/// Convert per-second RAY borrow rate to approximate annual APY percentage.
///
/// Ion Protocol's getCurrentBorrowRate returns a per-second compounding rate in RAY format:
///   - A rate of exactly RAY (1e27) means 0% APY
///   - A rate of RAY + x means x/RAY per-second excess rate
///
/// APY = ((rate / RAY) ^ SECONDS_PER_YEAR - 1) * 100
/// Using log approximation to avoid overflow: APY ≈ (rate/RAY - 1) * SECONDS_PER_YEAR * 100
pub fn borrow_rate_to_apy_pct(rate_per_sec: u128) -> f64 {
    const SECONDS_PER_YEAR: f64 = 31_536_000.0;
    let ray: f64 = 1e27;
    let rate_f64 = rate_per_sec as f64;

    if rate_f64 <= ray {
        // 0% or negative — clamp to 0
        return 0.0;
    }

    // per-second excess = (rate - RAY) / RAY
    let per_sec_excess = (rate_f64 - ray) / ray;

    // Linear approximation (good for <200% APY): APY ≈ per_sec_excess * SECONDS_PER_YEAR
    // For accuracy we use the log/exp: APY = exp(per_sec_excess * SECONDS_PER_YEAR) - 1
    // but linear is fine for display
    let apy = per_sec_excess * SECONDS_PER_YEAR * 100.0;
    apy
}

/// Compute normalizedAmount from actual amount and current rate.
/// normalizedAmount = actualAmount * RAY / rate
/// Uses u128 arithmetic with overflow protection via f64 fallback.
pub fn to_normalized(actual_wad: u128, rate: u128) -> u128 {
    if rate == 0 {
        return actual_wad;
    }
    // Use 256-bit-style arithmetic via u128 with careful ordering
    // normalizedDebt = actual * RAY / rate
    // RAY = 1e27, actual could be up to ~1e22 wad (1M ETH)
    // actual * RAY could overflow u128 (max ~3.4e38)
    // Safe approach: use f64 for large values, u128 for small
    let ray = crate::config::RAY as u128;
    if actual_wad < 1_000_000_000 {
        // Small amounts: u128 is safe
        (actual_wad * ray) / rate
    } else {
        // Large amounts: use f64 approximation
        let normalized_f64 = (actual_wad as f64) * (ray as f64) / (rate as f64);
        normalized_f64 as u128
    }
}
