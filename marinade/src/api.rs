// Marinade + Solana RPC API layer
use anyhow::Context;
use serde_json::Value;

use crate::config::{MARINADE_PRICE_API, MSOL_MINT, SOLANA_RPC_URL};

/// Fetch mSOL/SOL price from Marinade REST API.
/// Returns a plain float, e.g. 1.3713931272762248
/// meaning 1 mSOL = 1.371 SOL
pub async fn fetch_msol_price_sol() -> anyhow::Result<f64> {
    let client = reqwest::Client::new();
    let text = client
        .get(MARINADE_PRICE_API)
        .send()
        .await
        .context("Failed to fetch mSOL price")?
        .text()
        .await
        .context("Failed to read mSOL price response")?;
    text.trim()
        .parse::<f64>()
        .context("Failed to parse mSOL price as float")
}

/// Fetch total mSOL supply via Solana RPC getTokenSupply.
/// Returns uiAmount (f64) representing the total mSOL in circulation.
pub async fn fetch_msol_total_supply() -> anyhow::Result<f64> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenSupply",
        "params": [MSOL_MINT]
    });
    let resp: Value = client
        .post(SOLANA_RPC_URL)
        .json(&body)
        .send()
        .await
        .context("Failed to call getTokenSupply")?
        .json()
        .await
        .context("Failed to parse getTokenSupply response")?;
    let ui_amount = resp["result"]["value"]["uiAmount"]
        .as_f64()
        .unwrap_or(0.0);
    Ok(ui_amount)
}

/// Fetch user mSOL token account balance via Solana RPC.
/// Returns (msol_balance, token_account_address).
/// If wallet has no mSOL token account, returns (0.0, "").
pub async fn fetch_msol_balance(wallet: &str) -> anyhow::Result<(f64, String)> {
    let client = reqwest::Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getTokenAccountsByOwner",
        "params": [
            wallet,
            {"mint": MSOL_MINT},
            {"encoding": "jsonParsed"}
        ]
    });
    let resp: Value = client
        .post(SOLANA_RPC_URL)
        .json(&body)
        .send()
        .await
        .context("Failed to call getTokenAccountsByOwner")?
        .json()
        .await
        .context("Failed to parse getTokenAccountsByOwner response")?;

    let accounts = resp["result"]["value"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if accounts.is_empty() {
        return Ok((0.0, String::new()));
    }

    let account = &accounts[0];
    let token_account_addr = account["pubkey"].as_str().unwrap_or("").to_string();
    let ui_amount = account["account"]["data"]["parsed"]["info"]["tokenAmount"]["uiAmountString"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(0.0);

    Ok((ui_amount, token_account_addr))
}
