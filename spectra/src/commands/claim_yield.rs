use anyhow::Result;
use serde_json::Value;

use crate::config::rpc_url;
use crate::onchainos::{
    decode_uint, encode_address, eth_call, extract_tx_hash_or_err,
    resolve_wallet, wallet_contract_call,
};

pub async fn run(
    chain_id: u64,
    pt_address: &str,
    receiver: Option<&str>,
    from: Option<&str>,
    in_ibt: bool,   // if true, use claimYieldInIBT; otherwise claimYield (underlying)
    dry_run: bool,
) -> Result<Value> {
    let rpc = rpc_url(chain_id);

    // Resolve wallet
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        let w = resolve_wallet(chain_id).unwrap_or_default();
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet. Pass --from or ensure onchainos is logged in.");
        }
        w
    };
    let rcv = receiver.unwrap_or(&wallet);

    // Preview pending yield: getCurrentYieldOfUserInIBT(address) => 0x0e1b6d89
    let yield_calldata = format!("0x0e1b6d89{}", encode_address(&wallet));
    let pending_ibt = eth_call(rpc, pt_address, &yield_calldata)
        .await
        .map(|h| decode_uint(&h))
        .unwrap_or(0);

    if pending_ibt == 0 && !dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "operation": "claim-yield",
            "chain_id": chain_id,
            "pt": pt_address,
            "pending_yield_ibt": "0",
            "message": "No pending yield to claim.",
            "tx_hash": null,
            "dry_run": dry_run
        }));
    }

    // Build calldata
    // claimYield(address receiver) => 0x999927df
    // claimYieldInIBT(address receiver) => 0x0fba731e
    let selector = if in_ibt { "0x0fba731e" } else { "0x999927df" };
    let calldata = format!("{}{}", selector, encode_address(rcv));

    let tx_result = wallet_contract_call(
        chain_id,
        pt_address,
        &calldata,
        Some(&wallet),
        None,
        true,
        dry_run,
    )
    .await?;
    let tx_hash = extract_tx_hash_or_err(&tx_result)?;

    Ok(serde_json::json!({
        "ok": true,
        "operation": if in_ibt { "claimYieldInIBT" } else { "claimYield" },
        "chain_id": chain_id,
        "pt": pt_address,
        "pending_yield_ibt_raw": pending_ibt.to_string(),
        "receiver": rcv,
        "wallet": wallet,
        "calldata": calldata,
        "tx_hash": tx_hash,
        "dry_run": dry_run
    }))
}
