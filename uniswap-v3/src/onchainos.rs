/// Wrapper for `onchainos wallet contract-call` CLI.
/// Follows the exact template from the developer agent rules.

use std::process::Command;
use serde_json::Value;

/// Resolve wallet address for the given chain via `onchainos wallet balance`.
/// Returns the wallet's EVM address.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Execute `onchainos wallet contract-call` for a write operation.
///
/// # dry_run guard
/// When `dry_run` is true, this function returns immediately with a synthetic
/// response containing a zero txHash. The guard MUST be checked before calling
/// `resolve_wallet()` — wallet resolution fails on unlogged chains.
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
    ];

    let amt_str: String;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }

    let from_str: String;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }

    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("onchainos returned empty output. stderr: {}", stderr);
    }

    let v: Value = serde_json::from_str(&stdout)?;
    if v.get("ok").and_then(|b| b.as_bool()) == Some(false) {
        let msg = v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("unknown onchainos error");
        eprintln!("  [onchainos error] {}", msg);
    }
    Ok(v)
}

/// Extract txHash from onchainos response, checking multiple possible locations.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Poll `eth_getTransactionReceipt` until the tx is confirmed (status == "0x1").
/// Uses the given RPC URL. Polls every 3 seconds, times out after ~2 minutes.
pub async fn wait_for_tx(tx_hash: &str, rpc_url: &str) -> anyhow::Result<()> {
    if tx_hash == "pending"
        || tx_hash == "0x0000000000000000000000000000000000000000000000000000000000000000"
    {
        return Ok(());
    }

    let client = crate::rpc::build_client();
    for _ in 0..40 {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;

        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "eth_getTransactionReceipt",
            "params": [tx_hash],
            "id": 1
        });

        match client.post(rpc_url).json(&body).send().await {
            Ok(resp) => {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    if !v["result"].is_null() {
                        let status = v["result"]["status"].as_str().unwrap_or("0x0");
                        if status == "0x1" {
                            return Ok(());
                        } else {
                            anyhow::bail!("Transaction {} failed (status: {})", tx_hash, status);
                        }
                    }
                }
            }
            Err(_) => {}
        }
    }

    // Timeout — proceed anyway; the tx may still confirm
    eprintln!("  [warn] Timed out waiting for tx {} — proceeding anyway", tx_hash);
    Ok(())
}
