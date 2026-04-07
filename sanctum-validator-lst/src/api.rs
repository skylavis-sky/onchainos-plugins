// Sanctum Validator LSTs — API client
//
// Router API:   https://sanctum-s-api.fly.dev
// Extra API:    https://extra-api.sanctum.so
//
// Verified shapes (from sanctum-infinity reference):
//
// GET /v2/swap/quote → {"inAmount":"...","outAmount":"...","swapSrc":"SPool","fees":[...]}
// POST /v1/swap      → {"tx":"<base64>"}
// GET /v1/lsts       → {"lsts":[{"mint":"...","symbol":"...","name":"...","decimals":9},...]}
// GET /v1/apy/latest?lst=<mint>,<mint>  → {"apys":{"<mint>":0.07,...},"errs":{}}
// GET /v1/tvl/current?lst=<mint>        → {"tvls":{"<mint>":"1234567890000",...}}
// GET /v1/sol-value/current?lst=<mint>  → {"solValues":{"<mint>":"1050000000"},"errs":{}}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use crate::config::{EXTRA_API_BASE, ROUTER_API_BASE};

// ──────────────────────── Extra API response types ────────────────────────

#[derive(Debug, Deserialize)]
pub struct LstsResp {
    pub lsts: Vec<LstInfo>,
}

#[derive(Debug, Deserialize)]
pub struct LstInfo {
    pub mint: String,
    pub symbol: String,
    pub name: String,
    pub decimals: u8,
}

#[derive(Debug, Deserialize)]
pub struct ApyResp {
    pub apys: HashMap<String, f64>,
    #[serde(default)]
    pub errs: HashMap<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct TvlResp {
    #[serde(default)]
    pub tvls: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct SolValueResp {
    #[serde(rename = "solValues")]
    pub sol_values: HashMap<String, String>,
    #[serde(default)]
    pub errs: HashMap<String, Value>,
}

// ──────────────────────── Router API response types ────────────────────────

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SwapQuoteResp {
    #[serde(rename = "inAmount")]
    pub in_amount: String,
    #[serde(rename = "outAmount")]
    pub out_amount: String,
    #[serde(rename = "swapSrc")]
    pub swap_src: String,
    #[serde(default)]
    pub fees: Vec<FeeEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FeeEntry {
    pub code: String,
    pub rate: String,
    pub amt: String,
    pub mint: String,
}

#[derive(Debug, Deserialize)]
pub struct TxResp {
    pub tx: String, // base64-encoded VersionedTransaction
}

// ──────────────────────── API functions ────────────────────────

/// Fetch all LSTs from Extra API. Returns Err if API unavailable.
pub async fn get_lsts(client: &reqwest::Client) -> Result<Vec<LstInfo>> {
    let url = format!("{}/v1/lsts", EXTRA_API_BASE);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("GET /v1/lsts failed ({})", resp.status());
    }
    let r: LstsResp = resp.json().await?;
    Ok(r.lsts)
}

/// Fetch APY for multiple LSTs by mint address (comma-separated query param).
/// Returns ApyResp; individual entries may be absent if API has no data.
pub async fn get_apy(client: &reqwest::Client, mints: &[&str]) -> Result<ApyResp> {
    let params = mints.iter().map(|m| format!("lst={}", m)).collect::<Vec<_>>().join("&");
    let url = format!("{}/v1/apy/latest?{}", EXTRA_API_BASE, params);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("GET /v1/apy/latest failed ({})", resp.status());
    }
    Ok(resp.json::<ApyResp>().await?)
}

/// Fetch TVL for multiple LSTs.
pub async fn get_tvl(client: &reqwest::Client, mints: &[&str]) -> Result<TvlResp> {
    let params = mints.iter().map(|m| format!("lst={}", m)).collect::<Vec<_>>().join("&");
    let url = format!("{}/v1/tvl/current?{}", EXTRA_API_BASE, params);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("GET /v1/tvl/current failed ({})", resp.status());
    }
    Ok(resp.json::<TvlResp>().await?)
}

