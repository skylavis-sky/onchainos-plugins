// onchainos CLI wrapper — all on-chain writes go through here
// FACTS (verified v2.2.6):
// - EVM calldata flag: --input-data (NOT --calldata)
// - txHash location: data.txHash
// - No dex approve command; use wallet contract-call with manual calldata
// - dry_run: return simulated response early, never pass --dry-run to onchainos

use anyhow::Result;
use serde_json::Value;
use std::process::Command;

/// Resolve EVM wallet address for chain 1 (Ethereum)
pub fn resolve_wallet() -> Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)?;
    // Find evm address for chainIndex "1"
    if let Some(evm_arr) = json["data"]["evm"].as_array() {
        for entry in evm_arr {
            if entry["chainIndex"].as_str() == Some("1")
                || entry["chainIndex"].as_u64() == Some(1)
            {
                if let Some(addr) = entry["address"].as_str() {
                    return Ok(addr.to_string());
                }
            }
        }
        // fallback: return first evm address
        if let Some(first) = evm_arr.first() {
            if let Some(addr) = first["address"].as_str() {
                return Ok(addr.to_string());
            }
        }
    }
    anyhow::bail!("Could not resolve EVM wallet address. Make sure onchainos is logged in.")
}

/// Call onchainos wallet contract-call
/// dry_run=true returns a mock response without calling onchainos
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": input_data
        }));
    }
    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet",
        "contract-call",
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
    ];
    let from_str: String;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }
    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout).unwrap_or_else(|_| {
        serde_json::json!({ "ok": false, "error": stdout.to_string() })
    }))
}

/// ERC-20 approve(spender, amount)
/// selector: 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> Result<Value> {
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x").to_lowercase());
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, dry_run).await
}

/// Extract txHash from onchainos response
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .unwrap_or("pending")
        .to_string()
}

/// Check if onchainos returned a successful result
pub fn is_ok(result: &Value) -> bool {
    result["ok"].as_bool().unwrap_or(false)
}
