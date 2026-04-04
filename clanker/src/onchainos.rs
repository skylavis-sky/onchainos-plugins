// src/onchainos.rs — onchainos CLI wrapper (verified against v2.2.6)
use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in wallet address via `wallet balance --output json`.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Call `onchainos wallet contract-call`.
///
/// ⚠️ dry_run=true returns a simulated response immediately — contract-call does NOT
///    accept --dry-run and would fail if we passed it.
/// ⚠️ Add --force for DEX/reward operations to prevent "pending" txHash.
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
            "calldata": input_data,
            "to": to
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

    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_owned;
    if let Some(f) = from {
        from_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_owned]);
    }

    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from `wallet contract-call` response.
/// Response shape: {"ok":true,"data":{"txHash":"0x..."}}
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}

/// Run `onchainos security token-scan` and return the parsed JSON result.
pub fn security_token_scan(chain_id: u64, token_addr: &str) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args([
            "security",
            "token-scan",
            "--address",
            token_addr,
            "--chain",
            &chain_str,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Run `onchainos token info` for a contract address.
pub fn token_info(chain_id: u64, token_addr: &str) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args([
            "token",
            "info",
            "--address",
            token_addr,
            "--chain",
            &chain_str,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Run `onchainos token price-info` for a contract address.
pub fn token_price_info(chain_id: u64, token_addr: &str) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args([
            "token",
            "price-info",
            "--address",
            token_addr,
            "--chain",
            &chain_str,
        ])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Run `onchainos wallet status` and return JSON.
pub fn wallet_status() -> anyhow::Result<Value> {
    let output = Command::new("onchainos")
        .args(["wallet", "status"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Run `onchainos wallet addresses` and return the first EVM address.
pub fn wallet_addresses() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "addresses"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)?;
    Ok(json["data"]["evm"]
        .get(0)
        .and_then(|v| v["address"].as_str())
        .unwrap_or("")
        .to_string())
}
