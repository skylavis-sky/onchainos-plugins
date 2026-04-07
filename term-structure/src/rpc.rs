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

/// Low-level eth_call. Returns the hex-encoded return data (0x-prefixed).
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

/// Build reqwest client respecting system proxy (HTTPS_PROXY env var).
pub fn build_client() -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Ok(url) = std::env::var("HTTPS_PROXY") {
        builder = builder.proxy(reqwest::Proxy::https(&url).context("invalid HTTPS_PROXY")?);
    }
    if let Ok(url) = std::env::var("HTTP_PROXY") {
        builder = builder.proxy(reqwest::Proxy::http(&url).context("invalid HTTP_PROXY")?);
    }
    builder.build().context("reqwest client build failed")
}

// ── ABI helpers ──────────────────────────────────────────────────────────────

pub fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

/// Encode a 20-byte address into a 32-byte ABI slot (left-padded).
pub fn encode_address(addr: &str) -> anyhow::Result<Vec<u8>> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address length: {}", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = vec![0u8; 12];
    out.extend_from_slice(&bytes);
    Ok(out)
}

/// Encode a u256 value (as u128, upper bits zero) into a 32-byte ABI slot.
pub fn encode_u256(val: u128) -> Vec<u8> {
    let mut out = vec![0u8; 16];
    out.extend_from_slice(&val.to_be_bytes());
    out
}

/// Decode an address from a 32-byte ABI slot (positions 12..32).
pub fn decode_address_at(raw: &str, slot: usize) -> anyhow::Result<String> {
    let start = slot * 64;
    if raw.len() < start + 64 {
        anyhow::bail!("decode_address_at: slot {} out of range", slot);
    }
    let addr_hex = &raw[start + 24..start + 64]; // last 40 chars of 64-char slot
    Ok(format!("0x{}", addr_hex))
}

/// Decode a u128 (lower 16 bytes of a 32-byte ABI slot).
pub fn decode_u128_at(raw: &str, slot: usize) -> anyhow::Result<u128> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u128_at: slot {} out of range (raw len {})", slot, raw.len());
    }
    let slot_hex = &raw[start..end];
    let low32 = &slot_hex[32..64];
    u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))
}

/// Decode a u64 from the lower 8 bytes of a 32-byte ABI slot.
pub fn decode_u64_at(raw: &str, slot: usize) -> anyhow::Result<u64> {
    let start = slot * 64;
    let end = start + 64;
    if raw.len() < end {
        anyhow::bail!("decode_u64_at: slot {} out of range", slot);
    }
    let slot_hex = &raw[start..end];
    let low16 = &slot_hex[48..64]; // last 16 hex chars = 8 bytes
    u64::from_str_radix(low16, 16)
        .with_context(|| format!("decode_u64_at: invalid hex '{}'", low16))
}

// ── TermMaxMarket read helpers ────────────────────────────────────────────────

/// TermMaxMarket.config() selector: 0x79502c55
/// Returns: (address treasurer, uint64 maturity, FeeConfig feeConfig)
/// We only care about maturity (slot 0 lower 8 bytes for treasurer addr, slot 1 for maturity in packed struct).
/// The struct packs: treasurer (20 bytes) + maturity (8 bytes) in the first ABI word.
/// In practice ABI encoding expands: slot 0 = treasurer (address), slot 1 = maturity (uint64).
pub async fn market_config(market_addr: &str, rpc_url: &str) -> anyhow::Result<(String, u64)> {
    let data = "0x79502c55";
    let hex_result = eth_call(rpc_url, market_addr, data).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 128 {
        anyhow::bail!("market_config: short response {} chars", raw.len());
    }
    let treasurer = decode_address_at(raw, 0)?;
    let maturity = decode_u64_at(raw, 1)?;
    Ok((treasurer, maturity))
}

/// TermMaxMarket.tokens() selector: 0x9d63848a
/// Returns: (address ft, address xt, address gt, address collateral, address underlying)
pub async fn market_tokens(market_addr: &str, rpc_url: &str) -> anyhow::Result<MarketTokens> {
    let data = "0x9d63848a";
    let hex_result = eth_call(rpc_url, market_addr, data).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 320 {
        anyhow::bail!("market_tokens: short response {} chars", raw.len());
    }
    Ok(MarketTokens {
        ft: decode_address_at(raw, 0)?,
        xt: decode_address_at(raw, 1)?,
        gt: decode_address_at(raw, 2)?,
        collateral: decode_address_at(raw, 3)?,
        underlying: decode_address_at(raw, 4)?,
    })
}

#[derive(Debug, Clone)]
pub struct MarketTokens {
    pub ft: String,
    pub xt: String,
    pub gt: String,
    pub collateral: String,
    pub underlying: String,
}

/// TermMaxOrder.apr() selector: 0x57ded9c9
/// Returns: (uint256 lendApr, uint256 borrowApr) scaled 1e18
pub async fn order_apr(order_addr: &str, rpc_url: &str) -> anyhow::Result<(u128, u128)> {
    let data = "0x57ded9c9";
    let hex_result = eth_call(rpc_url, order_addr, data).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 128 {
        anyhow::bail!("order_apr: short response {} chars", raw.len());
    }
    let lend_apr = decode_u128_at(raw, 0)?;
    let borrow_apr = decode_u128_at(raw, 1)?;
    Ok((lend_apr, borrow_apr))
}

