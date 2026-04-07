/// REST client for Vertex Engine Gateway and Archive (Indexer).
///
/// All engine queries are POST {gateway_url}/query with {"type": "<query_type>", ...fields}.
/// All archive queries are POST {archive_url} with JSON body.
/// No authentication is required for read operations.
///
/// IMPORTANT: reqwest does not read system proxy env vars by default.
/// build_client() explicitly reads HTTPS_PROXY/HTTP_PROXY to avoid connection failures
/// in sandboxed environments.

use anyhow::Context;
use reqwest::Client;
use serde_json::{json, Value};

/// Build a reqwest client with explicit proxy support.
/// Reads HTTPS_PROXY and HTTP_PROXY environment variables if set.
pub fn build_client() -> anyhow::Result<Client> {
    let mut builder = Client::builder()
        .timeout(std::time::Duration::from_secs(30));

    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if !proxy_url.is_empty() {
            builder = builder.proxy(
                reqwest::Proxy::https(&proxy_url)
                    .with_context(|| format!("Invalid HTTPS_PROXY URL: {}", proxy_url))?,
            );
        }
    }
    if let Ok(proxy_url) = std::env::var("HTTP_PROXY").or_else(|_| std::env::var("http_proxy")) {
        if !proxy_url.is_empty() {
            builder = builder.proxy(
                reqwest::Proxy::http(&proxy_url)
                    .with_context(|| format!("Invalid HTTP_PROXY URL: {}", proxy_url))?,
            );
        }
    }

    builder.build().context("Failed to build HTTP client")
}

/// POST to the engine gateway /query endpoint.
/// body: JSON object with at minimum {"type": "<query_type>"}.
pub async fn engine_query(gateway_url: &str, body: Value) -> anyhow::Result<Value> {
    let client = build_client()?;
    let url = format!("{}/query", gateway_url);
    let resp = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("Failed to POST {}", url))?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read response body")?;

    if !status.is_success() {
        anyhow::bail!("Engine gateway returned HTTP {}: {}", status, text);
    }

    serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse engine response as JSON: {}", text))
}

/// POST to the archive (indexer) endpoint.
/// body: JSON object with at minimum {"type": "<query_type>"}.
pub async fn archive_query(archive_url: &str, body: Value) -> anyhow::Result<Value> {
    let client = build_client()?;
    let resp = client
        .post(archive_url)
        .json(&body)
        .send()
        .await
        .with_context(|| format!("Failed to POST {}", archive_url))?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read archive response body")?;

    if !status.is_success() {
        anyhow::bail!("Archive returned HTTP {}: {}", status, text);
    }

    serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse archive response as JSON: {}", text))
}

/// GET from the v2 gateway endpoint (CoinGecko-compatible orderbook format).
/// The v2 base URL is the gateway_url with /v1 replaced by /v2.
pub async fn gateway_v2_get(gateway_url: &str, path: &str, params: &[(&str, &str)]) -> anyhow::Result<Value> {
    let client = build_client()?;
    // Convert /v1 base to /v2
    let base_v2 = gateway_url.trim_end_matches("/v1").trim_end_matches('/');
    let url = format!("{}/v2{}", base_v2, path);

    let resp = client
        .get(&url)
        .query(params)
        .send()
        .await
        .with_context(|| format!("Failed to GET {}", url))?;

    let status = resp.status();
    let text = resp.text().await.context("Failed to read v2 response body")?;

    if !status.is_success() {
        anyhow::bail!("Gateway v2 returned HTTP {}: {}", status, text);
    }

    serde_json::from_str(&text)
        .with_context(|| format!("Failed to parse v2 response as JSON: {}", text))
}

/// Query all products from the engine gateway.
/// Returns the raw response which contains spot_products and perp_products.
pub async fn query_all_products(gateway_url: &str) -> anyhow::Result<Value> {
    engine_query(gateway_url, json!({"type": "all_products"})).await
}

/// Query market symbols (with funding rates) by product type.
/// product_type: "perp" or "spot"
pub async fn query_symbols(gateway_url: &str, product_type: Option<&str>) -> anyhow::Result<Value> {
    let mut body = json!({"type": "symbols"});
    if let Some(pt) = product_type {
        body["product_type"] = json!(pt);
    }
    engine_query(gateway_url, body).await
}

/// Query subaccount summary/info from the engine gateway.
/// subaccount: 32-byte hex string (0x + 40 addr chars + 24 name chars).
pub async fn query_subaccount_info(gateway_url: &str, subaccount: &str) -> anyhow::Result<Value> {
    engine_query(
        gateway_url,
        json!({
            "type": "subaccount_info",
            "subaccount": subaccount
        }),
    )
    .await
}

/// Query market liquidity (orderbook) for a product using the engine /query endpoint.
#[allow(dead_code)]
pub async fn query_market_liquidity(gateway_url: &str, product_id: u32, depth: u32) -> anyhow::Result<Value> {
    engine_query(
        gateway_url,
        json!({
            "type": "market_liquidity",
            "product_id": product_id,
            "depth": depth
        }),
    )
    .await
}

/// Query market prices for a list of product IDs from the archive.
pub async fn query_perp_prices(archive_url: &str, product_ids: &[u32]) -> anyhow::Result<Value> {
    archive_query(
        archive_url,
        json!({
            "type": "perp_prices",
            "product_ids": product_ids
        }),
    )
    .await
}

/// Query market prices for a list of product IDs from the engine gateway.
pub async fn query_market_prices(gateway_url: &str, product_ids: &[u32]) -> anyhow::Result<Value> {
    engine_query(
        gateway_url,
        json!({
            "type": "market_prices",
            "product_ids": product_ids
        }),
    )
    .await
}

/// Convert a fixed-point x18 string value (1e18) to a human-readable f64.
/// Vertex uses i128/u128 as decimal strings with 18 decimals of precision.
pub fn x18_to_f64(val_str: &str) -> f64 {
    val_str
        .trim_start_matches('-')
        .parse::<u128>()
        .map(|v| {
            let is_neg = val_str.starts_with('-');
            let f = v as f64 / 1e18;
            if is_neg { -f } else { f }
        })
        .unwrap_or(0.0)
}
