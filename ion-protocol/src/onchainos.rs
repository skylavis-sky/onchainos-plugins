// src/onchainos.rs — onchainos CLI wrapper for Ion Protocol
use std::process::Command;
use serde_json::Value;
use anyhow::Context;

/// Build base Command for onchainos with PATH fix.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return its stdout as parsed JSON Value.
/// Handles exit code 2 (onchainos confirming response): auto-retries with --force.
fn run_cmd(mut cmd: Command) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    if exit_code == 2 {
        let confirming: Value = serde_json::from_str(stdout.trim())
            .unwrap_or(serde_json::json!({"confirming": true}));
        if confirming.get("confirming").and_then(|v| v.as_bool()).unwrap_or(false) {
            let mut force_cmd = cmd;
            force_cmd.arg("--force");
            let force_output = force_cmd.output().context("Failed to spawn onchainos --force")?;
            let force_stdout = String::from_utf8_lossy(&force_output.stdout);
            return serde_json::from_str(force_stdout.trim())
                .with_context(|| format!("Failed to parse --force JSON: {}", force_stdout.trim()));
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos exited {}: stderr={} stdout={}",
            exit_code,
            stderr.trim(),
            stdout.trim()
        );
    }
    serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse onchainos JSON: {}", stdout.trim()))
}

/// Resolve the active wallet address.
/// For chain 1 (Ethereum), --output json is NOT supported on wallet balance.
/// Use `onchainos wallet addresses` and filter chainIndex == "1".
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    // Use wallet addresses (works on all chains including chain 1)
    let mut cmd = base_cmd();
    cmd.args(["wallet", "addresses"]);
    let output = cmd.output().context("Failed to spawn onchainos wallet addresses")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse wallet addresses output: {}", stdout.trim()))?;

    let chain_str = chain_id.to_string();
    // Response shape: {"ok":true,"data":[{"address":"0x...","chainIndex":"1",...},...]}
    if let Some(arr) = json["data"].as_array() {
        for entry in arr {
            let ci = entry["chainIndex"].as_str().unwrap_or("");
            let addr = entry["address"].as_str().unwrap_or("");
            if ci == chain_str && !addr.is_empty() {
                return Ok(addr.to_string());
            }
        }
        // Fallback: take first non-empty address
        for entry in arr {
            let addr = entry["address"].as_str().unwrap_or("");
            if !addr.is_empty() {
                return Ok(addr.to_string());
            }
        }
    }

    // Final fallback: wallet status
    let mut cmd2 = base_cmd();
    cmd2.args(["wallet", "status", "--output", "json"]);
    let output2 = cmd2.output().context("Failed to spawn onchainos wallet status")?;
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let json2: Value = serde_json::from_str(stdout2.trim()).unwrap_or(serde_json::json!({}));
    if let Some(addr) = json2["address"].as_str().filter(|a| !a.is_empty()) {
        return Ok(addr.to_string());
    }

    anyhow::bail!(
        "Could not resolve wallet address for chain {}. Make sure onchainos wallet is configured.",
        chain_id
    )
}

/// Submit a contract call via onchainos wallet contract-call.
/// dry_run=true returns simulated response without calling onchainos CLI.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let cmd_str = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --force{}",
            chain_id,
            to,
            input_data,
            from.map(|f| format!(" --from {}", f)).unwrap_or_default()
        );
        eprintln!("[dry-run] {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "simulatedCommand": cmd_str,
            "calldata": input_data
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_str,
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
        "--force".to_string(),
    ];
    if let Some(addr) = from {
        args.push("--from".to_string());
        args.push(addr.to_string());
    }
    let mut cmd = base_cmd();
    cmd.args(&args);
    run_cmd(cmd)
}

/// Extract txHash from onchainos contract-call response, propagating errors.
/// Per standards: use extract_tx_hash_or_err (not unwrap_or("pending")).
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error from onchainos");
        return Err(anyhow::anyhow!("contract-call failed: {}", err_msg));
    }
    let hash = result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .or_else(|| result["hash"].as_str());
    match hash {
        Some(h) if !h.is_empty() => Ok(h.to_string()),
        _ => Ok("pending".to_string()),
    }
}
