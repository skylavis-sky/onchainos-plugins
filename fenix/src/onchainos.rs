// onchainos.rs — onchainos CLI wrapper for Fenix Finance plugin
use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build a base Command for onchainos, adding ~/.local/bin to PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return its stdout as a parsed JSON Value.
/// Handles exit code 2 (onchainos confirming response) by retrying with --force.
fn run_cmd(cmd: Command) -> anyhow::Result<Value> {
    run_cmd_inner(cmd, false)
}

fn run_cmd_inner(mut cmd: Command, already_forced: bool) -> anyhow::Result<Value> {
    let output = cmd.output().context("Failed to spawn onchainos process")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let exit_code = output.status.code().unwrap_or(-1);

    // Exit code 2 = confirming response — re-run with --force (once)
    if exit_code == 2 && !already_forced {
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

    if !output.status.success() && exit_code != 2 {
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

/// Resolve the active wallet address from onchainos wallet balance.
/// NOTE: For EVM chains, wallet balance returns JSON natively without --output json.
/// Parses data.details[0].tokenAssets[0].address
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let mut cmd = base_cmd();
    // No --output json for EVM chains (per known constraints)
    cmd.args(["wallet", "balance", "--chain", &chain_str]);
    let output = cmd.output().context("Failed to spawn onchainos wallet balance")?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    let json: Value = serde_json::from_str(stdout.trim())
        .with_context(|| format!("Failed to parse wallet balance output: {}", stdout.trim()))?;

    // Try multiple paths to find the address
    if let Some(addr) = json["data"]["details"]
        .as_array()
        .and_then(|d| d.first())
        .and_then(|d| d["tokenAssets"].as_array())
        .and_then(|t| t.first())
        .and_then(|t| t["address"].as_str())
    {
        return Ok(addr.to_string());
    }

    // Fallback: data.address
    if let Some(addr) = json["data"]["address"].as_str() {
        return Ok(addr.to_string());
    }

    anyhow::bail!("Cannot resolve wallet address from onchainos wallet balance output")
}

/// Submit a contract call via onchainos wallet contract-call.
/// dry_run: returns simulated response without calling onchainos.
/// NOTE: do NOT pass --dry-run to onchainos; handle dry_run in plugin code only.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    force: bool,
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
    let mut cmd = base_cmd();
    cmd.args([
        "wallet",
        "contract-call",
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
    ]);
    if let Some(f) = from {
        cmd.args(["--from", f]);
    }
    if force {
        cmd.arg("--force");
    }

    run_cmd(cmd)
}

/// Extract txHash from wallet contract-call response
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// ERC-20 approve via wallet contract-call.
/// Encodes approve(address,uint256) = selector 0x095ea7b3 + spender(32) + amount(32)
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let calldata = crate::abi::encode_approve(spender, u128::MAX);
    // approve does not need --force
    wallet_contract_call(chain_id, token_addr, &calldata, None, false, dry_run).await
}
