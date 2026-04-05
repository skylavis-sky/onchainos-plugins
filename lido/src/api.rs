// src/api.rs — Lido REST API queries
use anyhow::Context;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AprSmaResponse {
    pub data: AprSmaData,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct AprSmaData {
    pub smaApr: Option<String>,
    pub apr: Option<Vec<AprEntry>>,
}

#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct AprEntry {
    pub apr: Option<f64>,
    pub timeUnix: Option<u64>,
}

/// Fetch the 7-day SMA APR from Lido's API.
pub async fn get_apr_sma() -> anyhow::Result<f64> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(crate::config::LIDO_APR_SMA_URL)
        .send()
        .await
        .context("Failed to fetch Lido APR SMA")?
        .json()
        .await
        .context("Failed to parse Lido APR SMA response")?;

    // Try data.smaApr first
    if let Some(sma) = resp["data"]["smaApr"].as_str() {
        if let Ok(v) = sma.parse::<f64>() {
            return Ok(v);
        }
    }
    if let Some(sma) = resp["data"]["smaApr"].as_f64() {
        return Ok(sma);
    }
    // Fallback: last element of apr array
    if let Some(arr) = resp["data"]["apr"].as_array() {
        if let Some(last) = arr.last() {
            if let Some(apr) = last["apr"].as_f64() {
                return Ok(apr);
            }
        }
    }
    anyhow::bail!("Could not parse APR from response: {}", resp)
}

/// Query withdrawal request estimated wait time.
pub async fn get_request_time(request_ids: &[u64]) -> anyhow::Result<serde_json::Value> {
    let client = reqwest::Client::new();
    let mut url = crate::config::LIDO_WQ_API_URL.to_string();
    for (i, id) in request_ids.iter().enumerate() {
        if i == 0 {
            url.push('?');
        } else {
            url.push('&');
        }
        url.push_str(&format!("ids={}", id));
    }
    let resp: serde_json::Value = client
        .get(&url)
        .send()
        .await
        .context("Failed to fetch withdrawal request time")?
        .json()
        .await
        .context("Failed to parse withdrawal request time response")?;
    Ok(resp)
}
