use serde_json::Value;
use std::process::Command;

/// Resolve the current EVM wallet address via onchainos.
///
/// Uses: onchainos wallet balance --chain 56
/// Returns the wallet address from data.details[0].tokenAssets[0].address
pub fn resolve_wallet_evm() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "56"])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos balance output: {e}\nOutput: {stdout}"))?;

    let addr = json["data"]["details"][0]["tokenAssets"][0]["address"]
        .as_str()
        .unwrap_or("")
        .to_string();

    if addr.is_empty() {
        anyhow::bail!(
            "Could not resolve EVM wallet address. Make sure onchainos is logged in.\nResponse: {}",
            stdout
        );
    }

    Ok(addr)
}

/// Execute a contract call (write) on BSC via onchainos.
///
/// - `to`: contract address
/// - `input_data`: hex-encoded calldata (0x-prefixed)
/// - `value_wei`: native BNB to send in wei (0 for non-payable calls)
/// - `dry_run`: if true, return early without calling onchainos
pub async fn wallet_contract_call_evm(
    to: &str,
    input_data: &str,
    value_wei: u128,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "" }
        }));
    }

    let value_str = value_wei.to_string();

    let mut args = vec![
        "wallet",
        "contract-call",
        "--chain",
        "56",
        "--to",
        to,
        "--input-data",
        input_data,
        "--force",
    ];

    // Only add --amt if value > 0 (payable calls)
    if value_wei > 0 {
        args.push("--amt");
        args.push(&value_str);
    }

    let output = Command::new("onchainos").args(&args).output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let result: Value = serde_json::from_str(&stdout).map_err(|e| {
        anyhow::anyhow!(
            "onchainos returned non-JSON: {stdout}\nstderr: {stderr}\nparse error: {e}"
        )
    })?;

    Ok(result)
}

/// Extract txHash from an onchainos contract-call response.
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// Make an eth_call to a BSC contract via direct JSON-RPC (not onchainos).
/// Returns the raw hex-encoded return data.
pub async fn eth_call(
    rpc_url: &str,
    to: &str,
    calldata: &str,
) -> anyhow::Result<String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            {
                "to": to,
                "data": calldata
            },
            "latest"
        ],
        "id": 1
    });

    let client = reqwest::Client::new();
    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call RPC error: {}", err);
    }

    let result = resp["result"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("eth_call missing result field: {}", resp))?
        .to_string();

    Ok(result)
}
