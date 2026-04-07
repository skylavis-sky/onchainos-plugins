use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build a base Command for onchainos, explicitly adding ~/.local/bin to PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return its stdout as a parsed JSON Value.
/// Handles exit code 2 (onchainos confirming response) transparently:
/// if the first call returns confirming=true, automatically retries with --force.
fn run_cmd(mut cmd: Command) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    // Exit code 2 = onchainos confirming response — re-run with --force
    if exit_code == 2 {
        let confirming: Value = serde_json::from_str(stdout.trim())
            .unwrap_or(serde_json::json!({"confirming": true}));
        if confirming
            .get("confirming")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            let mut force_cmd = cmd;
            force_cmd.arg("--force");
            let force_output = force_cmd
                .output()
                .context("Failed to spawn onchainos --force process")?;
            let force_stdout = String::from_utf8_lossy(&force_output.stdout);
            return serde_json::from_str(force_stdout.trim()).with_context(|| {
                format!(
                    "Failed to parse onchainos --force JSON output: {}",
                    force_stdout.trim()
                )
            });
        }
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!(
            "onchainos exited with status {}: stderr={} stdout={}",
            exit_code,
            stderr.trim(),
            stdout.trim()
        );
    }
    serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse onchainos JSON output: {}", stdout.trim()))
}

/// Submit a contract call via onchainos wallet contract-call.
///
/// - dry_run=true: prints command and returns a mock success JSON without executing.
/// - wallet contract-call does NOT accept --dry-run; handled here in the wrapper.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let cmd_str = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} {}",
            chain_id,
            to,
            input_data,
            from.map(|f| format!("--from {}", f)).unwrap_or_default()
        );
        eprintln!("[dry-run] would execute: {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" }
        }));
    }

    let mut args: Vec<String> = vec![
        "wallet".to_string(),
        "contract-call".to_string(),
        "--chain".to_string(),
        chain_id.to_string(),
        "--to".to_string(),
        to.to_string(),
        "--input-data".to_string(),
        input_data.to_string(),
    ];
    if let Some(addr) = from {
        args.push("--from".to_string());
        args.push(addr.to_string());
    }
    let mut cmd = base_cmd();
    cmd.args(&args);
    run_cmd(cmd)
}

/// Extract txHash from onchainos response, returning an error if the call failed.
///
/// Response shapes:
///   { "ok": true, "data": { "txHash": "0x..." } }
///   { "ok": true, "txHash": "0x..." }
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"]
            .as_str()
            .or_else(|| result["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("contract-call failed: {}", err_msg));
    }
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no txHash in response"))
}

/// Approve an ERC-20 token spend via wallet contract-call.
/// approve(address,uint256) selector: 0x095ea7b3
pub fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender);
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, dry_run)
}

/// Get the currently active wallet address for a given chain.
/// Uses wallet balance (no --output json) and reads data.details[0].tokenAssets[0].address.
/// Falls back to data.address.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let mut cmd = base_cmd();
    // Note: Do NOT add --output json — not supported on all chains (chain 1 fails)
    cmd.args(["wallet", "balance", "--chain", &chain_str]);
    let output = cmd.output().context("Failed to spawn onchainos wallet balance")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse wallet balance output: {}", stdout.trim()))?;

    // Primary: data.details[0].tokenAssets[0].address
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
        anyhow::bail!("Could not resolve wallet address from onchainos wallet balance");
    }
    Ok(addr)
}
