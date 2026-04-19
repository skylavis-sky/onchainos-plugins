use anyhow::Result;
use serde_json::Value;

/// Call Jupiter Swap API v2 /order endpoint.
/// Returns the full JSON response which contains both quote fields and `transaction` (base64 unsigned tx).
pub async fn get_order(
    input_mint: &str,
    output_mint: &str,
    amount: u64,
    slippage_bps: u32,
    taker: Option<&str>,
) -> Result<Value> {
    let client = reqwest::Client::new();
    let url = format!("{}/order", crate::config::SWAP_API_BASE);

    let mut query: Vec<(&str, String)> = vec![
        ("inputMint", input_mint.to_string()),
        ("outputMint", output_mint.to_string()),
        ("amount", amount.to_string()),
        ("slippageBps", slippage_bps.to_string()),
        ("onlyDirectRoutes", "false".to_string()),
    ];

    if let Some(wallet) = taker {
        query.push(("taker", wallet.to_string()));
    }

    let resp: Value = client.get(&url).query(&query).send().await?.json().await?;
    Ok(resp)
}

/// Call Jupiter Price API v3.
/// `ids`: comma-separated list of mint addresses.
/// `vs_token`: the denominator token mint (default: USDC).
pub async fn get_price(ids: &str, vs_token: &str) -> Result<Value> {
    let client = reqwest::Client::new();
    let resp: Value = client
        .get(crate::config::PRICE_API_BASE)
        .query(&[("ids", ids), ("vsToken", vs_token)])
        .send()
        .await?
        .json()
        .await?;
    Ok(resp)
}

/// Call Jupiter Tokens API — fetch tokens list or search by query.
/// Uses the Jupiter Tokens v2 search endpoint for both search and listing.
/// If `query` is None, defaults to searching well-known tokens (SOL, USDC, USDT, JUP).
pub async fn get_tokens(query: Option<&str>, limit: usize) -> Result<Value> {
    let client = reqwest::Client::new();

    // Use search endpoint for both cases; default query lists major tokens
    let q = query.unwrap_or("SOL");
    let resp: Value = client
        .get(crate::config::TOKENS_SEARCH_API)
        .query(&[("query", q)])
        .send()
        .await?
        .json()
        .await?;

    // Trim to limit if it's an array
    if let Some(arr) = resp.as_array() {
        let trimmed: Vec<Value> = arr.iter().take(limit).cloned().collect();
        return Ok(Value::Array(trimmed));
    }
    Ok(resp)
}
