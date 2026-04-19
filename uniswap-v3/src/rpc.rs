/// RPC helpers: eth_call, QuoterV2, factory pool queries, token metadata.

use anyhow::Result;
use serde_json::json;

// ── Client builder ─────────────────────────────────────────────────────────────

/// Build a reqwest client that respects HTTPS_PROXY environment variable.
pub fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::https(&url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

// ── Raw eth_call ──────────────────────────────────────────────────────────────

/// Execute an eth_call and return the hex result string.
pub async fn eth_call(to: &str, data: &str, rpc_url: &str) -> Result<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": data }, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = build_client()
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

/// Execute an eth_call with an explicit gas limit (needed for QuoterV2 simulation).
pub async fn eth_call_with_gas(to: &str, data: &str, rpc_url: &str, gas: &str) -> Result<String> {
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": data, "gas": gas }, "latest"],
        "id": 1
    });
    let resp: serde_json::Value = build_client()
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

// ── Hex decode helpers ────────────────────────────────────────────────────────

pub fn decode_u256_from_hex(hex: &str) -> u128 {
    let trimmed = hex.trim_start_matches("0x");
    let s = if trimmed.len() > 32 {
        &trimmed[trimmed.len() - 32..]
    } else {
        trimmed
    };
    u128::from_str_radix(s, 16).unwrap_or(0)
}

pub fn decode_address_from_hex(hex: &str) -> String {
    let raw = hex.trim_start_matches("0x");
    if raw.len() >= 40 {
        format!("0x{}", &raw[raw.len() - 40..])
    } else {
        format!("0x{:0>40}", raw)
    }
}

/// Decode ABI int256 tick value (64 hex chars) into i32.
/// Uses the last 8 hex chars (32 bits) reinterpreted as u32 → i32.
pub fn decode_tick(hex_word: &str) -> i32 {
    let clean = hex_word.trim_start_matches("0x");
    let last8 = if clean.len() >= 8 {
        &clean[clean.len() - 8..]
    } else {
        clean
    };
    u32::from_str_radix(last8, 16).unwrap_or(0) as i32
}

// ── Token metadata ────────────────────────────────────────────────────────────