/// Fetch SOL value for multiple LSTs.
pub async fn get_sol_value(client: &reqwest::Client, mints: &[&str]) -> Result<SolValueResp> {
    let params = mints.iter().map(|m| format!("lst={}", m)).collect::<Vec<_>>().join("&");
    let url = format!("{}/v1/sol-value/current?{}", EXTRA_API_BASE, params);
    let resp = client.get(&url).send().await?;
    if !resp.status().is_success() {
        anyhow::bail!("GET /v1/sol-value/current failed ({})", resp.status());
    }
    Ok(resp.json::<SolValueResp>().await?)
}

/// Get swap quote from Router API v2.
/// Retries up to 3 times on 502 / connection error.
pub async fn get_swap_quote(
    client: &reqwest::Client,
    input_mint: &str,
    output_mint: &str,
    amount: u64,
) -> Result<SwapQuoteResp> {
    let url = format!(
        "{}/v2/swap/quote?input={}&outputLstMint={}&amount={}&mode=ExactIn",
        ROUTER_API_BASE, input_mint, output_mint, amount
    );

    let mut last_err = anyhow::anyhow!("unknown error");
    for attempt in 1..=3u32 {
        match client.get(&url).send().await {
            Ok(resp) => {
                if resp.status() == 502 {
                    last_err = anyhow::anyhow!(
                        "Sanctum Router API is temporarily unavailable (502). Please try again."
                    );
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(last_err);
                }
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("Swap quote failed ({}): {}", status, body));
                }
                return Ok(resp.json::<SwapQuoteResp>().await?);
            }
            Err(e) => {
                last_err = anyhow::anyhow!("Request error: {}", e);
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }
    Err(last_err)
}

/// Execute swap — POSTs to /v1/swap, returns base64-encoded transaction.
/// swap_src: taken directly from the quote response (do not hardcode "SPool").
pub async fn execute_swap(
    client: &reqwest::Client,
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    quoted_amount: u64,
    signer: &str,
    swap_src: &str,
) -> Result<String> {
    let url = format!("{}/v1/swap", ROUTER_API_BASE);
    let body = serde_json::json!({
        "input": input_mint,
        "outputLstMint": output_mint,
        "amount": amount.to_string(),
        "quotedAmount": quoted_amount.to_string(),
        "mode": "ExactIn",
        "signer": signer,
        "swapSrc": swap_src
    });

    let mut last_err = anyhow::anyhow!("unknown error");
    for attempt in 1..=3u32 {
        match client.post(&url).json(&body).send().await {
            Ok(resp) => {
                if resp.status() == 502 {
                    last_err = anyhow::anyhow!(
                        "Sanctum Router API is temporarily unavailable (502). Please try again."
                    );
                    if attempt < 3 {
                        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                        continue;
                    }
                    return Err(last_err);
                }
                if !resp.status().is_success() {
                    let status = resp.status();
                    let err_body = resp.text().await.unwrap_or_default();
                    return Err(anyhow::anyhow!("Swap failed ({}): {}", status, err_body));
                }
                let tx_resp: TxResp = resp.json().await?;
                return Ok(tx_resp.tx);
            }
            Err(e) => {
                last_err = anyhow::anyhow!("Request error: {}", e);
                if attempt < 3 {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            }
        }
    }
    Err(last_err)
}

// ──────────────────────── Unit helpers ────────────────────────

pub fn ui_to_atomics(amount: f64, decimals: u32) -> u64 {
    (amount * 10f64.powi(decimals as i32)).round() as u64
}

pub fn atomics_to_ui(atomics: u64, decimals: u32) -> f64 {
    atomics as f64 / 10f64.powi(decimals as i32)
}

/// Apply slippage — returns minimum acceptable out amount.
/// min_out = floor(amount * (1 - slippage_pct / 100))
pub fn apply_slippage(amount: u64, slippage_pct: f64) -> u64 {
    let factor = 1.0 - slippage_pct / 100.0;
    (amount as f64 * factor).floor() as u64
}
