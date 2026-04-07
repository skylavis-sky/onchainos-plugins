/// onchainos CLI wrappers for Vertex Edge plugin.
///
/// Uses wallet contract-call for on-chain EVM operations (ERC-20 approve, depositCollateral).
/// Follows the established patterns from the DApp onboarding pipeline.

use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Build an onchainos Command with HOME/.local/bin in PATH.
fn base_cmd() -> Command {
    let mut cmd = Command::new("onchainos");
    let home = std::env::var("HOME").unwrap_or_default();
    let existing_path = std::env::var("PATH").unwrap_or_default();
    let path = format!("{}/.local/bin:{}", home, existing_path);
    cmd.env("PATH", path);
    cmd
}

/// Run a Command and return stdout as a parsed JSON Value.
/// Handles exit code 2 (onchainos confirming response) by retrying with --force.
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

/// Extract txHash from an onchainos contract-call response.
/// Checks data.txHash then falls back to root txHash.
/// Returns an error if ok != true or no txHash found.
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

/// Resolve the current wallet address on a given chain.
/// Uses wallet balance without --output json (not supported on all chains).
/// Address comes from data.details[0].tokenAssets[0].address.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let mut cmd = base_cmd();
    cmd.args(["wallet", "balance", "--chain", &chain_str]);
    let json = run_cmd(cmd)?;

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
    let addr = json["data"]["address"].as_str().unwrap_or("").to_string();
    if addr.is_empty() {
        anyhow::bail!("Could not resolve wallet address on chain {}", chain_id);
    }
    Ok(addr)
}

/// Submit an EVM contract call via onchainos wallet contract-call.
/// dry_run=true: return a simulated response without calling onchainos.
/// NOTE: onchainos does not support --dry-run flag; handle dry-run in wrapper.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        let cmd_str = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {}{}",
            chain_id,
            to,
            input_data,
            from.map(|f| format!(" --from {}", f)).unwrap_or_default()
        );
        eprintln!("[dry-run] would execute: {}", cmd_str);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "simulatedCommand": cmd_str
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

/// ERC-20 approve(spender, amount) via wallet contract-call.
/// selector: 0x095ea7b3
pub fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // approve(address,uint256) selector = 0x095ea7b3
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, dry_run)
}
