use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::config::BALANCER_API_BASE;

fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
        .or_else(|_| std::env::var("HTTP_PROXY"))
        .or_else(|_| std::env::var("http_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::all(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}

/// A Balancer pool entry from the Balancer REST API
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BalancerPool {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub pool_type: String,
    #[serde(default)]
    pub tokens: Vec<PoolToken>,
    #[serde(default)]
    pub total_liquidity: Option<Value>,
    #[serde(default)]
    pub apr: Option<Value>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PoolToken {
    #[serde(default)]
    pub address: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub decimals: Option<u32>,
}

/// Fetch pools from the Balancer REST API for a given chain
pub async fn fetch_balancer_pools(chain_id: u64) -> Result<Vec<BalancerPool>> {
    let url = format!("{}/pools/{}", BALANCER_API_BASE, chain_id);
    let client = build_client();
    let resp: Value = client.get(&url)
        .header("User-Agent", "aura-finance-plugin/0.1")
        .send()
        .await?
        .json()
        .await?;

    // The API may return a top-level array or a nested structure
    let pools_raw = if resp.is_array() {
        resp.as_array().cloned().unwrap_or_default()
    } else {
        resp["pools"].as_array().cloned()
            .or_else(|| resp["data"].as_array().cloned())
            .unwrap_or_default()
    };

    let mut result = Vec::new();
    for pool_val in pools_raw {
        if let Ok(pool) = serde_json::from_value::<BalancerPool>(pool_val) {
            result.push(pool);
        }
    }
    Ok(result)
}

/// Pool summary for display (Aura-oriented)
#[derive(Debug, Serialize)]
pub struct AuraPoolSummary {
    pub aura_pid: u64,
    pub lp_token: String,
    pub crv_rewards: String,
    pub tokens: Vec<String>,
    pub tvl_usd: String,
    pub shutdown: bool,
}
