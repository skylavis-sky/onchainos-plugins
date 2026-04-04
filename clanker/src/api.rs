// src/api.rs — Clanker REST API client
use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const CLANKER_API_BASE: &str = "https://clanker.world/api";

// ── Deserialization helpers ────────────────────────────────────────────────

/// Handles API fields that may arrive as JSON number or string.
mod deser_number_or_string {
    use serde::{self, Deserialize, Deserializer};
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<String>, D::Error> {
        use serde_json::Value;
        Ok(match Option::<Value>::deserialize(d)? {
            Some(Value::String(s)) => Some(s),
            Some(Value::Number(n)) => Some(n.to_string()),
            _ => None,
        })
    }
}

// ── Response types ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClankerToken {
    pub contract_address: Option<String>,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub chain_id: Option<u64>,
    pub deployed_at: Option<String>,
    pub img_url: Option<String>,
    #[serde(default)]
    pub trust_status: Option<TrustStatus>,
    #[serde(default)]
    pub pool_address: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub creator: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustStatus {
    pub is_trusted_clanker: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenListResponse {
    pub tokens: Option<Vec<ClankerToken>>,
    pub total: Option<u64>,
    pub has_more: Option<bool>,
    #[serde(default, deserialize_with = "deser_number_or_string::deserialize")]
    pub page: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchCreatorResponse {
    pub tokens: Option<Vec<ClankerToken>>,
    pub total: Option<u64>,
    pub user: Option<Value>,
    pub searched_address: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployResponse {
    pub success: Option<bool>,
    pub message: Option<String>,
    pub expected_address: Option<String>,
    // Extra fields that may appear
    #[serde(default)]
    pub fee_locker_address: Option<String>,
}

// ── Request body types ─────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployTokenRequest {
    pub token: TokenConfig,
    pub rewards: Vec<RewardConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool: Option<PoolConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vault: Option<VaultConfig>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenConfig {
    pub name: String,
    pub symbol: String,
    pub token_admin: String,
    pub request_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewardConfig {
    pub admin: String,
    pub recipient: String,
    pub allocation: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolConfig {
    #[serde(rename = "type")]
    pub pool_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_market_cap: Option<f64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultConfig {
    pub percentage: u32,
    pub lockup_duration: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vesting_duration: Option<u32>,
}

// ── API functions ──────────────────────────────────────────────────────────

/// GET /api/tokens — list recently deployed tokens
pub async fn list_tokens(
    page: u32,
    limit: u32,
    sort: &str,
    chain_id: Option<u64>,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let mut params = vec![
        ("page", page.to_string()),
        ("limit", limit.to_string()),
        ("sort", sort.to_string()),
    ];
    if let Some(cid) = chain_id {
        params.push(("chain_id", cid.to_string()));
    }
    let resp = client
        .get(format!("{}/tokens", CLANKER_API_BASE))
        .query(&params)
        .send()
        .await
        .context("list_tokens HTTP request failed")?
        .json::<Value>()
        .await
        .context("list_tokens JSON parse failed")?;
    Ok(resp)
}

/// GET /api/search-creator — search tokens by creator address or Farcaster username
pub async fn search_creator(
    q: &str,
    limit: u32,
    offset: u32,
    sort: &str,
    trusted_only: bool,
) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let trusted_str = trusted_only.to_string();
    let params = vec![
        ("q", q.to_string()),
        ("limit", limit.to_string()),
        ("offset", offset.to_string()),
        ("sort", sort.to_string()),
        ("trustedOnly", trusted_str),
    ];
    let resp = client
        .get(format!("{}/search-creator", CLANKER_API_BASE))
        .query(&params)
        .send()
        .await
        .context("search_creator HTTP request failed")?
        .json::<Value>()
        .await
        .context("search_creator JSON parse failed")?;
    Ok(resp)
}

/// POST /api/tokens/deploy — deploy a new ERC-20 token via Clanker REST API
pub async fn deploy_token(api_key: &str, req: &DeployTokenRequest) -> anyhow::Result<Value> {
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/tokens/deploy", CLANKER_API_BASE))
        .header("Content-Type", "application/json")
        .header("x-api-key", api_key)
        .json(req)
        .send()
        .await
        .context("deploy_token HTTP request failed")?
        .json::<Value>()
        .await
        .context("deploy_token JSON parse failed")?;
    Ok(resp)
}
