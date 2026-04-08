use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build a base Command for onchainos, ensuring ~/.local/bin is on PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command, returning its stdout as a parsed JSON Value.
///
/// Handles exit code 2 (onchainos confirming response) by automatically
/// retrying with --force, per onchainos CLI behavior.
fn run_cmd(mut cmd: Command) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    // Exit code 2 = onchainos confirming mode — retry with --force
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
                    "Failed to parse onchainos --force JSON: {}",
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

    serde_json::from_str(stdout.trim()).with_context(|| {
        format!(
            "Failed to parse onchainos JSON output: {}",
            stdout.trim()
        )
    })
}

/// Submit a contract-call transaction via onchainos wallet contract-call.
///
/// Critical rules:
/// - Uses --input-data (not --calldata)
/// - Uses --force on every invocation
/// - dry_run: returns a simulated response immediately; does NOT pass --dry-run to onchainos
///
/// Chain 1 (Ethereum) is used for all Archimedes calls.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let mut args: Vec<String> = vec![
        "wallet".into(),
        "contract-call".into(),
        "--chain".into(),
        chain_str,
        "--to".into(),
        to.to_string(),
        "--input-data".into(),
        input_data.to_string(),
        "--force".into(),
    ];
    if let Some(addr) = from {
        args.push("--from".into());
        args.push(addr.to_string());
    }

    if dry_run {
        let cmd_str = format!("onchainos {}", args.join(" "));
        eprintln!("[dry-run] would execute: {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str
        }));
    }

    let mut cmd = base_cmd();
    cmd.args(&args);
    run_cmd(cmd)
}

/// Resolve the active wallet address.
///
/// onchainos wallet balance --chain 1 (no --output json for chain 1)
/// Address is at data.details[0].tokenAssets[0].address
pub fn resolve_wallet() -> anyhow::Result<String> {
    let mut cmd = base_cmd();
    // For chain 1, wallet balance does NOT accept --output json (causes EOF failure)
    // Use wallet status instead
    cmd.args(["wallet", "balance", "--chain", "1"]);
    let result = run_cmd(cmd)?;

    // Try data.details[0].tokenAssets[0].address
    if let Some(addr) = result
        .pointer("/data/details/0/tokenAssets/0/address")
        .and_then(|v| v.as_str())
    {
        return Ok(addr.to_string());
    }

    // Fallback: try data.address or address at top level
    if let Some(addr) = result
        .pointer("/data/address")
        .or_else(|| result.get("address"))
        .and_then(|v| v.as_str())
    {
        return Ok(addr.to_string());
    }

    anyhow::bail!(
        "Could not resolve wallet address from onchainos wallet balance. \
         Make sure you are logged in: onchainos wallet login"
    )
}

/// Extract txHash from an onchainos contract-call response.
/// Tries multiple known JSON paths.
pub fn extract_tx_hash(result: &Value) -> String {
    result
        .pointer("/data/txHash")
        .or_else(|| result.pointer("/data/hash"))
        .or_else(|| result.get("txHash"))
        .or_else(|| result.get("hash"))
        .and_then(|v| v.as_str())
        .unwrap_or("pending")
        .to_string()
}