/// TermMaxOrder.tokenReserves() selector: 0x4bad9510
/// Returns: (uint256 ftReserve, uint256 xtReserve)
pub async fn order_reserves(order_addr: &str, rpc_url: &str) -> anyhow::Result<(u128, u128)> {
    let data = "0x4bad9510";
    let hex_result = eth_call(rpc_url, order_addr, data).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 128 {
        anyhow::bail!("order_reserves: short response {} chars", raw.len());
    }
    let ft_reserve = decode_u128_at(raw, 0)?;
    let xt_reserve = decode_u128_at(raw, 1)?;
    Ok((ft_reserve, xt_reserve))
}

/// ERC-20 balanceOf(address) selector: 0x70a08231
pub async fn erc20_balance(token_addr: &str, account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let mut calldata = hex::decode("70a08231")?;
    let addr_bytes = encode_address(account)?;
    calldata.extend_from_slice(&addr_bytes);
    let data_hex = format!("0x{}", hex::encode(&calldata));
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        return Ok(0);
    }
    decode_u128_at(raw, 0)
}

/// ERC-20 decimals() selector: 0x313ce567
pub async fn erc20_decimals(token_addr: &str, rpc_url: &str) -> anyhow::Result<u8> {
    let data = "0x313ce567";
    let hex_result = eth_call(rpc_url, token_addr, data).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        return Ok(18);
    }
    let val = decode_u128_at(raw, 0)?;
    Ok(val as u8)
}

/// ERC-721 balanceOf(address) selector: 0x70a08231 (same as ERC-20)
pub async fn erc721_balance(token_addr: &str, account: &str, rpc_url: &str) -> anyhow::Result<u128> {
    erc20_balance(token_addr, account, rpc_url).await
}

/// ERC-20 allowance(owner, spender) selector: 0xdd62ed3e
pub async fn erc20_allowance(
    token_addr: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut calldata = hex::decode("dd62ed3e")?;
    calldata.extend_from_slice(&encode_address(owner)?);
    calldata.extend_from_slice(&encode_address(spender)?);
    let data_hex = format!("0x{}", hex::encode(&calldata));
    let hex_result = eth_call(rpc_url, token_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);
    if raw.len() < 64 {
        return Ok(0);
    }
    decode_u128_at(raw, 0)
}

/// TermMaxViewer.getPositionDetail(address market, address owner) selector: 0x34c2cb2e
/// Returns a complex struct; we decode the basic fields:
/// Position { underlyingBalance, collateralBalance, ftBalance, xtBalance, ... }
pub async fn viewer_get_position(
    viewer_addr: &str,
    market_addr: &str,
    owner_addr: &str,
    rpc_url: &str,
) -> anyhow::Result<PositionDetail> {
    let mut calldata = hex::decode("34c2cb2e")?;
    calldata.extend_from_slice(&encode_address(market_addr)?);
    calldata.extend_from_slice(&encode_address(owner_addr)?);
    let data_hex = format!("0x{}", hex::encode(&calldata));

    let hex_result = eth_call(rpc_url, viewer_addr, &data_hex).await?;
    let raw = strip_0x(&hex_result);

    // The Position struct is ABI-encoded as a tuple, so the first word is an offset pointer.
    // slot[0] = 0x20 = 32 (tuple offset, NOT a data field — this is standard ABI tuple encoding)
    // Actual struct fields start at slot[1]:
    //   slot[1] = underlyingBalance
    //   slot[2] = collateralBalance
    //   slot[3] = ftBalance
    //   slot[4] = xtBalance
    //   slot[5] = offset to gtInfo[] dynamic array
    //   slot[6] = gtInfo[].length
    if raw.len() < 448 {
        // Return zero position if response too short (7 slots × 64 chars = 448)
        return Ok(PositionDetail::default());
    }

    let underlying_balance = decode_u128_at(raw, 1).unwrap_or(0);
    let collateral_balance = decode_u128_at(raw, 2).unwrap_or(0);
    let ft_balance = decode_u128_at(raw, 3).unwrap_or(0);
    let xt_balance = decode_u128_at(raw, 4).unwrap_or(0);

    Ok(PositionDetail {
        underlying_balance,
        collateral_balance,
        ft_balance,
        xt_balance,
    })
}

#[derive(Debug, Clone, Default)]
pub struct PositionDetail {
    pub underlying_balance: u128,
    pub collateral_balance: u128,
    pub ft_balance: u128,
    pub xt_balance: u128,
}

/// Poll eth_getTransactionReceipt until mined (or 60s timeout).
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

/// Convert apr scaled 1e18 to human-readable percent string.
pub fn apr_to_pct(apr_scaled: u128) -> f64 {
    apr_scaled as f64 / 1e18 * 100.0
}

/// Human-readable amount to minimal units.
pub fn human_to_minimal(amount: f64, decimals: u8) -> u128 {
    let factor = 10u128.pow(decimals as u32);
    (amount * factor as f64) as u128
}

/// Format Unix timestamp as a date string.
pub fn ts_to_date(ts: u64) -> String {
    // Simple conversion: days since epoch
    let days = ts / 86400;
    let year_approx = 1970 + days / 365;
    format!("~{} (ts={})", year_approx, ts)
}
