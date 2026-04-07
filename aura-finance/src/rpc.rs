use anyhow::Result;
use serde_json::{json, Value};
use crate::config::ETH_RPC;

fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// Execute an eth_call on Ethereum mainnet
pub async fn eth_call(to: &str, data: &str) -> Result<String> {
    let client = build_client();
    let body = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client.post(ETH_RPC)
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

/// Pad address to 32 bytes (remove 0x prefix, left-pad with zeros)
pub fn pad_address(addr: &str) -> String {
    let clean = addr.strip_prefix("0x").unwrap_or(addr).to_lowercase();
    format!("{:0>64}", clean)
}

/// Decode uint256 from ABI-encoded result
pub fn decode_u256(hex: &str) -> u128 {
    let clean = hex.strip_prefix("0x").unwrap_or(hex);
    if clean.len() < 64 {
        return 0;
    }
    u128::from_str_radix(&clean[32..64], 16).unwrap_or(0)
}

/// Read ERC-20 balanceOf(address) - returns raw balance as u128
pub async fn erc20_balance_of(token: &str, wallet: &str) -> Result<u128> {
    let data = format!("0x70a08231{}", pad_address(wallet));
    let result = eth_call(token, &data).await?;
    Ok(decode_u256(&result))
}

/// Read ERC-20 allowance(owner, spender) - returns raw allowance as u128
pub async fn erc20_allowance(token: &str, owner: &str, spender: &str) -> Result<u128> {
    let data = format!("0xdd62ed3e{}{}", pad_address(owner), pad_address(spender));
    let result = eth_call(token, &data).await?;
    Ok(decode_u256(&result))
}

/// Read BaseRewardPool.earned(address) - pending BAL rewards
pub async fn base_reward_pool_earned(reward_pool: &str, wallet: &str) -> Result<u128> {
    // earned(address) selector: 0x008cc262
    let data = format!("0x008cc262{}", pad_address(wallet));
    let result = eth_call(reward_pool, &data).await?;
    Ok(decode_u256(&result))
}

/// Read Booster.poolLength() - total number of pools
pub async fn booster_pool_length(booster: &str) -> Result<u64> {
    // poolLength() selector: 0x081e3eda
    let data = "0x081e3eda";
    let result = eth_call(booster, data).await?;
    let clean = result.strip_prefix("0x").unwrap_or(&result);
    if clean.len() < 64 {
        return Ok(0);
    }
    Ok(u64::from_str_radix(&clean[32..64], 16).unwrap_or(0))
}

/// Read Booster.poolInfo(pid) - returns (lptoken, token, gauge, crvRewards, stash, shutdown)
/// Returns (lptoken, crvRewards, shutdown) - the three most useful fields
pub async fn booster_pool_info(booster: &str, pid: u64) -> Result<(String, String, bool)> {
    // poolInfo(uint256) selector: 0x1526fe27
    let pid_hex = format!("{:064x}", pid);
    let data = format!("0x1526fe27{}", pid_hex);
    let result = eth_call(booster, &data).await?;
    let clean = result.strip_prefix("0x").unwrap_or(&result);

    // Result is 6 x 32 bytes: lptoken, token, gauge, crvRewards, stash, shutdown
    if clean.len() < 6 * 64 {
        anyhow::bail!("poolInfo response too short for pid {}", pid);
    }

    // Each address field is right-padded in a 32-byte slot; last 40 hex chars = address
    let lptoken = format!("0x{}", &clean[24..64]);
    let crv_rewards = format!("0x{}", &clean[24 + 3 * 64..64 + 3 * 64]);
    // shutdown is the 6th field (index 5), last byte of that 32-byte slot
    let shutdown_raw = u64::from_str_radix(&clean[5 * 64..6 * 64], 16).unwrap_or(0);
    let shutdown = shutdown_raw != 0;

    Ok((lptoken, crv_rewards, shutdown))
}

/// Format token amount with decimals for display
pub fn format_amount(raw: u128, decimals: u32) -> String {
    let divisor = 10u128.pow(decimals);
    let whole = raw / divisor;
    let frac = raw % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        format!("{}.{:0>width$}", whole, frac, width = decimals as usize)
            .trim_end_matches('0')
            .to_string()
    }
}
