use anyhow::Context;
use reqwest::Client;
use serde_json::Value;
use std::env;

use crate::config::DEBRIDGE_API_BASE;

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

// ---------------------------------------------------------------------------
// GET /v1.0/dln/order/create-tx
// ---------------------------------------------------------------------------

/// Parameters for the create-tx API call.
pub struct CreateTxParams<'a> {
    /// deBridge chain ID for source chain (e.g. "42161" or "7565164" for Solana)
    pub src_chain_id: &'a str,
    pub src_token: &'a str,
    pub src_amount: &'a str,
    /// deBridge chain ID for destination chain
    pub dst_chain_id: &'a str,
    pub dst_token: &'a str,
    /// None for quote-only mode (no tx returned); Some for full tx construction
    pub src_authority: Option<&'a str>,
    pub dst_authority: Option<&'a str>,
    pub dst_recipient: Option<&'a str>,
    /// Skip Solana recipient PDA validation
    pub skip_solana_recipient_validation: bool,
}

/// Call GET /v1.0/dln/order/create-tx.
///
/// Quote-only mode: omit src_authority/dst_authority/dst_recipient.
/// Full tx mode: provide all three authority/recipient fields.
pub async fn create_tx(params: &CreateTxParams<'_>) -> anyhow::Result<Value> {
    let client = build_client()?;
    let mut query: Vec<(&str, String)> = vec![
        ("srcChainId", params.src_chain_id.to_string()),
        ("srcChainTokenIn", params.src_token.to_string()),
        ("srcChainTokenInAmount", params.src_amount.to_string()),
        ("dstChainId", params.dst_chain_id.to_string()),
        ("dstChainTokenOut", params.dst_token.to_string()),
        ("dstChainTokenOutAmount", "auto".to_string()),
        ("prependOperatingExpenses", "true".to_string()),
    ];
    if let Some(src_auth) = params.src_authority {
        query.push(("srcChainOrderAuthorityAddress", src_auth.to_string()));
    }
    if let Some(dst_auth) = params.dst_authority {
        query.push(("dstChainOrderAuthorityAddress", dst_auth.to_string()));
    }
    if let Some(dst_rec) = params.dst_recipient {
        query.push(("dstChainTokenOutRecipient", dst_rec.to_string()));
    }
    if params.skip_solana_recipient_validation {
        query.push(("skipSolanaRecipientValidation", "true".to_string()));
    }

    let url = format!("{}/dln/order/create-tx", DEBRIDGE_API_BASE);
    let resp = client
        .get(&url)
        .query(&query)
        .send()
        .await
        .context("Failed to call create-tx API")?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .context("Failed to parse create-tx response")?;
    if !status.is_success() {
        anyhow::bail!("create-tx API error {}: {}", status, body);
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// GET /v1.0/dln/order/{orderId}/status
// ---------------------------------------------------------------------------

pub async fn get_order_status(order_id: &str) -> anyhow::Result<Value> {
    let client = build_client()?;
    let url = format!("{}/dln/order/{}/status", DEBRIDGE_API_BASE, order_id);
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call order status API")?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .context("Failed to parse order status response")?;
    if !status.is_success() {
        anyhow::bail!("order status API error {}: {}", status, body);
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// GET /v1.0/supported-chains-info
// ---------------------------------------------------------------------------

pub async fn get_supported_chains() -> anyhow::Result<Value> {
    let client = build_client()?;
    let url = format!("{}/supported-chains-info", DEBRIDGE_API_BASE);
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to call supported-chains-info API")?;
    let status = resp.status();
    let body: Value = resp
        .json()
        .await
        .context("Failed to parse supported-chains-info response")?;
    if !status.is_success() {
        anyhow::bail!("supported-chains-info API error {}: {}", status, body);
    }
    Ok(body)
}

// ---------------------------------------------------------------------------
// EVM allowance check via eth_call
// ---------------------------------------------------------------------------

/// Check ERC-20 allowance via eth_call to public RPC.
/// Returns None if the call fails (safe to proceed with approve in that case).
pub async fn get_erc20_allowance(
    rpc_url: &str,
    token: &str,
    owner: &str,
    spender: &str,
) -> Option<u128> {
    let client = build_client().ok()?;
    // allowance(address,address) = 0xdd62ed3e
    let owner_padded = format!("{:0>64}", owner.trim_start_matches("0x").to_lowercase());
    let spender_padded = format!("{:0>64}", spender.trim_start_matches("0x").to_lowercase());
    let data = format!("0xdd62ed3e{}{}", owner_padded, spender_padded);

    let payload = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [
            { "to": token, "data": data },
            "latest"
        ],
        "id": 1
    });
    let resp = client.post(rpc_url).json(&payload).send().await.ok()?;
    let body: Value = resp.json().await.ok()?;
    let result_hex = body["result"].as_str()?;
    let hex_clean = result_hex.trim_start_matches("0x");
    if hex_clean.is_empty() {
        return None;
    }
    u128::from_str_radix(hex_clean, 16).ok()
}
