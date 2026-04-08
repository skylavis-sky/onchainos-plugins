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

/// Perform a raw eth_call against the Ethereum JSON-RPC endpoint.
/// `to` and `data` are 0x-prefixed hex strings.
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let req = RpcRequest {
        jsonrpc: "2.0",
        method: "eth_call",
        params: json!([{ "to": to, "data": data }, "latest"]),
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

// ── Decoding helpers ──────────────────────────────────────────────────────────

pub fn strip_0x(s: &str) -> &str {
    s.strip_prefix("0x").unwrap_or(s)
}

/// Decode a 20-byte address from a 32-byte ABI word (last 40 hex chars).
pub fn decode_address(hex_result: &str) -> anyhow::Result<String> {
    let raw = strip_0x(hex_result);
    if raw.len() < 40 {
        anyhow::bail!("decode_address: short result '{}'", hex_result);
    }
    let addr_hex = &raw[raw.len() - 40..];
    Ok(format!("0x{}", addr_hex))
}

/// Decode a u128 value from the N-th 32-byte ABI word (0-indexed).
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
    // Take lower 16 bytes (32 hex chars) to fit u128
    let low32 = &slot_hex[32..64];
    u128::from_str_radix(low32, 16)
        .with_context(|| format!("decode_u128_at: invalid hex '{}'", low32))
}

/// Decode a boolean from the N-th 32-byte ABI word.
pub fn decode_bool_at(raw: &str, slot: usize) -> anyhow::Result<bool> {
    let v = decode_u128_at(raw, slot)?;
    Ok(v != 0)
}

/// Parse a 20-byte address string into [u8; 20].
pub fn parse_address_bytes(addr: &str) -> anyhow::Result<[u8; 20]> {
    let clean = strip_0x(addr);
    if clean.len() != 40 {
        anyhow::bail!("Invalid address (must be 40 hex chars): {}", addr);
    }
    let bytes = hex::decode(clean).context("Invalid hex address")?;
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes);
    Ok(out)
}

/// Encode a u256 value (u128 fits) into 32 zero-padded bytes.
pub fn encode_u256(value: u128) -> [u8; 32] {
    let mut buf = [0u8; 32];
    let bytes = value.to_be_bytes(); // 16 bytes
    buf[16..].copy_from_slice(&bytes);
    buf
}

/// Encode a u256 value from a u64.
pub fn encode_u256_u64(value: u64) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[24..].copy_from_slice(&value.to_be_bytes());
    buf
}

/// Encode a 20-byte address into a 32-byte ABI word (left-padded with zeros).
pub fn encode_address(addr: &str) -> anyhow::Result<[u8; 32]> {
    let bytes = parse_address_bytes(addr)?;
    let mut buf = [0u8; 32];
    buf[12..].copy_from_slice(&bytes);
    Ok(buf)
}

/// Encode a bool into a 32-byte ABI word.
pub fn encode_bool(val: bool) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[31] = if val { 1 } else { 0 };
    buf
}

// ── Contract-specific calls ───────────────────────────────────────────────────

