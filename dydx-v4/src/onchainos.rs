use serde_json::Value;

const BRIDGE_CONTRACT: &str = "0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9";

/// Call `onchainos wallet contract-call` and return parsed JSON output.
/// Always uses --force to broadcast on non-dry-run paths.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let mut args: Vec<&str> = vec![
        "wallet",
        "contract-call",
        "--chain",
        &chain_str,
        "--to",
        to,
        "--input-data",
        input_data,
    ];

    if dry_run {
        eprintln!(
            "[dydx-v4] [dry-run] Would run: onchainos {}",
            args.join(" ")
        );
        return Ok(serde_json::json!({
            "ok": true,
            "data": {
                "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
            }
        }));
    }

    // --force is required to broadcast real txs
    args.push("--force");

    let output = tokio::process::Command::new("onchainos")
        .args(&args)
        .output()
        .await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {} — raw: {}", e, stdout))?;
    Ok(parsed)
}

/// Execute the DYDX bridge deposit on Ethereum mainnet.
pub async fn bridge_deposit(input_data: &str, dry_run: bool) -> anyhow::Result<Value> {
    wallet_contract_call(1, BRIDGE_CONTRACT, input_data, dry_run).await
}

/// Extract txHash from wallet contract-call response.
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}
