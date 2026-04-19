/// get-tokens: Search for SPL tokens by symbol/name or list verified tokens.
use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::api;
use crate::config::DEFAULT_TOKENS_LIMIT;

#[derive(Args, Debug)]
pub struct GetTokensArgs {
    /// Search query: token symbol, name, or mint address (optional)
    #[arg(long)]
    pub search: Option<String>,

    /// Maximum number of results to return (default: 20)
    #[arg(long, default_value_t = DEFAULT_TOKENS_LIMIT)]
    pub limit: usize,
}

pub async fn execute(args: &GetTokensArgs) -> Result<()> {
    let resp = api::get_tokens(args.search.as_deref(), args.limit).await?;

    // Normalize response to a flat list
    let tokens: Vec<Value> = match resp.as_array() {
        Some(arr) => arr
            .iter()
            .take(args.limit)
            .map(|t| {
                serde_json::json!({
                    "symbol": t["symbol"].as_str().unwrap_or(""),
                    "name": t["name"].as_str().unwrap_or(""),
                    "mint": t["id"].as_str()
                        .or_else(|| t["address"].as_str())
                        .or_else(|| t["mint"].as_str())
                        .unwrap_or(""),
                    "decimals": t["decimals"].as_u64().unwrap_or(9),
                    "verified": t["tags"].as_array()
                        .map(|tags| tags.iter().any(|tag| tag.as_str() == Some("verified")))
                        .unwrap_or(false)
                })
            })
            .collect(),
        None => {
            // Response might be wrapped: { "tokens": [...] } or similar
            let inner = resp["tokens"]
                .as_array()
                .or_else(|| resp["data"].as_array());
            match inner {
                Some(arr) => arr
                    .iter()
                    .take(args.limit)
                    .map(|t| {
                        serde_json::json!({
                            "symbol": t["symbol"].as_str().unwrap_or(""),
                            "name": t["name"].as_str().unwrap_or(""),
                            "mint": t["id"].as_str()
                                .or_else(|| t["address"].as_str())
                                .or_else(|| t["mint"].as_str())
                                .unwrap_or(""),
                            "decimals": t["decimals"].as_u64().unwrap_or(9),
                            "verified": t["tags"].as_array()
                                .map(|tags| tags.iter().any(|tag| tag.as_str() == Some("verified")))
                                .unwrap_or(false)
                        })
                    })
                    .collect(),
                None => {
                    // Fall back to raw response
                    println!("{}", serde_json::to_string_pretty(&resp)?);
                    return Ok(());
                }
            }
        }
    };

    let output = serde_json::json!({
        "count": tokens.len(),
        "tokens": tokens
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
