use std::process::Command;
use serde_json::Value;

/// Resolve the wallet address for a given chain using onchainos wallet balance.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {e}\nRaw: {stdout}"))?;

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
    let addr = json["data"]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address from onchainos on chain {chain_id}");
    }
    Ok(addr)
}

/// Execute a wallet contract-call via onchainos CLI.
///
/// * `chain_id`   — numeric chain ID (42161, 1, etc.)
/// * `to`         — contract address
/// * `input_data` — hex calldata (0x-prefixed)
/// * `amt`        — optional ETH value in wei
/// * `dry_run`    — if true, skip actual broadcast and return dummy response
pub async fn wallet_contract_call(
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
    Ok(serde_json::from_str(&stdout).unwrap_or_else(|_| {
        serde_json::json!({"ok": false, "error": stdout.to_string()})
    }))
}

/// Extract txHash from a contract-call result.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Check if the result indicates success.
pub fn is_ok(result: &Value) -> bool {
    result["ok"].as_bool().unwrap_or(false)
}

/// Build a reqwest client that honours the HTTPS_PROXY environment variable.
pub fn build_client() -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY") {
        builder = builder.proxy(reqwest::Proxy::https(&proxy_url)?);
    } else if let Ok(proxy_url) = std::env::var("https_proxy") {
        builder = builder.proxy(reqwest::Proxy::https(&proxy_url)?);
    }
    Ok(builder.build()?)
}
