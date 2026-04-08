use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in Solana wallet address (base58).
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try details[0].tokenAssets[0].address first, then data.address
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        return Ok(addr.to_string());
    }
    Ok(json["data"]["address"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

/// Submit a Solana serialized transaction via onchainos.
/// serialized_tx: base64-encoded VersionedTransaction from Raydium API.
/// onchainos --unsigned-tx expects base58, so we convert here.
/// NOTE: Solana blockhash expires in ~60s — call immediately after receiving tx.
pub async fn wallet_contract_call_solana(
    to: &str,
    serialized_tx: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": serialized_tx
        }));
    }
    // onchainos --unsigned-tx expects base58; Raydium API returns base64
    let tx_bytes = BASE64.decode(serialized_tx)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    let tx_base58 = bs58::encode(&tx_bytes).into_string();

    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            to,
            "--unsigned-tx",
            &tx_base58,
            "--force", // required — without this onchainos won't broadcast
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from onchainos response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
