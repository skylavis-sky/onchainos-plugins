use std::process::Command;
use serde_json::Value;

/// Resolve the current logged-in wallet address using wallet balance.
/// Chain 501 (Solana) cannot use --output json; EVM chains work fine.
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    Ok(json["data"]["address"]
        .as_str()
        .unwrap_or("")
        .to_string())
}

/// Submit a transaction via `onchainos wallet contract-call`.
/// dry_run=true returns a simulated response; never passes --dry-run to onchainos CLI
/// (unsupported flag per KNOWLEDGE_HUB).
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    value: Option<u64>,
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

    let value_str;
    if let Some(v) = value {
        value_str = v.to_string();
        args.extend_from_slice(&["--value", &value_str]);
    }

    let from_owned;
    if let Some(f) = from {
        from_owned = f.to_string();
        args.extend_from_slice(&["--from", &from_owned]);
    }

    if force {
        args.push("--force");
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: Value = serde_json::from_str(&stdout).unwrap_or_else(|_| {
        serde_json::json!({"ok": false, "raw": stdout.to_string()})
    });
    Ok(parsed)
}

/// Extract txHash from onchainos response, propagating errors.
pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String> {
    if result["ok"].as_bool() != Some(true) {
        let err_msg = result["error"].as_str()
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

/// Build ERC-20 approve calldata (max uint256) and submit via wallet contract-call.
/// approve(address,uint256) selector = 0x095ea7b3
/// Always uses --force (force=true) for approve to ensure approval tx is broadcast.
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender).to_lowercase();
    let spender_padded = format!("{:0>64}", spender_clean);
    // max uint256
    let amount_hex = "f".repeat(64);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, true, dry_run).await
}

/// Perform an eth_call (read-only) against the given RPC endpoint.
pub async fn eth_call(rpc_url: &str, to: &str, data: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{"to": to, "data": data}, "latest"],
        "id": 1
    });
    let resp: Value = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json()
        .await?;
    if let Some(err) = resp.get("error") {
        anyhow::bail!("eth_call error: {}", err);
    }
    Ok(resp["result"].as_str().unwrap_or("0x").to_string())
}

/// Decode a 32-byte hex result to u128 (handles 0x prefix)
pub fn decode_uint(hex: &str) -> u128 {
    let clean = hex.strip_prefix("0x").unwrap_or(hex);
    u128::from_str_radix(&clean[clean.len().saturating_sub(32)..], 16).unwrap_or(0)
}

/// Decode a 32-byte address result (last 20 bytes)
pub fn decode_address(hex: &str) -> String {
    let clean = hex.strip_prefix("0x").unwrap_or(hex);
    if clean.len() >= 40 {
        format!("0x{}", &clean[clean.len() - 40..])
    } else {
        format!("0x{}", clean)
    }
}

/// ABI-encode a uint256 argument (padded to 32 bytes)
pub fn encode_uint256(val: u128) -> String {
    format!("{:064x}", val)
}

/// ABI-encode an address argument (padded to 32 bytes)
pub fn encode_address(addr: &str) -> String {
    let clean = addr.strip_prefix("0x").unwrap_or(addr).to_lowercase();
    format!("{:0>64}", clean)
}
