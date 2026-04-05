use std::process::Command;
use serde_json::Value;

/// Get the currently logged-in wallet address for the given chain.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Submit a transaction via `onchainos wallet contract-call`.
///
/// dry_run=true returns a simulated response without calling onchainos.
/// NOTE: `onchainos wallet contract-call` does NOT accept a --dry-run flag.
/// The DEX/farming operations require --force to avoid "txHash: pending" non-broadcast.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "calldata": input_data
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_str.clone(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];

    if let Some(f) = from {
        args.push("--from".to_string());
        args.push(f.to_string());
    }

    if let Some(v) = amt {
        args.push("--amt".to_string());
        args.push(v.to_string());
    }

    if force {
        args.push("--force".to_string());
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)
        .unwrap_or_else(|_| serde_json::json!({ "ok": false, "error": stdout.to_string() })))
}

/// Extract txHash from a wallet contract-call response.
/// Response shape: {"ok":true,"data":{"txHash":"0x..."}}
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
