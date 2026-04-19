use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use serde_json::Value;
use std::process::Command;

/// Resolve the current logged-in Solana wallet address (base58 pubkey).
/// Uses `onchainos wallet balance --chain 501` — NOTE: no --output json (Solana returns JSON natively).
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // path: data.details[0].tokenAssets[0].address
    let addr = json["data"]["details"][0]["tokenAssets"][0]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve Solana wallet address from onchainos");
    }
    Ok(addr)
}

/// Submit a Solana unsigned transaction via onchainos wallet contract-call.
///
/// `unsigned_tx_base64`: the base64-encoded transaction from Jupiter API.
/// onchainos --unsigned-tx expects base58, so we convert: base64 -> bytes -> base58.
///
/// dry_run=true: return a simulated response without calling onchainos.
/// NOTE: Solana blockhash expires in ~60s — call immediately after receiving the tx from Jupiter.
pub fn wallet_contract_call_solana(
    unsigned_tx_base64: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" },
            "serialized_tx": unsigned_tx_base64
        }));
    }

    // onchainos --unsigned-tx expects base58; Jupiter API returns base64
    let tx_bytes = BASE64
        .decode(unsigned_tx_base64)
        .map_err(|e| anyhow::anyhow!("Failed to decode base64 tx: {}", e))?;
    let tx_base58 = bs58::encode(&tx_bytes).into_string();

    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--unsigned-tx",
            &tx_base58,
            "--to",
            "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",
            "--chain",
            "501",
            "--force",
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let result: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}\nraw: {}", e, stdout))?;
    Ok(result)
}

/// Extract txHash from an onchainos response Value.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
