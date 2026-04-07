use anyhow::Result;
use serde_json::Value;

use crate::config::{registry_address, rpc_url, KNOWN_BASE_POOLS};
use crate::onchainos::{decode_address, decode_uint, encode_address, encode_uint256, eth_call, resolve_wallet};

/// Fetch ERC-20 balance of token for owner
async fn erc20_balance(rpc: &str, token: &str, owner: &str) -> u128 {
    // balanceOf(address) selector = 0x70a08231
    let calldata = format!("0x70a08231{}", encode_address(owner));
    eth_call(rpc, token, &calldata)
        .await
        .map(|h| decode_uint(&h))
        .unwrap_or(0)
}

/// Get decimals for a token
async fn erc20_decimals(rpc: &str, token: &str) -> u8 {
    // decimals() selector = 0x313ce567
    eth_call(rpc, token, "0x313ce567")
        .await
        .map(|h| decode_uint(&h) as u8)
        .unwrap_or(18)
}

fn format_amount(raw: u128, decimals: u8) -> String {
    let div = 10u128.pow(decimals as u32);
    let whole = raw / div;
    let frac = raw % div;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

pub async fn run(user: Option<&str>, chain_id: u64) -> Result<Value> {
    let rpc = rpc_url(chain_id);
    let registry = registry_address(chain_id);

    // Resolve wallet
    let wallet = if let Some(u) = user {
        u.to_string()
    } else {
        let w = resolve_wallet(chain_id).unwrap_or_default();
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet address. Pass --user or ensure onchainos is logged in.");
        }
        w
    };

    // Get total PT count from registry
    let count_hex = eth_call(rpc, registry, "0x704bdadc").await?;
    let count = decode_uint(&count_hex) as u64;
    let limit = count.min(69); // scan up to 69 PTs

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut positions: Vec<Value> = Vec::new();

    for i in 0..limit {
        let calldata = format!("0x6c40a4f0{}", encode_uint256(i as u128));
        let pt_hex = match eth_call(rpc, registry, &calldata).await {
            Ok(h) => h,
            Err(_) => continue,
        };
        let pt_addr = decode_address(&pt_hex);
        if pt_addr == "0x0000000000000000000000000000000000000000" {
            continue;
        }

        // Check PT balance — skip if zero
        let pt_balance = erc20_balance(rpc, &pt_addr, &wallet).await;

        // Get YT address
        let yt_addr = eth_call(rpc, &pt_addr, "0x04aa50ad")
            .await
            .map(|h| decode_address(&h))
            .unwrap_or_default();
        let yt_balance = erc20_balance(rpc, &yt_addr, &wallet).await;

        if pt_balance == 0 && yt_balance == 0 {
            continue;
        }

        // Maturity
        let maturity = eth_call(rpc, &pt_addr, "0x204f83f9")
            .await
            .map(|h| decode_uint(&h) as u64)
            .unwrap_or(0);
        let is_expired = maturity > 0 && maturity <= now_ts;
        let days_to_maturity: i64 = if maturity > now_ts {
            ((maturity - now_ts) / 86400) as i64
        } else if maturity > 0 {
            -(((now_ts - maturity) / 86400) as i64)
        } else {
            0
        };

        // Underlying
        let underlying_addr = eth_call(rpc, &pt_addr, "0x6f307dc3")
            .await
            .map(|h| decode_address(&h))
            .unwrap_or_default();

        // IBT
        let ibt_addr = eth_call(rpc, &pt_addr, "0xc644fe94")
            .await
            .map(|h| decode_address(&h))
            .unwrap_or_default();

        let decimals = erc20_decimals(rpc, &pt_addr).await;

        // Pending yield: getCurrentYieldOfUserInIBT(address) => 0x0e1b6d89
        let yield_calldata = format!("0x0e1b6d89{}", encode_address(&wallet));
        let pending_yield_ibt = eth_call(rpc, &pt_addr, &yield_calldata)
            .await
            .map(|h| decode_uint(&h))
            .unwrap_or(0);

        // previewRedeem for redemption value
        let mut redeem_value: u128 = 0;
        if pt_balance > 0 {
            let preview_calldata = format!("0x4cdad506{}", encode_uint256(pt_balance));
            redeem_value = eth_call(rpc, &pt_addr, &preview_calldata)
                .await
                .map(|h| decode_uint(&h))
                .unwrap_or(0);
        }

        let known = KNOWN_BASE_POOLS
            .iter()
            .find(|p| p.pt.to_lowercase() == pt_addr.to_lowercase());
        let name = known
            .map(|p| p.name.to_string())
            .unwrap_or_else(|| format!("PT-{}", &pt_addr[..10]));

        positions.push(serde_json::json!({
            "name": name,
            "pt": pt_addr,
            "yt": yt_addr,
            "ibt": ibt_addr,
            "underlying": underlying_addr,
            "maturity_ts": maturity,
            "days_to_maturity": days_to_maturity,
            "expired": is_expired,
            "pt_balance_raw": pt_balance.to_string(),
            "pt_balance": format_amount(pt_balance, decimals),
            "yt_balance_raw": yt_balance.to_string(),
            "yt_balance": format_amount(yt_balance, decimals),
            "redeem_value_raw": redeem_value.to_string(),
            "redeem_value": format_amount(redeem_value, decimals),
            "pending_yield_ibt_raw": pending_yield_ibt.to_string(),
            "pending_yield_ibt": format_amount(pending_yield_ibt, decimals)
        }));
    }

    Ok(serde_json::json!({
        "ok": true,
        "wallet": wallet,
        "chain_id": chain_id,
        "position_count": positions.len(),
        "positions": positions
    }))
}
