// onchainos CLI helpers for Sanctum Validator LSTs plugin
//
// Key facts (verified against onchainos v2.2.6):
//   - `wallet balance --chain 501` returns JSON natively; do NOT pass --output json
//   - `wallet contract-call --unsigned-tx` requires BASE58; convert from base64 internally
//   - Must pass --force or broadcast returns txHash:"pending"

use anyhow::Result;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;
use std::process::Command;

/// Resolve the logged-in Solana wallet address.
/// Address is at data.details[0].tokenAssets[0].address (fallback: data.address).
pub fn resolve_wallet_solana() -> Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let err = if !stderr.is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
        anyhow::bail!("onchainos failed (exit {}): {}", output.status, err);
    }

    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet balance: {}\nOutput: {}", e, stdout))?;

    if json["ok"].as_bool() != Some(true) {
        let err_msg = json["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos execution failed: {}", err_msg);
    }

    // Primary path: details[0].tokenAssets[0].address
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }

    // Fallback: data.address
    json["data"]["address"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!(
            "Cannot resolve Solana wallet address. Make sure onchainos is logged in."
        ))
}

/// Convert a base64-encoded serialized transaction to base58.
/// onchainos --unsigned-tx requires base58; Sanctum API returns base64.
pub fn base64_to_base58(b64: &str) -> Result<String> {
    let bytes = BASE64
        .decode(b64.trim())
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    Ok(bs58::encode(bytes).into_string())
}

/// Submit a Solana transaction via `onchainos wallet contract-call`.
///
/// serialized_tx: base64-encoded transaction (converted to base58 internally)
/// program_id: the target program address (--to)
/// dry_run: if true, return a mock response without calling onchainos
pub async fn wallet_contract_call_solana(
    program_id: &str,
    serialized_tx: &str, // base64-encoded
    dry_run: bool,
) -> Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": serialized_tx
        }));
    }

    let tx_base58 = base64_to_base58(serialized_tx)?;

    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            program_id,
            "--unsigned-tx",
            &tx_base58,
            "--force",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        let err = if !stderr.is_empty() { stderr.trim().to_string() } else { stdout.trim().to_string() };
        anyhow::bail!("onchainos failed (exit {}): {}", output.status, err);
    }

    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}\nOutput: {}", e, stdout))?;

    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos execution failed: {}", err_msg);
    }

    Ok(result)
}

/// Extract txHash from an onchainos response.
/// Checks: data.swapTxHash → data.txHash → txHash (root)
pub fn extract_tx_hash(result: &Value) -> Result<String> {
    let hash = result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str());

    match hash {
        Some(h) if !h.is_empty() && h != "pending" => Ok(h.to_string()),
        _ => anyhow::bail!("txHash not found in onchainos output; raw: {}", result),
    }
}
