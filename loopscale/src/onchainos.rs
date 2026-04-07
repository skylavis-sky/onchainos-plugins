// src/onchainos.rs — onchainos CLI wrappers for Solana (chain 501)
use std::process::Command;
use serde_json::Value;

/// Resolve the connected Solana wallet address from onchainos.
/// NOTE: Do NOT add --output json — chain 501 returns JSON natively.
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse wallet balance: {}. stdout: {}", e, stdout))?;

    // Primary path: data.details[0].tokenAssets[0].address
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
    let addr = json["data"]["address"].as_str().unwrap_or("").to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve Solana wallet address from onchainos. Ensure you are logged in.");
    }
    Ok(addr)
}

/// Convert base64-encoded transaction (from Loopscale API) to base58 for onchainos.
/// Loopscale returns base64; onchainos --unsigned-tx requires base58.
pub fn base64_to_base58(b64: &str) -> anyhow::Result<String> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine as _;
    let bytes = STANDARD.decode(b64.trim())
        .map_err(|e| anyhow::anyhow!("base64 decode failed: {}", e))?;
    Ok(bs58::encode(bytes).into_string())
}

/// Submit a Loopscale Solana transaction via onchainos.
/// `b64_tx` — base64-encoded transaction from the Loopscale API.
/// `vault_or_program` — the target program/vault address (for --to).
///
/// CRITICAL: Solana blockhash expires ~60 seconds — call immediately after receiving tx.
pub async fn submit_solana_tx(b64_tx: &str, vault_or_program: &str, dry_run: bool) -> anyhow::Result<Value> {
    let b58_tx = base64_to_base58(b64_tx)?;

    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "unsigned_tx_base58_preview": &b58_tx[..b58_tx.len().min(40)],
            "note": "dry_run=true; transaction not broadcast"
        }));
    }

    let output = Command::new("onchainos")
        .args([
            "wallet", "contract-call",
            "--chain", "501",
            "--to", vault_or_program,
            "--unsigned-tx", &b58_tx,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos response: {}. stdout: {}", e, stdout))?;
    Ok(result)
}

/// Extract txHash from contract-call response or return an error.
/// Checks result["ok"] first; errors out with message on failure.
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("contract-call failed: {}", err_msg));
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response: {}", result))
}
