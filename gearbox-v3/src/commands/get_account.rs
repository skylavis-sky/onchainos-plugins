/// get-account: Fetch Credit Account data for a borrower.
///
/// Calls DataCompressor.getCreditAccountsByBorrower(address borrower, PriceOnDemand[] priceUpdates)
/// Selector: 0x16e5b9f1
/// Pass empty priceUpdates array [] for standard tokens (USDC, WETH).

use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::onchainos::resolve_wallet;
use crate::rpc::{eth_call, strip_0x, parse_address};

pub async fn run(
    chain_id: u64,
    from: Option<&str>,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let dc = cfg.data_compressor;

    // Resolve borrower address
    let borrower = match from {
        Some(addr) => addr.to_string(),
        None => resolve_wallet(chain_id).context("Failed to resolve wallet address")?,
    };

    // Encode getCreditAccountsByBorrower(address borrower, (address,uint256,bytes)[] priceUpdates)
    // Selector: 0x16e5b9f1
    // Params: address (32 bytes) + offset to priceUpdates array (32 bytes) + empty array (32 bytes for length=0)
    let addr_bytes = parse_address(&borrower)
        .with_context(|| format!("Invalid borrower address: {}", borrower))?;

    let mut calldata = hex::decode("16e5b9f1")?;
    // Param 1: borrower address
    calldata.extend_from_slice(&[0u8; 12]);
    calldata.extend_from_slice(&addr_bytes);
    // Param 2: offset to priceUpdates array = 64 (0x40) — right after the two fixed params
    calldata.extend_from_slice(&[0u8; 24]);
    calldata.extend_from_slice(&(64u64).to_be_bytes());
    // priceUpdates array: length = 0 (empty array)
    calldata.extend_from_slice(&[0u8; 32]);

    let calldata_hex = format!("0x{}", hex::encode(&calldata));

    let eth_result = eth_call(rpc, dc, &calldata_hex).await;

    // DataCompressor reverts for addresses with no open accounts — treat as empty list
    let result_hex = match eth_result {
        Ok(hex) => hex,
        Err(_) => {
            return Ok(json!({
                "ok": true,
                "chain": chain_id,
                "borrower": borrower,
                "creditAccounts": [],
                "message": "No open Credit Accounts found."
            }));
        }
    };

    let raw = strip_0x(&result_hex);

    if raw.len() < 128 {
        return Ok(json!({
            "ok": true,
            "chain": chain_id,
            "borrower": borrower,
            "creditAccounts": [],
            "message": "No open Credit Accounts found."
        }));
    }

    // Parse outer array length (at offset 0x20 = slot 1 after the initial offset pointer)
    // Response: [offset_ptr (32)] [array_len (32)] [element_offsets...] [element_data...]
    let array_len_hex = &raw[64..128];
    let count = u64::from_str_radix(&array_len_hex[48..64], 16).unwrap_or(0);

    if count == 0 {
        return Ok(json!({
            "ok": true,
            "chain": chain_id,
            "borrower": borrower,
            "creditAccounts": [],
            "message": "No open Credit Accounts found."
        }));
    }

    // CreditAccountData struct is extremely complex with nested arrays.
    // For v0.1, we return the raw count and a note to use the facade address directly.
    // Users should use the facade address to query specific account data.
    Ok(json!({
        "ok": true,
        "chain": chain_id,
        "borrower": borrower,
        "creditAccountCount": count,
        "message": format!("Found {} Credit Account(s). Use DataCompressor.getCreditAccountsByBorrower() for full details.", count),
        "tip": "To query a specific account, use the Gearbox app or call DataCompressor directly with the account address.",
        "dataCompressor": dc,
        "rawResponseLength": raw.len()
    }))
}
