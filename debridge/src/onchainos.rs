use std::process::Command;
use serde_json::Value;

// ---------------------------------------------------------------------------
// Wallet address resolution
// ---------------------------------------------------------------------------

/// Resolve the wallet address for an EVM chain via onchainos.
pub fn resolve_wallet_evm(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
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
    // Fallback path: data.address
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Resolve the wallet address for Solana via onchainos.
/// NOTE: --output json must NOT be passed for chain 501 (causes EOF failure).
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Could not resolve Solana wallet address from onchainos")
}

// ---------------------------------------------------------------------------
// Encoding helpers
// ---------------------------------------------------------------------------

/// Convert a hex-encoded Solana VersionedTransaction (from deBridge API)
/// to base58, which is required by onchainos --unsigned-tx.
/// Input may optionally have a leading "0x" prefix.
pub fn hex_to_base58(hex_tx: &str) -> anyhow::Result<String> {
    let hex_str = hex_tx.trim_start_matches("0x");
    let bytes = hex::decode(hex_str)?;
    Ok(bs58::encode(&bytes).into_string())
}

/// Build approve(spender, amount) calldata for ERC-20.
/// Returns hex string with 0x prefix, padded to 68 bytes (4 + 32 + 32).
pub fn encode_approve(spender: &str, amount_hex: &str) -> String {
    // Strip leading 0x from both addresses
    let spender_clean = spender.trim_start_matches("0x");
    let amount_clean = amount_hex.trim_start_matches("0x");
    // Pad to 32 bytes each
    let spender_padded = format!("{:0>64}", spender_clean.to_lowercase());
    let amount_padded = format!("{:0>64}", amount_clean.to_lowercase());
    format!("0x095ea7b3{}{}", spender_padded, amount_padded)
}

// ---------------------------------------------------------------------------
// onchainos wallet contract-call wrappers
// ---------------------------------------------------------------------------

/// Submit an EVM contract call via onchainos.
pub async fn wallet_contract_call_evm(
    chain_id: u64,
    to: &str,
    input_data: &str,
    amt: Option<u128>,
    dry_run: bool,
) -> anyhow::Result<Value> {
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
        "--force",
    ];
    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }
    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({"ok": false, "error": stdout.to_string()})))
}

/// Submit a Solana transaction via onchainos.
/// unsigned_tx_base58: base58-encoded VersionedTransaction (convert from hex first).
pub async fn wallet_contract_call_solana(
    to: &str,
    unsigned_tx_base58: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" }
        }));
    }
    let output = Command::new("onchainos")
        .args([
            "wallet",
            "contract-call",
            "--chain",
            "501",
            "--to",
            to,
            "--unsigned-tx",
            unsigned_tx_base58,
            "--force",
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({"ok": false, "error": stdout.to_string()})))
}

// ---------------------------------------------------------------------------
// Result helpers
// ---------------------------------------------------------------------------

/// Extract txHash from an onchainos result.
/// Checks data.swapTxHash first (Solana), then data.txHash, then txHash.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
