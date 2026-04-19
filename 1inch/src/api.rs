/// 1inch Aggregation API v6 client.
///
/// All requests require a Bearer token (`Authorization: Bearer {ONEINCH_API_KEY}`).
/// Base URL: https://api.1inch.dev/swap/v6.0/{chainId}

use anyhow::Context;
use reqwest::blocking::Client;
use serde_json::Value;
use std::time::Duration;

const BASE_URL: &str = "https://api.1inch.dev/swap/v6.0";
const MAX_RETRIES: u32 = 3;

pub fn get_api_key() -> String {
    std::env::var("ONEINCH_API_KEY").unwrap_or_else(|_| "demo".to_string())
}

/// Build a reqwest blocking client with optional proxy support.
pub fn build_client() -> Client {
    let mut builder = Client::builder().timeout(Duration::from_secs(30));
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::https(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// Perform a GET request to the 1inch API with retry on 429.
fn get_with_retry(client: &Client, url: &str) -> anyhow::Result<Value> {
    let api_key = get_api_key();
    let mut last_err = anyhow::anyhow!("No attempts made");

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            std::thread::sleep(Duration::from_millis(1000 * 2u64.pow(attempt - 1)));
        }

        let resp = client
            .get(url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Accept", "application/json")
            .send();

        match resp {
            Err(e) => {
                last_err = anyhow::anyhow!("HTTP request failed: {}", e);
                continue;
            }
            Ok(r) => {
                let status = r.status();
                let body = r.text().unwrap_or_default();

                match status.as_u16() {
                    200 => {
                        return serde_json::from_str(&body)
                            .with_context(|| format!("Failed to parse 1inch response: {}", &body[..body.len().min(500)]));
                    }
                    400 => {
                        let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                        let desc = v["description"]
                            .as_str()
                            .or_else(|| v["error"].as_str())
                            .unwrap_or("Bad request");
                        anyhow::bail!("1inch API error (400): {}", desc);
                    }
                    401 => {
                        anyhow::bail!(
                            "Invalid or missing API key. Set the ONEINCH_API_KEY environment variable. Obtain a key at https://portal.1inch.dev"
                        );
                    }
                    429 => {
                        last_err = anyhow::anyhow!("Rate limit exceeded (429). Retrying...");
                        eprintln!("  [warn] 1inch rate limit hit. Waiting before retry...");
                        continue;
                    }
                    500..=599 => {
                        anyhow::bail!("1inch API temporarily unavailable ({}). Please try again later.", status);
                    }
                    other => {
                        anyhow::bail!("Unexpected HTTP status {} from 1inch API. Body: {}", other, &body[..body.len().min(200)]);
                    }
                }
            }
        }
    }

    Err(last_err)
}

/// GET /quote — returns expected output amount for a swap (read-only).
pub fn get_quote(
    client: &Client,
    chain_id: u64,
    src: &str,
    dst: &str,
    amount: &str,
) -> anyhow::Result<Value> {
    let url = format!(
        "{}/{}/quote?src={}&dst={}&amount={}",
        BASE_URL, chain_id, src, dst, amount
    );
    get_with_retry(client, &url)
}

/// GET /swap — returns swap calldata (tx.data, tx.to, tx.value).
pub fn get_swap(
    client: &Client,
    chain_id: u64,
    src: &str,
    dst: &str,
    amount: &str,
    from: &str,
    slippage_percent: f64,
    disable_estimate: bool,
) -> anyhow::Result<Value> {
    let mut url = format!(
        "{}/{}/swap?src={}&dst={}&amount={}&from={}&slippage={}",
        BASE_URL, chain_id, src, dst, amount, from, slippage_percent
    );
    if disable_estimate {
        url.push_str("&disableEstimate=true");
    }
    get_with_retry(client, &url)
}

/// GET /approve/allowance — returns current ERC-20 allowance for the 1inch router.
pub fn get_allowance(
    client: &Client,
    chain_id: u64,
    token_address: &str,
    wallet_address: &str,
) -> anyhow::Result<Value> {
    let url = format!(
        "{}/{}/approve/allowance?tokenAddress={}&walletAddress={}",
        BASE_URL, chain_id, token_address, wallet_address
    );
    get_with_retry(client, &url)
}

/// GET /approve/transaction — returns ERC-20 approve calldata.
/// Pass amount as None for unlimited (uint256.max).
pub fn get_approve_tx(
    client: &Client,
    chain_id: u64,
    token_address: &str,
    amount: Option<&str>,
) -> anyhow::Result<Value> {
    let mut url = format!(
        "{}/{}/approve/transaction?tokenAddress={}",
        BASE_URL, chain_id, token_address
    );
    if let Some(amt) = amount {
        url.push_str(&format!("&amount={}", amt));
    }
    get_with_retry(client, &url)
}
