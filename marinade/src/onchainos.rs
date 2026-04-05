// onchainos CLI wrapper for Marinade (Solana)
use anyhow::Context;
use serde_json::Value;
use std::process::Command;

/// Resolve the current Solana wallet address from onchainos.
/// ⚠️  Solana chain does NOT support --output json on wallet balance.
/// Address path: data.details[0].tokenAssets[0].address
pub fn resolve_wallet_solana() -> anyhow::Result<String> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()
        .context("Failed to run onchainos wallet balance")?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value =
        serde_json::from_str(&stdout).context("Failed to parse onchainos wallet balance JSON")?;
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
    if let Some(addr) = json["data"]["address"].as_str() {
        if !addr.is_empty() {
            return Ok(addr.to_string());
        }
    }
    anyhow::bail!("Cannot resolve Solana wallet address. Ensure onchainos is logged in.")
}

/// Execute a Solana DEX swap via onchainos swap execute.
/// Used for both stake (SOL→mSOL) and unstake (mSOL→SOL).
///
/// from_mint: source token mint address (or native SOL: "11111111111111111111111111111111")
/// to_mint:   destination token mint address
/// readable_amount: human-readable amount string (e.g. "0.001")
/// slippage:  percent, e.g. "1.0"
/// dry_run:   if true, return simulated response without executing
pub async fn swap_execute(
    from_mint: &str,
    to_mint: &str,
    readable_amount: &str,
    slippage: &str,
    dry_run: bool,
) -> anyhow::Result<Value> {
    if dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "swapTxHash": "", "txHash": "" },
            "from": from_mint,
            "to": to_mint,
            "amount": readable_amount
        }));
    }

    let wallet = resolve_wallet_solana()?;

    let output = Command::new("onchainos")
        .args([
            "swap",
            "execute",
            "--chain",
            "501",
            "--from",
            from_mint,
            "--to",
            to_mint,
            "--readable-amount",
            readable_amount,
            "--slippage",
            slippage,
            "--wallet",
            &wallet,
        ])
        .output()
        .context("Failed to run onchainos swap execute")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).context("Failed to parse onchainos swap execute response")
}

/// Extract txHash from onchainos response.
/// Checks: data.swapTxHash → data.txHash → txHash (root)
pub fn extract_tx_hash(result: &Value) -> String {
    result["data"]["swapTxHash"]
        .as_str()
        .or_else(|| result["data"]["txHash"].as_str())
        .or_else(|| result["txHash"].as_str())
        .unwrap_or("pending")
        .to_string()
}
