/// Exactly Protocol Previewer decoder.
///
/// Previewer.exactly(address account) — selector: 0x157c9e0e
/// Returns MarketAccount[] — an array of structs, one per market.
///
/// MarketAccount struct (from Exactly Protocol source):
/// struct MarketAccount {
///   Market market;                 // address (slot 0)
///   uint8 decimals;                // slot 1 (packed in uint256)
///   string symbol;                 // dynamic string (slot 2 = offset ptr)
///   bool isCollateral;             // slot 3
///   uint128 maxFuturePools;        // slot 4
///   uint256 fixedDeposits;         // slot 5 (bitfield for maturity bitmask)
///   uint256 fixedBorrows;          // slot 6 (bitfield)
///   uint256 floatingBorrowShares;  // slot 7
///   uint256 floatingDepositShares; // slot 8
///   uint256 floatingAssets;        // slot 9 (totalFloatingDepositAssets)
///   uint256 floatingBorrowAssets;  // slot 10 (totalFloatingBorrowAssets)
///   uint256 seizeAvailable;        // slot 11
///   uint256 claimableRewards;      // slot 12
///   uint256 floatingUtilization;   // slot 13
///   FixedPool[] fixedPools;        // slot 14 = offset ptr (dynamic)
///   ... more fields (rates, market config) ...
/// }
///
/// Due to the complexity of full recursive ABI decoding, we use a best-effort approach:
/// - Market addresses are read from slot 0 of each tuple
/// - Symbol/decimals are enriched from our config (which we know)
/// - The contract call confirms the Previewer is live and returns data

use serde_json::{json, Value};

use crate::config::ChainConfig;
use crate::rpc;

/// Call Previewer.exactly(account) and return the raw hex response.
/// Use address(0) = 0x0000...0000 for market-only queries (no position data).
pub async fn call_previewer(
    previewer_addr: &str,
    account: &str,
    rpc_url: &str,
) -> anyhow::Result<String> {
    // exactly(address) selector: 0x157c9e0e
    let addr_bytes = rpc::parse_address(account)?;
    let mut calldata_bytes = hex::decode("157c9e0e")?;
    calldata_bytes.extend_from_slice(&[0u8; 12]);
    calldata_bytes.extend_from_slice(&addr_bytes);
    let calldata = format!("0x{}", hex::encode(&calldata_bytes));
    rpc::eth_call(rpc_url, previewer_addr, &calldata)
        .await
        .map_err(|e| anyhow::anyhow!("Previewer.exactly() eth_call failed: {}", e))
}

/// Parse the market count from Previewer output.
/// The ABI-encoded MarketAccount[] starts with: [offset (32b)] [length (32b)] [element offsets...]
pub fn parse_market_count(raw: &str) -> usize {
    if raw.len() < 128 {
        return 0;
    }
    let len_word = &raw[64..128];
    u64_from_hex_word(len_word) as usize
}

/// Parse market addresses from Previewer output.
#[allow(dead_code)]
/// Each MarketAccount tuple is at a relative offset from the array data start.
/// The first field of each MarketAccount is the market contract address.
pub fn parse_market_addresses(raw: &str) -> Vec<String> {
    let market_count = parse_market_count(raw);
    let mut addresses = Vec::new();

    let array_data_start = 128usize; // offset (64 chars) + length (64 chars)

    for i in 0..market_count.min(10) {
        let offset_ptr_pos = array_data_start + i * 64;
        if raw.len() < offset_ptr_pos + 64 {
            break;
        }
        let offset_word = &raw[offset_ptr_pos..offset_ptr_pos + 64];
        let rel_offset = u64_from_hex_word(offset_word) as usize;

        // Absolute position: array_data_start + rel_offset * 2 (hex chars per byte)
        let tuple_start = array_data_start + rel_offset * 2;

        if raw.len() < tuple_start + 64 {
            break;
        }

        // Slot 0 of each tuple: market address (last 40 chars of 64-char slot)
        let slot0 = &raw[tuple_start..tuple_start + 64];
        let market_addr = format!("0x{}", &slot0[24..64]);
        addresses.push(market_addr);
    }
    addresses
}

