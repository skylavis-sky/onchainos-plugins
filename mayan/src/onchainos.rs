use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;
use std::process::Command;

/// Resolve EVM wallet address from onchainos wallet balance
pub fn resolve_wallet_evm(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try details[0].tokenAssets[0].address first
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // Fallback to data.address
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Resolve Solana wallet address from onchainos wallet balance (no --output json)
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value =
        serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Could not resolve Solana wallet address from onchainos output")
}

/// Convert base64-encoded transaction to base58 (required for --unsigned-tx)
pub fn base64_to_base58(b64: &str) -> anyhow::Result<String> {
    let bytes = BASE64.decode(b64.trim())?;
    Ok(bs58::encode(&bytes).into_string())
}

/// Execute an EVM contract call via onchainos
pub async fn wallet_contract_call_evm(
    chain_id: u64,
    to: &str,
    input_data: &str,
    amt: Option<u64>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "to": to,
            "input_data": input_data,
            "amt": amt,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }
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
        "--force",
    ];
    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }
    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(
        serde_json::from_str(&stdout).unwrap_or_else(|_| {
            serde_json::json!({"ok": false, "error": stdout.to_string()})
        }),
    )
}

/// Execute a Solana contract call via onchainos using a base58-encoded unsigned tx
pub async fn wallet_contract_call_solana(
    program_id: &str,
    tx_base58: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "program_id": program_id,
            "data": {"txHash": ""}
        }));
    }
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            program_id,
            "--unsigned-tx",
            tx_base58,
            "--force",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(
        serde_json::from_str(&stdout).unwrap_or_else(|_| {
            serde_json::json!({"ok": false, "error": stdout.to_string()})
        }),
    )
}

/// Extract transaction hash from onchainos response (checks swapTxHash, txHash)
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
