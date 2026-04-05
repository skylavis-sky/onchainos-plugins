/// onchainos CLI wrapper for Umami Finance plugin.
/// All facts verified against onchainos v2.2.6.

use std::process::Command;
use serde_json::Value;

/// Resolve current logged-in EVM wallet address for the given chain.
/// onchainos wallet balance --chain <id> returns JSON with address at:
///   data.details[0].tokenAssets[0].address
pub fn resolve_wallet(chain_id: u64) -> anyhow::Result<String> {
    let chain_str = chain_id.to_string();
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", &chain_str])
        .output()?;
    let json: Value = serde_json::from_str(&String::from_utf8_lossy(&output.stdout))?;
    // Try data.details[0].tokenAssets[0].address first
    if let Some(addr) = json["data"]["details"]
        .get(0)
        .and_then(|d| d["tokenAssets"].get(0))
        .and_then(|t| t["address"].as_str())
    {
        return Ok(addr.to_string());
    }
    // Fallback to data.address
    Ok(json["data"]["address"].as_str().unwrap_or("").to_string())
}

/// Call onchainos wallet contract-call on EVM.
/// ⚠️  dry_run=true: returns mock response immediately; onchainos does NOT support --dry-run.
/// ⚠️  force=true: adds --force flag (required for vault deposits/redeems to actually broadcast)
pub async fn wallet_contract_call(
    chain_id: u64,
    to: &str,
    input_data: &str,
    from: Option<&str>,
    amt: Option<u64>,
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
        "wallet", "contract-call",
        "--chain", &chain_str,
        "--to", to,
        "--input-data", input_data,
        "--force",   // Required to broadcast; without it onchainos returns txHash:"pending"
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
    Ok(serde_json::from_str(&stdout).unwrap_or_else(|_| serde_json::json!({"error": stdout.to_string()})))
}

/// Extract txHash from onchainos response
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}

/// ERC-20 approve calldata helper
/// approve(address,uint256) selector = 0x095ea7b3
pub fn build_approve_calldata(spender: &str, amount: u128) -> String {
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x"));
    let amount_hex = format!("{:064x}", amount);
    format!("0x095ea7b3{}{}", spender_padded, amount_hex)
}

/// Umami custom deposit(uint256 assets, uint256 minSharesOut, address receiver) calldata
/// selector: 0x8dbdbe6d (verified via cast sig + live tx analysis)
/// minSharesOut = 0 (no slippage protection, acceptable for small test amounts)
pub fn build_deposit_calldata(assets: u128, receiver: &str) -> String {
    let receiver_padded = format!("{:0>64}", receiver.trim_start_matches("0x"));
    let min_shares_out: u128 = 0;  // no slippage check
    format!("0x8dbdbe6d{:064x}{:064x}{}", assets, min_shares_out, receiver_padded)
}

/// Umami custom redeem(uint256 shares, uint256 minAssetsOut, address receiver, address owner) calldata
/// selector: 0x0169a996 (verified via cast sig + live tx analysis)
/// minAssetsOut = 0 (no slippage protection, acceptable for small test amounts)
pub fn build_redeem_calldata(shares: u128, receiver: &str, owner: &str) -> String {
    let receiver_padded = format!("{:0>64}", receiver.trim_start_matches("0x"));
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x"));
    let min_assets_out: u128 = 0;  // no slippage check
    format!("0x0169a996{:064x}{:064x}{}{}", shares, min_assets_out, receiver_padded, owner_padded)
}