/// Get market data enriched with config-known symbols.
/// We call the Previewer to get live data (market addresses, utilization, deposit/borrow totals),
/// then enrich with known symbols and decimals from config.
pub async fn get_markets(
    previewer_addr: &str,
    rpc_url: &str,
    chain_cfg: &'static ChainConfig,
    account: Option<&str>,
) -> anyhow::Result<Value> {
    let addr = account.unwrap_or("0x0000000000000000000000000000000000000000");
    let hex = call_previewer(previewer_addr, addr, rpc_url).await?;
    let raw = rpc::strip_0x(&hex);

    if raw.len() < 128 {
        anyhow::bail!(
            "Previewer returned too short a response ({} chars)",
            raw.len()
        );
    }

    let market_count = parse_market_count(raw);
    if market_count == 0 {
        return Ok(json!({
            "ok": true,
            "markets": [],
            "message": "No markets found"
        }));
    }

    let array_data_start = 128usize;
    let mut markets = Vec::new();

    for i in 0..market_count.min(10) {
        let offset_ptr_pos = array_data_start + i * 64;
        if raw.len() < offset_ptr_pos + 64 {
            break;
        }
        let offset_word = &raw[offset_ptr_pos..offset_ptr_pos + 64];
        let rel_offset = u64_from_hex_word(offset_word) as usize;
        let tuple_start = array_data_start + rel_offset * 2;

        if raw.len() < tuple_start + 64 * 15 {
            eprintln!("Warning: insufficient data for market index {}", i);
            continue;
        }

        // Helper to read a 64-char slot from tuple_start + slot_idx * 64
        let slot = |n: usize| -> &str {
            let s = tuple_start + n * 64;
            let e = s + 64;
            if e <= raw.len() {
                &raw[s..e]
            } else {
                "0000000000000000000000000000000000000000000000000000000000000000"
            }
        };

        // Slot 0: market address
        let market_addr_raw = format!("0x{}", &slot(0)[24..64]);

        // Look up known config for this market address
        let known_market = chain_cfg.markets.iter().find(|m| {
            m.market_address.eq_ignore_ascii_case(&market_addr_raw)
        });

        let symbol = known_market.map(|m| m.symbol).unwrap_or("UNKNOWN");
        let asset_addr = known_market
            .map(|m| m.asset_address.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let decimals = known_market.map(|m| m.decimals).unwrap_or(18);

        // Slot 1: decimals (uint8 in uint256 slot) — use lower byte
        // We prefer config decimals over parsed (config is more reliable)

        // Slot 3: isCollateral (bool — 1 if true)
        let is_collateral = u128_from_slot(slot(3)) != 0;

        // Slot 7: floatingBorrowShares (user)
        let floating_borrow_shares = u128_from_slot(slot(7));

        // Slot 8: floatingDepositShares (user)
        let floating_deposit_shares = u128_from_slot(slot(8));

        // Slot 9: floatingAssets (total floating deposit assets)
        let total_floating_deposit = u128_from_slot(slot(9));

        // Slot 10: floatingBorrowAssets (total floating borrow assets)
        let total_floating_borrow = u128_from_slot(slot(10));

        // Slot 13: floatingUtilization (1e18 scaled)
        let utilization = u128_from_slot(slot(13));

        let decimal_factor = 10u128.pow(decimals as u32) as f64;
        let total_supply_human = total_floating_deposit as f64 / decimal_factor;
        let total_borrow_human = total_floating_borrow as f64 / decimal_factor;
        let util_pct = utilization as f64 / 1e16;

        // User floating deposit assets (slot 8 is shares, not assets)
        // We use non-zero shares to indicate user has a position
        let user_has_deposit = floating_deposit_shares > 0;
        let user_has_borrow = floating_borrow_shares > 0;

        markets.push(json!({
            "market": market_addr_raw,
            "asset": asset_addr,
            "symbol": symbol,
            "decimals": decimals,
            "totalFloatingDeposit": format!("{:.4}", total_supply_human),
            "totalFloatingBorrow": format!("{:.4}", total_borrow_human),
            "utilization": format!("{:.2}%", util_pct),
            "isCollateral": is_collateral,
            "userHasFloatingDeposit": user_has_deposit,
            "userHasFloatingBorrow": user_has_borrow,
            "floatingDepositShares": floating_deposit_shares.to_string(),
            "floatingBorrowShares": floating_borrow_shares.to_string(),
        }));
    }

    Ok(json!({
        "ok": true,
        "account": addr,
        "marketCount": markets.len(),
        "markets": markets
    }))
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn u64_from_hex_word(word: &str) -> u64 {
    let trimmed = word.trim_start_matches('0');
    if trimmed.is_empty() {
        return 0;
    }
    u64::from_str_radix(trimmed, 16).unwrap_or(0)
}

fn u128_from_slot(slot: &str) -> u128 {
    if slot.len() < 64 {
        return 0;
    }
    // Take lower 32 hex chars (16 bytes) to fit in u128
    u128::from_str_radix(&slot[32..64], 16).unwrap_or(0)
}