/// Get ERC-20 decimals via eth_call.
/// decimals() selector = 0x313ce567
pub async fn get_decimals(token: &str, rpc_url: &str) -> Result<u8> {
    let hex = eth_call(token, "0x313ce567", rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    Ok(u8::from_str_radix(&raw[raw.len().saturating_sub(2)..], 16).unwrap_or(18))
}

/// Get ERC-20 symbol via eth_call.
/// symbol() selector = 0x95d89b41
pub async fn get_symbol(token: &str, rpc_url: &str) -> Result<String> {
    let hex = eth_call(token, "0x95d89b41", rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 {
        return Ok(format!("0x{}", &token[2..6]));
    }
    let len_hex = &raw[64..128];
    let len = usize::from_str_radix(len_hex, 16).unwrap_or(0);
    if 128 + len * 2 > raw.len() {
        return Ok(format!("0x{}", &token[2..6]));
    }
    let data_hex = &raw[128..128 + len * 2];
    let bytes = hex::decode(data_hex).unwrap_or_default();
    Ok(String::from_utf8_lossy(&bytes).to_string())
}

/// Get ERC-20 allowance via eth_call.
/// allowance(address,address) selector = 0xdd62ed3e
pub async fn get_allowance(
    token: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> Result<u128> {
    let padded_owner = format!("{:0>64}", &owner.trim_start_matches("0x"));
    let padded_spender = format!("{:0>64}", &spender.trim_start_matches("0x"));
    let hex = eth_call(
        token,
        &format!("0xdd62ed3e{}{}", padded_owner, padded_spender),
        rpc_url,
    )
    .await?;
    Ok(decode_u256_from_hex(&hex))
}

/// Get ERC-20 balance via eth_call.
/// balanceOf(address) selector = 0x70a08231
pub async fn get_balance(token: &str, account: &str, rpc_url: &str) -> Result<u128> {
    let padded = format!("{:0>64}", &account.trim_start_matches("0x"));
    let hex = eth_call(token, &format!("0x70a08231{}", padded), rpc_url).await?;
    Ok(decode_u256_from_hex(&hex))
}

// ── UniswapV3Factory ──────────────────────────────────────────────────────────

/// Get pool address from UniswapV3Factory.
/// getPool(address,address,uint24) selector = 0x1698ee82
/// Returns None if pool is address(0) — i.e., not deployed.
pub async fn get_pool_address(
    factory: &str,
    token_a: &str,
    token_b: &str,
    fee: u32,
    rpc_url: &str,
) -> Result<Option<String>> {
    let addr_a = token_a.trim_start_matches("0x");
    let addr_b = token_b.trim_start_matches("0x");
    let calldata = format!(
        "0x1698ee82{:0>64}{:0>64}{:0>64x}",
        addr_a, addr_b, fee
    );
    let result = eth_call(factory, &calldata, rpc_url).await?;
    let pool = decode_address_from_hex(&result);
    if pool == "0x0000000000000000000000000000000000000000" {
        return Ok(None);
    }
    Ok(Some(pool))
}

// ── Pool slot0 ────────────────────────────────────────────────────────────────

/// Query slot0() from pool contract.
/// slot0() selector = 0x3850c7bd
/// Returns (sqrtPriceX96, currentTick)
pub async fn get_slot0(pool: &str, rpc_url: &str) -> Result<(u128, i32)> {
    let hex = eth_call(pool, "0x3850c7bd", rpc_url).await?;
    let raw = hex.trim_start_matches("0x");
    if raw.len() < 128 {
        anyhow::bail!("Invalid slot0 response from pool {}", pool);
    }
    let sqrt_price_hex = &raw[0..64];
    let tick_hex = &raw[64..128];

    let sqrt_price = u128::from_str_radix(sqrt_price_hex, 16).unwrap_or(0);
    let tick = decode_tick(tick_hex);

    Ok((sqrt_price, tick))
}

/// Query liquidity() from pool contract.
/// liquidity() selector = 0x1a686502
pub async fn get_pool_liquidity(pool: &str, rpc_url: &str) -> Result<u128> {
    let hex = eth_call(pool, "0x1a686502", rpc_url).await?;
    Ok(decode_u256_from_hex(&hex))
}

// ── QuoterV2 ──────────────────────────────────────────────────────────────────

/// Quote exact input single via QuoterV2 eth_call.
/// quoteExactInputSingle((address,address,uint256,uint24,uint160)) selector = 0xc6a5026a
/// Uses ~5M gas limit to avoid false out-of-gas from the simulation.
pub async fn quote_exact_input_single(
    quoter: &str,
    token_in: &str,
    token_out: &str,
    amount_in: u128,
    fee: u32,
    rpc_url: &str,
) -> Result<u128> {
    use crate::calldata::encode_quote_exact_input_single;
    let calldata = encode_quote_exact_input_single(token_in, token_out, amount_in, fee)?;
    // Use 0x4C4B40 (~5M gas) as required by QuoterV2 simulation
    let result = eth_call_with_gas(quoter, &calldata, rpc_url, "0x4C4B40").await?;
    let raw = result.trim_start_matches("0x");
    if raw.len() < 64 {
        anyhow::bail!(
            "QuoterV2 returned empty/short result — pool may not exist or fee tier mismatch"
        );
    }
    // amountOut is the first 32 bytes of the return
    let amount_out = u128::from_str_radix(&raw[0..64], 16).unwrap_or(0);
    Ok(amount_out)
}

// ── ERC-721 ownerOf ───────────────────────────────────────────────────────────

/// Check owner of an NFT position.
/// ownerOf(uint256) selector = 0x6352211e
pub async fn get_owner_of(nfpm: &str, token_id: u128, rpc_url: &str) -> Result<String> {
    let calldata = format!("0x6352211e{:0>64x}", token_id);
    let hex = eth_call(nfpm, &calldata, rpc_url).await?;
    Ok(decode_address_from_hex(&hex))
}

// ── NonfungiblePositionManager ────────────────────────────────────────────────

/// Data returned from positions(tokenId).
pub struct PositionData {
    pub token0: String,
    pub token1: String,
    pub fee: u32,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity: u128,
    pub tokens_owed0: u128,
    pub tokens_owed1: u128,
}

/// positions(uint256) selector = 0x99fbab88
/// Fields: nonce(0), operator(1), token0(2), token1(3), fee(4), tickLower(5),
///         tickUpper(6), liquidity(7), feeGrowthInside0LastX128(8),
///         feeGrowthInside1LastX128(9), tokensOwed0(10), tokensOwed1(11)
pub async fn get_position(nfpm: &str, token_id: u128, rpc_url: &str) -> Result<PositionData> {
    let calldata = format!("0x99fbab88{:0>64x}", token_id);
    let hex = eth_call(nfpm, &calldata, rpc_url).await?;
    let raw = hex.trim_start_matches("0x");

    if raw.len() < 12 * 64 {
        anyhow::bail!("Invalid positions() response for tokenId {}", token_id);
    }

    let field = |n: usize| &raw[n * 64..(n + 1) * 64];

    let token0 = decode_address_from_hex(field(2));
    let token1 = decode_address_from_hex(field(3));
    let fee = u32::from_str_radix(field(4), 16).unwrap_or(0);
    let tick_lower = decode_tick(field(5));
    let tick_upper = decode_tick(field(6));
    let liquidity = u128::from_str_radix(field(7), 16).unwrap_or(0);
    let tokens_owed0 = u128::from_str_radix(field(10), 16).unwrap_or(0);
    let tokens_owed1 = u128::from_str_radix(field(11), 16).unwrap_or(0);

    Ok(PositionData {
        token0,
        token1,
        fee,
        tick_lower,
        tick_upper,
        liquidity,
        tokens_owed0,
        tokens_owed1,
    })
}

/// balanceOf + tokenOfOwnerByIndex to enumerate all position NFTs for an owner.
pub async fn get_token_ids_for_owner(
    nfpm: &str,
    owner: &str,
    rpc_url: &str,
) -> Result<Vec<u128>> {
    // balanceOf(address) = 0x70a08231
    let padded_owner = format!("{:0>64}", &owner.trim_start_matches("0x"));
    let balance_hex = eth_call(nfpm, &format!("0x70a08231{}", padded_owner), rpc_url).await?;
    let balance = decode_u256_from_hex(&balance_hex) as usize;

    let mut ids = Vec::with_capacity(balance);
    for i in 0..balance {
        // tokenOfOwnerByIndex(address,uint256) = 0x2f745c59
        let calldata = format!(
            "0x2f745c59{:0>64}{:0>64x}",
            &owner.trim_start_matches("0x"),
            i
        );
        let hex = eth_call(nfpm, &calldata, rpc_url).await?;
        ids.push(decode_u256_from_hex(&hex));
    }
    Ok(ids)
}
