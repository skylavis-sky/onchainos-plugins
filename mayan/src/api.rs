use anyhow::{anyhow, Context};
use reqwest::Client;
use serde_json::Value;

use crate::config::{EXPLORER_API_BASE, MAYAN_FORWARDER_CONTRACT, PRICE_API_BASE};

/// Fetch cross-chain swap quote from Mayan price API.
/// Returns the full JSON array of route quotes.
pub async fn get_quote(
    client: &Client,
    amount_in64: &str,
    from_token: &str,
    from_chain: &str,
    to_token: &str,
    to_chain: &str,
    slippage_bps: u32,
    destination_address: Option<&str>,
    full_list: bool,
) -> anyhow::Result<Vec<Value>> {
    let mut params = vec![
        ("amountIn64", amount_in64.to_string()),
        ("fromToken", from_token.to_string()),
        ("fromChain", from_chain.to_string()),
        ("toToken", to_token.to_string()),
        ("toChain", to_chain.to_string()),
        ("slippageBps", slippage_bps.to_string()),
        ("fullList", full_list.to_string()),
    ];
    if let Some(dest) = destination_address {
        if !dest.is_empty() {
            params.push(("destinationAddress", dest.to_string()));
        }
    }

    let url = format!("{}/quote", PRICE_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to reach Mayan price API /quote")?;

    let status = resp.status();
    let body = resp.text().await.context("Failed to read /quote response body")?;

    if !status.is_success() {
        return Err(anyhow!("Mayan /quote returned HTTP {}: {}", status, body));
    }

    let json: Value =
        serde_json::from_str(&body).context("Failed to parse /quote JSON")?;

    // API returns an array at top level
    if let Some(arr) = json.as_array() {
        return Ok(arr.clone());
    }
    Err(anyhow!("Unexpected /quote response format: {}", body))
}

/// Build Solana swap transaction from Mayan API.
/// Returns the full response JSON (contains serialized base64 tx or instruction fields).
pub async fn get_swap_solana(
    client: &Client,
    amount_in64: &str,
    from_token: &str,
    user_wallet: &str,
    slippage_bps: u32,
    to_chain_name: &str,
    deposit_mode: &str,
    middle_token: Option<&str>,
    min_middle_amount: Option<f64>,
    to_token: &str,
    referrer_address: Option<&str>,
) -> anyhow::Result<Value> {
    let mut params = vec![
        ("amountIn64", amount_in64.to_string()),
        ("fromToken", from_token.to_string()),
        ("userWallet", user_wallet.to_string()),
        ("slippageBps", slippage_bps.to_string()),
        ("chainName", to_chain_name.to_string()),
        ("depositMode", deposit_mode.to_string()),
        ("toToken", to_token.to_string()),
    ];
    if let Some(mt) = middle_token {
        if !mt.is_empty() {
            params.push(("middleToken", mt.to_string()));
        }
    }
    if let Some(mma) = min_middle_amount {
        params.push(("minMiddleAmount", mma.to_string()));
    }
    if let Some(ref_addr) = referrer_address {
        if !ref_addr.is_empty() {
            params.push(("referrerAddress", ref_addr.to_string()));
        }
    }

    let url = format!("{}/get-swap/solana", PRICE_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to reach Mayan price API /get-swap/solana")?;

    let status = resp.status();
    let body = resp.text().await.context("Failed to read /get-swap/solana response body")?;

    if !status.is_success() {
        return Err(anyhow!(
            "Mayan /get-swap/solana returned HTTP {}: {}",
            status,
            body
        ));
    }

    serde_json::from_str(&body).context("Failed to parse /get-swap/solana JSON")
}

/// Build EVM swap calldata from Mayan API.
/// Returns JSON with swapRouterAddress + swapRouterCalldata (or tx.to + tx.data).
pub async fn get_swap_evm(
    client: &Client,
    amount_in64: &str,
    from_token: &str,
    from_chain_name: &str,
    to_token: &str,
    to_chain_name: &str,
    slippage_bps: u32,
    destination_address: &str,
    middle_token: Option<&str>,
    referrer_address: Option<&str>,
) -> anyhow::Result<Value> {
    let mut params = vec![
        ("amountIn64", amount_in64.to_string()),
        ("fromToken", from_token.to_string()),
        ("forwarderAddress", MAYAN_FORWARDER_CONTRACT.to_string()),
        ("chainName", from_chain_name.to_string()),
        ("toToken", to_token.to_string()),
        ("toChainName", to_chain_name.to_string()),
        ("slippageBps", slippage_bps.to_string()),
        ("destinationAddress", destination_address.to_string()),
    ];
    if let Some(mt) = middle_token {
        if !mt.is_empty() {
            params.push(("middleToken", mt.to_string()));
        }
    }
    if let Some(ref_addr) = referrer_address {
        if !ref_addr.is_empty() {
            params.push(("referrerAddress", ref_addr.to_string()));
        }
    }

    let url = format!("{}/get-swap/evm", PRICE_API_BASE);
    let resp = client
        .get(&url)
        .query(&params)
        .send()
        .await
        .context("Failed to reach Mayan price API /get-swap/evm")?;

    let status = resp.status();
    let body = resp.text().await.context("Failed to read /get-swap/evm response body")?;

    if !status.is_success() {
        return Err(anyhow!(
            "Mayan /get-swap/evm returned HTTP {}: {}",
            status,
            body
        ));
    }

    serde_json::from_str(&body).context("Failed to parse /get-swap/evm JSON")
}

/// Poll swap status from Mayan explorer API by source tx hash.
pub async fn get_swap_status(client: &Client, tx_hash: &str) -> anyhow::Result<Value> {
    let url = format!("{}/swap/trx/{}", EXPLORER_API_BASE, tx_hash);
    let resp = client
        .get(&url)
        .send()
        .await
        .context("Failed to reach Mayan explorer API")?;

    let status = resp.status();
    let body = resp.text().await.context("Failed to read status response body")?;

    if !status.is_success() {
        return Err(anyhow!(
            "Mayan explorer API returned HTTP {}: {}",
            status,
            body
        ));
    }

    serde_json::from_str(&body).context("Failed to parse swap status JSON")
}

/// Extract the best route from a list of quotes.
/// Prefers SWIFT, then MCTP, then WH (or first available if none match).
pub fn pick_best_route(quotes: &[Value]) -> Option<&Value> {
    // Prefer SWIFT (fastest)
    for q in quotes {
        if q["type"].as_str() == Some("SWIFT") {
            return Some(q);
        }
    }
    // Then MCTP (good for stablecoins)
    for q in quotes {
        if q["type"].as_str() == Some("MCTP") {
            return Some(q);
        }
    }
    // Then WH (Wormhole)
    for q in quotes {
        if q["type"].as_str() == Some("WH") {
            return Some(q);
        }
    }
    quotes.first()
}

/// Determine Mayan deposit mode from route type
pub fn route_type_to_deposit_mode(route_type: &str) -> &'static str {
    match route_type {
        "MCTP" => "WITH_FEE",
        "SWIFT" => "SWIFT",
        "WH" => "WITH_FEE",
        _ => "WITH_FEE",
    }
}
