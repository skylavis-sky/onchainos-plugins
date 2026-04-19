/// onchainos CLI wrappers for the 1inch plugin.
///
/// Follows the standard EVM onchainos integration pattern:
/// - resolve_wallet()        — get connected wallet address
/// - wallet_contract_call()  — broadcast a write tx (approve or swap)
/// - extract_tx_hash()       — parse txHash from onchainos JSON response
/// - wait_for_tx()           — poll onchainos wallet status until confirmed

use std::process::Command;
use serde_json::Value;

/// Resolve the connected wallet address for the given EVM chain via `onchainos wallet balance`.
///
/// Address is at `.data.details[0].tokenAssets[0].address` in the JSON output.
/// Falls back to `.data.address` if the nested path is absent.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos wallet balance output: {}. stdout: {}", e, &stdout[..stdout.len().min(300)]))?;

    // Try nested path first
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
        .filter(|a| !a.is_empty())
    {
        return Ok(addr.to_string());
    }

    // Fallback to top-level address field
    if let Some(addr) = json["data"]["address"].as_str().filter(|a| !a.is_empty()) {
        return Ok(addr.to_string());
    }

    anyhow::bail!(
        "Could not resolve wallet address from onchainos output. Make sure you are logged in with: onchainos wallet login"
    )
}

/// Broadcast a write transaction via `onchainos wallet contract-call`.
///
/// # dry_run
/// When `dry_run` is true, returns a synthetic response immediately without calling onchainos.
/// The dry_run guard MUST be checked BEFORE calling resolve_wallet() — wallet resolution
/// will fail when no wallet is logged in.
///
/// # value_wei
/// Optional native token value to send with the tx (e.g. ETH amount for ETH→token swaps).
/// Pass `Some("0")` or `None` for token→token swaps.
pub fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    value_wei: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            },
            "calldata": input_data,
            "to": to
        }));
    }

    let chain_str = chain_id.to_string();
    let mut args: Vec<&str> = vec![
        "wallet", "contract-call",
        "--chain", &chain_str,
        "--to", to,
        "--input-data", input_data,
        "--force",
    ];

    // Only add --amt if value > 0
    let should_add_value = value_wei
        .map(|v| v != "0" && !v.is_empty())
        .unwrap_or(false);

    let value_str: String;
    if should_add_value {
        value_str = value_wei.unwrap().to_string();
        args.extend_from_slice(&["--amt", &value_str]);
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos wallet contract-call returned empty output. stderr: {}", stderr);
    }

    let v: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}. stdout: {}", e, &stdout[..stdout.len().min(300)]))?;

    if v.get("ok").and_then(|b| b.as_bool()) == Some(false) {
        let msg = v["error"].as_str().unwrap_or("unknown onchainos error");
        eprintln!("  [onchainos error] {}", msg);
    }

    Ok(v)
}

/// Extract txHash from an onchainos response.
/// Checks multiple paths to handle different response shapes.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["data"]["swapTxHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Poll `onchainos wallet status` until the tx is confirmed or the timeout is reached.
///
/// Polls every 3 seconds for up to 90 seconds (30 attempts).
/// Proceeds without error on timeout — the tx may still confirm later.
pub fn wait_for_tx(tx_hash: &str, chain_id: u64) -> anyhow::Result<()> {
    // Skip polling for dry-run zero hash
    if tx_hash == "pending"
        || tx_hash == "0x0000000000000000000000000000000000000000000000000000000000000000"
    {
        return Ok(());
    }

    eprintln!("  [info] Waiting for tx {} to confirm...", tx_hash);
    let chain_str = chain_id.to_string();

    for i in 0..30 {
        std::thread::sleep(std::time::Duration::from_secs(3));

        let output = Command::new("onchainos")
            .args(["wallet", "status", "--tx-hash", tx_hash, "--chain", &chain_str])
            .output();

        match output {
            Err(e) => {
                eprintln!("  [warn] onchainos status call failed: {}", e);
                continue;
            }
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if let Ok(json) = serde_json::from_str::<Value>(&stdout) {
                    let status = json["data"]["status"].as_str().unwrap_or("");
                    if status == "confirmed" {
                        eprintln!("  [info] Tx confirmed after {} polls.", i + 1);
                        return Ok(());
                    }
                    if status == "failed" {
                        anyhow::bail!("Transaction {} failed on-chain.", tx_hash);
                    }
                }
            }
        }
    }

    eprintln!("  [warn] Timed out waiting for tx {} — proceeding anyway.", tx_hash);
    Ok(())
}
