// Solana JSON-RPC helpers

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::config::SOLANA_RPC;
use crate::instructions::StakePoolInfo;

/// Make a Solana JSON-RPC call (single retry on 429 / connection error).
pub async fn solana_rpc(method: &str, params: Value) -> Result<Value> {
    let client = reqwest::Client::new();
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params
    });

    let mut last_err = anyhow!("unknown rpc error");
    for attempt in 1..=2u32 {
        match client.post(SOLANA_RPC).json(&body).send().await {
            Ok(resp) => {
                if resp.status() == 429 {
                    last_err = anyhow!("Solana RPC rate limited (429)");
                    if attempt < 2 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(last_err);
                }
                let json_resp: Value = resp.json().await?;
                if let Some(err) = json_resp.get("error") {
                    return Err(anyhow!("RPC error: {}", err));
                }
                return Ok(json_resp["result"].clone());
            }
            Err(e) => {
                last_err = anyhow!("RPC request error: {}", e);
                if attempt < 2 {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }
    Err(last_err)
}

/// Get account data as raw bytes (base64-decoded).
pub async fn get_account_data(address: &str) -> Result<Vec<u8>> {
    let result = solana_rpc(
        "getAccountInfo",
        json!([address, {"encoding": "base64"}]),
    )
    .await?;

    let b64 = result["value"]["data"][0]
        .as_str()
        .ok_or_else(|| anyhow!("No account data for {}", address))?;

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    Ok(STANDARD.decode(b64)?)
}

/// Parse SPL Stake Pool account data, returning StakePoolInfo.
pub async fn fetch_stake_pool(address: &str) -> Result<StakePoolInfo> {
    let data = get_account_data(address).await?;
    crate::instructions::parse_stake_pool(&data)
}

/// Get latest finalized blockhash.
pub async fn get_latest_blockhash() -> Result<String> {
    let result = solana_rpc(
        "getLatestBlockhash",
        json!([{"commitment": "finalized"}]),
    )
    .await?;

    result["value"]["blockhash"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow!("Failed to get blockhash"))
}

/// Get the highest-balance token account for the given wallet and mint.
/// Returns (ui_balance, raw_balance, account_pubkey).
/// Returns Err if no token accounts exist.
pub async fn get_token_accounts_by_owner(
    wallet: &str,
    mint: &str,
) -> Result<(f64, u64, String)> {
    let result = solana_rpc(
        "getTokenAccountsByOwner",
        json!([wallet, {"mint": mint}, {"encoding": "jsonParsed"}]),
    )
    .await?;

    let accounts = result["value"]
        .as_array()
        .ok_or_else(|| anyhow!("No token accounts array for wallet {}", wallet))?;

    let mut best_ui = 0.0f64;
    let mut best_raw = 0u64;
    let mut best_addr = String::new();

    for acc in accounts {
        let amount = &acc["account"]["data"]["parsed"]["info"]["tokenAmount"];
        let ui = amount["uiAmount"].as_f64().unwrap_or(0.0);
        let raw_str = amount["amount"].as_str().unwrap_or("0");
        let raw: u64 = raw_str.parse().unwrap_or(0);
        let addr = acc["pubkey"].as_str().unwrap_or("").to_string();

        if raw > best_raw {
            best_ui = ui;
            best_raw = raw;
            best_addr = addr;
        }
    }

    if best_addr.is_empty() {
        return Err(anyhow!("No token accounts found for mint {} / wallet {}", mint, wallet));
    }

    Ok((best_ui, best_raw, best_addr))
}

/// Get ALL token accounts for a wallet (for get-position scan).
/// Returns Vec of (mint, ui_balance, raw_balance, account_pubkey).
pub async fn get_all_token_accounts(wallet: &str) -> Result<Vec<(String, f64, u64, String)>> {
    let result = solana_rpc(
        "getTokenAccountsByOwner",
        json!([wallet, {"programId": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"}, {"encoding": "jsonParsed"}]),
    )
    .await?;

    let accounts = result["value"]
        .as_array()
        .ok_or_else(|| anyhow!("No token accounts array"))?;

    let mut out = Vec::new();
    for acc in accounts {
        let info = &acc["account"]["data"]["parsed"]["info"];
        let mint = info["mint"].as_str().unwrap_or("").to_string();
        let amount = &info["tokenAmount"];
        let ui = amount["uiAmount"].as_f64().unwrap_or(0.0);
        let raw_str = amount["amount"].as_str().unwrap_or("0");
        let raw: u64 = raw_str.parse().unwrap_or(0);
        let addr = acc["pubkey"].as_str().unwrap_or("").to_string();

        if !mint.is_empty() && raw > 0 {
            out.push((mint, ui, raw, addr));
        }
    }

    Ok(out)
}
