// commands/claim_withdrawal.rs — Claim finalized ETH withdrawal from Lido
use anyhow::Result;
use serde_json::json;

use crate::config;
use crate::onchainos;
use crate::rpc;

pub async fn run(
    request_ids: Vec<u64>,
    from: Option<String>,
    dry_run: bool,
) -> Result<()> {
    if request_ids.is_empty() {
        anyhow::bail!("Provide at least one --request-ids value");
    }

    // Resolve wallet
    let wallet = from.unwrap_or_else(|| {
        onchainos::resolve_wallet(config::CHAIN_ETHEREUM).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Provide --from or ensure onchainos is logged in.");
    }

    // Step 1: Get last checkpoint index
    // getLastCheckpointIndex() selector: 0x526eae3e
    let checkpoint_hex = rpc::eth_call(
        config::WITHDRAWAL_QUEUE_ADDRESS,
        "0x526eae3e",
        config::RPC_ETHEREUM,
    )
    .await?;
    let last_checkpoint = rpc::decode_uint256(&checkpoint_hex);

    // Step 2: Get hints via findCheckpointHints(uint256[], uint256, uint256)
    // selector: 0x62abe3fa
    let hints_calldata = build_find_hints_calldata(&request_ids, 1, last_checkpoint);
    let hints_hex = rpc::eth_call(
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &hints_calldata,
        config::RPC_ETHEREUM,
    )
    .await?;
    let hints = decode_uint256_array(&hints_hex);

    // Build claimWithdrawals(uint256[], uint256[]) calldata
    // selector: 0xe3afe0a3
    let calldata = build_claim_withdrawals_calldata(&request_ids, &hints);

    let preview = json!({
        "operation": "claim-withdrawal",
        "from": wallet,
        "requestIds": request_ids,
        "hints": hints.iter().map(|h| h.to_string()).collect::<Vec<_>>(),
        "lastCheckpointIndex": last_checkpoint.to_string(),
        "calldata": calldata,
        "note": "Ask user to confirm before claiming ETH from finalized withdrawal requests"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Execute: ask user to confirm claim transaction
    let result = onchainos::wallet_contract_call(
        config::CHAIN_ETHEREUM,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!(
        "{}",
        json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "operation": "claim-withdrawal",
                "requestIds": request_ids,
                "message": "ETH successfully claimed to your wallet"
            }
        })
    );
    Ok(())
}

/// Build findCheckpointHints(uint256[] _requestIds, uint256 _firstIndex, uint256 _lastIndex) calldata
/// selector: 0x62abe3fa
fn build_find_hints_calldata(ids: &[u64], first: u128, last: u128) -> String {
    // ABI: (uint256[], uint256, uint256)
    // offset to array = 3 * 32 = 96 = 0x60
    let array_offset = format!("{:064x}", 96u64);
    let first_hex = format!("{:064x}", first);
    let last_hex = format!("{:064x}", last);
    let array_len = format!("{:064x}", ids.len());
    let elements: String = ids.iter().map(|id| format!("{:064x}", id)).collect();
    format!(
        "0x62abe3fa{}{}{}{}{}",
        array_offset, first_hex, last_hex, array_len, elements
    )
}

/// Build claimWithdrawals(uint256[] requestIds, uint256[] hints) calldata
/// selector: 0xe3afe0a3
fn build_claim_withdrawals_calldata(ids: &[u64], hints: &[u128]) -> String {
    // ABI: (uint256[], uint256[])
    // Two dynamic arrays. Offsets:
    //   slot 0: offset to ids array = 2 * 32 = 64
    //   slot 1: offset to hints array = 64 + 32 + ids.len() * 32
    let ids_array_offset = 64u64;
    let hints_array_offset = ids_array_offset + 32 + (ids.len() as u64) * 32;

    let ids_offset_hex = format!("{:064x}", ids_array_offset);
    let hints_offset_hex = format!("{:064x}", hints_array_offset);

    let ids_len = format!("{:064x}", ids.len());
    let ids_elems: String = ids.iter().map(|id| format!("{:064x}", id)).collect();

    let hints_len = format!("{:064x}", hints.len());
    let hints_elems: String = hints.iter().map(|h| format!("{:064x}", h)).collect();

    format!(
        "0xe3afe0a3{}{}{}{}{}{}",
        ids_offset_hex, hints_offset_hex, ids_len, ids_elems, hints_len, hints_elems
    )
}

/// Decode ABI-encoded uint256[] response
fn decode_uint256_array(hex: &str) -> Vec<u128> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return vec![];
    }
    let bytes: Vec<u8> = (0..clean.len() / 2)
        .filter_map(|i| u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).ok())
        .collect();

    if bytes.len() < 32 {
        return vec![];
    }

    // First 32 bytes: offset (should be 0x20)
    let offset = read_u256_bytes(&bytes, 0) as usize;
    if offset + 32 > bytes.len() {
        return vec![];
    }
    let length = read_u256_bytes(&bytes, offset) as usize;
    let mut result = Vec::with_capacity(length);
    for i in 0..length {
        let pos = offset + 32 + i * 32;
        if pos + 32 > bytes.len() {
            break;
        }
        result.push(read_u128_bytes(&bytes, pos));
    }
    result
}

fn read_u256_bytes(bytes: &[u8], offset: usize) -> u64 {
    if offset + 32 > bytes.len() {
        return 0;
    }
    let mut v = 0u64;
    for b in &bytes[offset + 24..offset + 32] {
        v = (v << 8) | (*b as u64);
    }
    v
}

fn read_u128_bytes(bytes: &[u8], offset: usize) -> u128 {
    if offset + 32 > bytes.len() {
        return 0;
    }
    let mut v = 0u128;
    for b in &bytes[offset + 16..offset + 32] {
        v = (v << 8) | (*b as u128);
    }
    v
}
