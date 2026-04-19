use serde_json::Value;
use std::process::Command;

/// Resolve the current logged-in Solana wallet address and token balances.
///
/// Uses `onchainos wallet balance --chain 501`.
/// NOTE: Do NOT add `--output json` for chain 501 — Solana returns JSON natively;
/// adding --output json causes an EOF parse failure.
pub fn resolve_wallet_solana() -> anyhow::Result<Value> {
    let output = Command::new("onchainos")
        .args(["wallet", "balance", "--chain", "501"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: Value = serde_json::from_str(&stdout)
        .map_err(|e| anyhow::anyhow!("Failed to parse onchainos output: {}\nraw: {}", e, stdout))?;
    Ok(json)
}

/// Extract the wallet address from an onchainos wallet balance response.
pub fn extract_wallet_address(balance_json: &Value) -> Option<String> {
    balance_json["data"]["details"][0]["tokenAssets"][0]["address"]
        .as_str()
        .map(|s| s.to_string())
}

/// Find a token asset entry by symbol (case-insensitive) in the tokenAssets array.
pub fn find_token_asset<'a>(balance_json: &'a Value, symbol: &str) -> Option<&'a Value> {
    let assets = balance_json["data"]["details"][0]["tokenAssets"].as_array()?;
    let symbol_lower = symbol.to_lowercase();
    assets.iter().find(|a| {
        a["symbol"]
            .as_str()
            .map(|s| s.to_lowercase() == symbol_lower)
            .unwrap_or(false)
    })
}
