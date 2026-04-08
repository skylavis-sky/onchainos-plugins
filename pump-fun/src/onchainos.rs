use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::process::Command;
use serde_json::Value;

/// Resolve the current Solana wallet address (base58) via onchainos.
///
/// onchainos wallet balance --chain 501 --output json
/// → json["data"]["address"]  (base58 pubkey)
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // onchainos wallet balance --chain 501 response shape:
    // { "ok": true, "data": { "details": [{ "tokenAssets": [{ "address": "<base58>", ... }] }] } }
    let addr = json["data"]["details"][0]["tokenAssets"][0]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve Solana wallet address. Make sure onchainos is logged in.");
    }
    Ok(addr)
}

/// Submit a pre-built, base64-encoded VersionedTransaction to Solana via onchainos.
///
/// ⚠️  Solana blockhash expires in ~60 seconds. Call this immediately after building the tx.
/// ⚠️  --force is required for wallet contract-call on Solana (onchainos won't broadcast without it).
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

    // onchainos --unsigned-tx expects base58; pump-fun builds base64-encoded VersionedTransaction
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
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("onchainos returned non-JSON: {stdout}\n{e}"))?;
    Ok(result)
}

/// Extract the txHash from an onchainos wallet contract-call response.
/// Response shape: { "ok": true, "data": { "txHash": "<sig>" } }
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
