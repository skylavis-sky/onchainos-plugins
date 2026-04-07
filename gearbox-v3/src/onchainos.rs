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
/// Handles exit code 2 (onchainos confirming response) by re-running with --force.
fn run_cmd(cmd: Command) -> anyhow::Result<Value> {
    let mut cmd = cmd;
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    // Exit code 2 = confirming response — re-run with --force
    if exit_code == 2 {
        let confirming: Value = serde_json::from_str(stdout.trim())
            .unwrap_or(serde_json::json!({"confirming": true}));
        if confirming.get("confirming").and_then(|v| v.as_bool()).unwrap_or(false) {
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

/// Resolve the active wallet address for a given chain.
/// Uses `wallet balance --chain <id>` (no --output json — not supported on all chains).
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let mut cmd = base_cmd();
    cmd.args(["wallet", "balance", "--chain", &chain_str]);
    let result = run_cmd(cmd)?;

    // Try data.details[0].tokenAssets[0].address first (most reliable)
    if let Some(addr) = result["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    // Fallback: data.address
    let addr = result["data"]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();
    if addr.is_empty() {
        anyhow::bail!(
            "Could not resolve wallet address from onchainos wallet balance --chain {}",
            chain_id
        );
    }
    Ok(addr)
}

/// Submit a contract call via `onchainos wallet contract-call`.
/// dry_run=true returns a simulated response without calling onchainos.
/// Note: onchainos does NOT accept --dry-run — handle entirely in-process.
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
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }
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

/// Extract tx hash from onchainos contract-call response, returning an error on failure.
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
        .ok_or_else(|| anyhow::anyhow!("no txHash in contract-call response"))
}

/// ERC-20 approve via wallet contract-call.
/// ⚠️ Approve must target CreditManagerV3, NOT CreditFacadeV3 (Gearbox-specific).
pub fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str, // CreditManagerV3 address
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let calldata = crate::abi::encode_erc20_approve(spender, amount)
        .context("Failed to encode approve calldata")?;
    let calldata_hex = format!("0x{}", hex::encode(&calldata));
    wallet_contract_call(chain_id, token_addr, &calldata_hex, from, dry_run)
}
