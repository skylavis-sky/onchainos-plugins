use anyhow::Context;
use reqwest::Client;
use serde_json::Value;
use std::env;

use crate::config::ACROSS_API_BASE;

/// Build an HTTP client that respects HTTPS_PROXY env var.
pub fn build_client() -> anyhow::Result<Client> {
    let mut builder = Client::builder();
    if let Ok(proxy_url) = env::var("HTTPS_PROXY").or_else(|_| env::var("https_proxy")) {
        let proxy = reqwest::Proxy::https(&proxy_url)
            .with_context(|| format!("Invalid proxy URL: {}", proxy_url))?;
        builder = builder.proxy(proxy);
    }
    builder.build().context("Failed to build HTTP client")
}

/// GET /api/suggested-fees
pub async fn get_suggested_fees(
    input_token: &str,
    output_token: &str,
    origin_chain_id: u64,
    destination_chain_id: u64,
    amount: &str,
    depositor: Option<&str>,
    recipient: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let mut params = vec![
        ("inputToken", input_token.to_string()),
        ("outputToken", output_token.to_string()),
        ("originChainId", origin_chain_id.to_string()),
        ("destinationChainId", destination_chain_id.to_string()),
        ("amount", amount.to_string()),
    ];
    if let Some(d) = depositor {
        params.push(("depositor", d.to_string()));
    }
    if let Some(r) = recipient {
        params.push(("recipient", r.to_string()));
    }
    let url = format!("{}/suggested-fees", ACROSS_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to call suggested-fees API")?;
    let status = resp.status();
    let body: Value = resp.json().await.context("Failed to parse suggested-fees response")?;
    if !status.is_success() {
        anyhow::bail!("suggested-fees API error {}: {}", status, body);
    }
    Ok(body)
}

/// GET /api/limits
pub async fn get_limits(
    input_token: &str,
    output_token: &str,
    origin_chain_id: u64,
    destination_chain_id: u64,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let params = [
        ("inputToken", input_token.to_string()),
        ("outputToken", output_token.to_string()),
        ("originChainId", origin_chain_id.to_string()),
        ("destinationChainId", destination_chain_id.to_string()),
    ];
    let url = format!("{}/limits", ACROSS_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to call limits API")?;
    let status = resp.status();
    let body: Value = resp.json().await.context("Failed to parse limits response")?;
    if !status.is_success() {
        anyhow::bail!("limits API error {}: {}", status, body);
    }
    Ok(body)
}

/// GET /api/available-routes
pub async fn get_available_routes(
    origin_chain_id: Option<u64>,
    destination_chain_id: Option<u64>,
    origin_token: Option<&str>,
    destination_token: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let mut params: Vec<(&str, String)> = vec![];
    if let Some(id) = origin_chain_id {
        params.push(("originChainId", id.to_string()));
    }
    if let Some(id) = destination_chain_id {
        params.push(("destinationChainId", id.to_string()));
    }
    if let Some(t) = origin_token {
        params.push(("originToken", t.to_string()));
    }
    if let Some(t) = destination_token {
        params.push(("destinationToken", t.to_string()));
    }
    let url = format!("{}/available-routes", ACROSS_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to call available-routes API")?;
    let status = resp.status();
    let body: Value = resp.json().await.context("Failed to parse available-routes response")?;
    if !status.is_success() {
        anyhow::bail!("available-routes API error {}: {}", status, body);
    }
    Ok(body)
}

/// GET /api/deposit/status
pub async fn get_deposit_status(
    deposit_txn_ref: Option<&str>,
    deposit_id: Option<u64>,
    origin_chain_id: Option<u64>,
    relay_data_hash: Option<&str>,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let mut params: Vec<(&str, String)> = vec![];
    if let Some(txn) = deposit_txn_ref {
        params.push(("depositTxnRef", txn.to_string()));
    }
    if let Some(id) = deposit_id {
        params.push(("depositId", id.to_string()));
    }
    if let Some(id) = origin_chain_id {
        params.push(("originChainId", id.to_string()));
    }
    if let Some(h) = relay_data_hash {
        params.push(("relayDataHash", h.to_string()));
    }
    let url = format!("{}/deposit/status", ACROSS_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to call deposit/status API")?;
    let status = resp.status();
    let body: Value = resp.json().await.context("Failed to parse deposit/status response")?;
    if !status.is_success() {
        // For 404 (deposit not found), return a synthetic "not_found" status instead of bailing
        if status.as_u16() == 404 {
            return Ok(serde_json::json!({
                "status": "not_found",
                "message": body["message"].as_str().unwrap_or("Deposit not found"),
                "depositTxnHash": "N/A",
                "fillTxnHash": "N/A",
                "depositRefundTxnHash": "N/A",
                "depositId": null,
                "originChainId": null,
                "destinationChainId": null
            }));
        }
        anyhow::bail!("deposit/status API error {}: {}", status, body);
    }
    Ok(body)
}
