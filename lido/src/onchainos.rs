// src/onchainos.rs — onchainos CLI wrapper
use std::process::Command;
use serde_json::Value;

/// Resolve wallet address from onchainos wallet balance
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str, "--output", "json"])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Call onchainos wallet contract-call
/// dry_run=true returns simulated response without calling onchainos.
/// NOTE: onchainos wallet contract-call does NOT support --dry-run flag.
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u128>, // wei value for ETH-valued calls
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
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
        "--force",
    ];

    let amt_str;
    if let Some(v) = amt {
        amt_str = v.to_string();
        args.extend_from_slice(&["--amt", &amt_str]);
    }
    let from_str;
    if let Some(f) = from {
        from_str = f.to_string();
        args.extend_from_slice(&["--from", &from_str]);
    }

    let output = Command::new("onchainos").args(&args).output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(serde_json::from_str(&stdout)?)
}

/// Extract txHash from wallet contract-call response
/// Response shape: {"ok":true,"data":{"txHash":"0x..."}}
pub fn extract_tx_hash(result: &Value) -> &str {
    result["data"]["txHash"]
        .as_str()
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
}

/// ERC-20 approve — manually encode approve(address,uint256) calldata
/// selector: 0x095ea7b3
pub async fn erc20_approve(
    chain_id: u64,
    token_addr: &str,
    spender: &str,
    amount: u128, // u128::MAX for unlimited
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let spender_clean = spender.trim_start_matches("0x");
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    let calldata = format!("0x095ea7b3{}{}", spender_padded, amount_hex);
    wallet_contract_call(chain_id, token_addr, &calldata, from, None, dry_run).await
}

/// Query ERC-20 allowance via eth_call
pub async fn erc20_allowance(
    chain_id: u64,
    token_addr: &str,
    owner: &str,
    spender: &str,
    rpc_url: &str,
) -> anyhow::Result<u128> {
    // allowance(address,address) selector: 0xdd62ed3e
    let owner_clean = owner.trim_start_matches("0x");
    let spender_clean = spender.trim_start_matches("0x");
    let data = format!(
        "0xdd62ed3e{:0>64}{:0>64}",
        owner_clean, spender_clean
    );
    let result = crate::rpc::eth_call(token_addr, &data, rpc_url).await?;
    let hex = result.trim_start_matches("0x");
    if hex.is_empty() {
        return Ok(0);
    }
    Ok(u128::from_str_radix(&hex[hex.len().saturating_sub(32)..], 16).unwrap_or(0))
}

/// wallet balance
pub fn wallet_balance(chain_id: u64) -> anyhow::Result<Value> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    Ok(serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?)
}