/// ERC-20 balanceOf(address) → 0x70a08231
#[allow(dead_code)]
pub async fn erc20_balance_of(
    token: &str,
    owner: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("70a08231")?;
    data.extend_from_slice(&encode_address(owner)?);
    let hex_result = eth_call(rpc_url, token, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// PositionToken.getTokenIDsArray(address owner) → uint256[]
/// Selector: 0x9aeb5813
pub async fn get_token_ids_array(
    position_token: &str,
    owner: &str,
    rpc_url: &str,
) -> anyhow::Result<Vec<u128>> {
    let mut data = hex::decode("9aeb5813")?;
    data.extend_from_slice(&encode_address(owner)?);
    let hex_result = eth_call(rpc_url, position_token, &format!("0x{}", hex::encode(&data))).await?;
    let raw = strip_0x(&hex_result);

    // ABI-encoded dynamic array: offset (32 bytes) + length (32 bytes) + elements
    if raw.len() < 128 {
        // No positions
        return Ok(vec![]);
    }
    // offset is at slot 0
    // length of array is at the offset position
    let offset_hex = &raw[0..64];
    let offset = usize::from_str_radix(&offset_hex[56..64], 16).unwrap_or(32) * 2;
    if raw.len() < offset + 64 {
        return Ok(vec![]);
    }
    let count_hex = &raw[offset..offset + 64];
    let count = usize::from_str_radix(&count_hex[56..64], 16).unwrap_or(0);
    let mut ids = Vec::with_capacity(count);
    let data_start = offset + 64;
    for i in 0..count {
        let start = data_start + i * 64;
        if start + 64 > raw.len() {
            break;
        }
        let id = decode_u128_at(raw, (offset / 64) + 1 + i).unwrap_or(0);
        ids.push(id);
    }
    // Re-parse properly: array starts at offset/2 byte position
    // Let's redo with raw bytes offset
    let raw_bytes = hex::decode(raw).unwrap_or_default();
    if raw_bytes.len() < 64 {
        return Ok(ids);
    }
    // Read offset (first 32 bytes)
    let offset_bytes = u64::from_be_bytes(raw_bytes[24..32].try_into().unwrap_or([0u8; 8])) as usize;
    if raw_bytes.len() < offset_bytes + 32 {
        return Ok(ids);
    }
    // Read count
    let count_bytes = u64::from_be_bytes(
        raw_bytes[offset_bytes + 24..offset_bytes + 32]
            .try_into()
            .unwrap_or([0u8; 8]),
    ) as usize;
    let mut result = Vec::with_capacity(count_bytes);
    for i in 0..count_bytes {
        let pos = offset_bytes + 32 + i * 32;
        if pos + 16 > raw_bytes.len() {
            break;
        }
        let val = u128::from_be_bytes(raw_bytes[pos + 16..pos + 32].try_into().unwrap_or([0u8; 16]));
        result.push(val);
    }
    Ok(result)
}

/// CDPosition.getOUSDPrinciple(uint256 nftId) → uint256
/// Selector: 0x02c4e2eb
pub async fn get_ousd_principle(
    cd_position: &str,
    nft_id: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("02c4e2eb")?;
    data.extend_from_slice(&encode_u256(nft_id));
    let hex_result = eth_call(rpc_url, cd_position, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// CDPosition.getOUSDInterestEarned(uint256 nftId) → uint256
/// Selector: 0xffce8b9b
pub async fn get_ousd_interest_earned(
    cd_position: &str,
    nft_id: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("ffce8b9b")?;
    data.extend_from_slice(&encode_u256(nft_id));
    let hex_result = eth_call(rpc_url, cd_position, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// CDPosition.getOUSDTotalIncludeInterest(uint256 nftId) → uint256
/// Selector: 0xe8b61371
pub async fn get_ousd_total_include_interest(
    cd_position: &str,
    nft_id: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("e8b61371")?;
    data.extend_from_slice(&encode_u256(nft_id));
    let hex_result = eth_call(rpc_url, cd_position, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// CDPosition.getLvUSDBorrowed(uint256 nftId) → uint256
/// Selector: 0xb3344644
pub async fn get_lvusd_borrowed(
    cd_position: &str,
    nft_id: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("b3344644")?;
    data.extend_from_slice(&encode_u256(nft_id));
    let hex_result = eth_call(rpc_url, cd_position, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// CDPosition.getPositionExpireTime(uint256 nftId) → uint256
/// Selector: 0x0d01a60f
pub async fn get_position_expire_time(
    cd_position: &str,
    nft_id: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("0d01a60f")?;
    data.extend_from_slice(&encode_u256(nft_id));
    let hex_result = eth_call(rpc_url, cd_position, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// Coordinator.getAvailableLeverage() → uint256
/// Selector: 0x67ac631c
pub async fn get_available_leverage(coordinator: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex_result = eth_call(rpc_url, coordinator, "0x67ac631c").await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// ParameterStore.getArchToLevRatio() → uint256
/// Selector: 0x64a2411b
pub async fn get_arch_to_lev_ratio(param_store: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex_result = eth_call(rpc_url, param_store, "0x64a2411b").await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// ParameterStore.getMaxNumberOfCycles() → uint256
/// Selector: 0x167446b5
pub async fn get_max_number_of_cycles(param_store: &str, rpc_url: &str) -> anyhow::Result<u128> {
    let hex_result = eth_call(rpc_url, param_store, "0x167446b5").await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// ParameterStore.getMinPositionCollateral() → uint256
/// Selector: 0xdb41f54a
pub async fn get_min_position_collateral(
    param_store: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let hex_result = eth_call(rpc_url, param_store, "0xdb41f54a").await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// ParameterStore.calculateArchNeededForLeverage(uint256 leverageAmount) → uint256
/// Selector: 0x34f8e9e5
#[allow(dead_code)]
pub async fn calculate_arch_needed(
    param_store: &str,
    leverage_amount: u128,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let mut data = hex::decode("34f8e9e5")?;
    data.extend_from_slice(&encode_u256(leverage_amount));
    let hex_result = eth_call(rpc_url, param_store, &format!("0x{}", hex::encode(&data))).await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// ParameterStore.getOriginationFeeRate() → uint256
/// Selector: 0xf9ef30b5
pub async fn get_origination_fee_rate(
    param_store: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    let hex_result = eth_call(rpc_url, param_store, "0xf9ef30b5").await?;
    decode_u128_at(strip_0x(&hex_result), 0)
}

/// Zapper.previewZapInAmount(uint256,uint256,address,bool) → (uint256,uint256)
/// Selector: 0xe6e8f7ca
/// Returns (ousdAmount, archAmount)
pub async fn preview_zap_in_amount(
    zapper: &str,
    stable_coin_amount: u128,
    cycles: u64,
    address_base_stable: &str,
    use_user_arch: bool,
    rpc_url: &str,
) -> anyhow::Result<(u128, u128)> {
    let mut data = hex::decode("e6e8f7ca")?;
    data.extend_from_slice(&encode_u256(stable_coin_amount));
    data.extend_from_slice(&encode_u256_u64(cycles));
    data.extend_from_slice(&encode_address(address_base_stable)?);
    data.extend_from_slice(&encode_bool(use_user_arch));
    let hex_result =
        eth_call(rpc_url, zapper, &format!("0x{}", hex::encode(&data))).await?;
    let raw = strip_0x(&hex_result);
    let ousd_amount = decode_u128_at(raw, 0)?;
    let arch_amount = decode_u128_at(raw, 1)?;
    Ok((ousd_amount, arch_amount))
}

/// PositionToken.ownerOf(uint256 tokenId) → address
/// Selector: 0x6352211e
pub async fn owner_of(
    position_token: &str,
    nft_id: u128,
    rpc_url: &str,
) -> anyhow::Result<String> {
    let mut data = hex::decode("6352211e")?;
    data.extend_from_slice(&encode_u256(nft_id));
    let hex_result =
        eth_call(rpc_url, position_token, &format!("0x{}", hex::encode(&data))).await?;
    decode_address(&hex_result)
}

/// PositionToken.isApprovedForAll(address owner, address operator) → bool
/// Selector: 0xe985e9c5
pub async fn is_approved_for_all(
    position_token: &str,
    owner: &str,
    operator: &str,
    rpc_url: &str,
) -> anyhow::Result<bool> {
    let mut data = hex::decode("e985e9c5")?;
    data.extend_from_slice(&encode_address(owner)?);
    data.extend_from_slice(&encode_address(operator)?);
    let hex_result =
        eth_call(rpc_url, position_token, &format!("0x{}", hex::encode(&data))).await?;
    decode_bool_at(strip_0x(&hex_result), 0)
}
