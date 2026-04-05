// commands/get_withdrawal_status.rs — Query Lido withdrawal request status
use anyhow::Result;
use serde_json::json;

use crate::config;
use crate::rpc;

pub async fn run(request_ids: Vec<u64>) -> Result<()> {
    if request_ids.is_empty() {
        anyhow::bail!("Provide at least one --request-ids value");
    }

    // Call getWithdrawalStatus(uint256[])
    // selector: 0xb8c4b85a
    let calldata = build_get_withdrawal_status_calldata(&request_ids);
    let result_hex =
        rpc::eth_call(config::WITHDRAWAL_QUEUE_ADDRESS, &calldata, config::RPC_ETHEREUM).await?;

    // Parse the raw hex output as an array of WithdrawalRequestStatus structs
    // Each struct has 6 fields: amountOfStETH(uint256), amountOfShares(uint256), owner(address),
    //   timestamp(uint256), isFinalized(bool), isClaimed(bool)
    // ABI-decoded: first 32 bytes = offset to array data, then length, then 6*32 per element
    let statuses = decode_withdrawal_statuses(&result_hex, &request_ids);

    // Also get estimated wait times from API
    let wait_times = crate::api::get_request_time(&request_ids).await.unwrap_or(json!({}));

    println!(
        "{}",
        json!({
            "ok": true,
            "data": {
                "requestIds": request_ids,
                "statuses": statuses,
                "estimatedWait": wait_times
            }
        })
    );
    Ok(())
}

fn build_get_withdrawal_status_calldata(ids: &[u64]) -> String {
    // getWithdrawalStatus(uint256[]) selector: 0xb8c4b85a
    // ABI: offset to array = 0x20, length, elements
    let offset = format!("{:064x}", 32u64);
    let length = format!("{:064x}", ids.len());
    let elements: String = ids.iter().map(|id| format!("{:064x}", id)).collect();
    format!("0xb8c4b85a{}{}{}", offset, length, elements)
}

fn decode_withdrawal_statuses(hex: &str, ids: &[u64]) -> serde_json::Value {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return json!([]);
    }

    // The response is ABI-encoded as (WithdrawalRequestStatus[])
    // First 32 bytes: offset to array (0x20)
    // Next 32 bytes: array length
    // Then each element: 6 slots * 32 bytes = 192 bytes
    let bytes: Vec<u8> = (0..clean.len() / 2)
        .filter_map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).ok())
        .collect();

    if bytes.len() < 64 {
        return json!([]);
    }

    // Skip first 32 bytes (outer tuple offset), next 32 bytes is inner array offset
    // Array length at offset
    let arr_offset = read_u256_at(&bytes, 0) as usize;
    if arr_offset + 32 > bytes.len() {
        return json!([]);
    }
    let arr_len = read_u256_at(&bytes, arr_offset) as usize;

    let mut result = Vec::new();
    for i in 0..arr_len {
        let base = arr_offset + 32 + i * 6 * 32;
        if base + 6 * 32 > bytes.len() {
            break;
        }
        let amount_steth = read_u256_at(&bytes, base);
        let amount_shares = read_u256_at(&bytes, base + 32);
        let owner = read_address_at(&bytes, base + 64);
        let timestamp = read_u256_at(&bytes, base + 96);
        let is_finalized = read_u256_at(&bytes, base + 128) != 0;
        let is_claimed = read_u256_at(&bytes, base + 160) != 0;

        let request_id = ids.get(i).copied().unwrap_or(0);
        result.push(json!({
            "requestId": request_id,
            "amountOfStETH_wei": amount_steth.to_string(),
            "amountOfStETH": rpc::format_18dec(amount_steth as u128),
            "amountOfShares": amount_shares.to_string(),
            "owner": format!("0x{}", owner),
            "timestamp": timestamp,
            "isFinalized": is_finalized,
            "isClaimed": is_claimed,
            "status": if is_claimed {
                "claimed"
            } else if is_finalized {
                "ready_to_claim"
            } else {
                "pending"
            }
        }));
    }

    json!(result)
}

fn read_u256_at(bytes: &[u8], offset: usize) -> u64 {
    if offset + 32 > bytes.len() {
        return 0;
    }
    // Read last 8 bytes of the 32-byte slot as u64
    let mut v = 0u64;
    for b in &bytes[offset + 24..offset + 32] {
        v = (v << 8) | (*b as u64);
    }
    v
}

fn read_address_at(bytes: &[u8], offset: usize) -> String {
    if offset + 32 > bytes.len() {
        return "0000000000000000000000000000000000000000".to_string();
    }
    // Address is in the last 20 bytes of the 32-byte slot
    bytes[offset + 12..offset + 32]
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}
