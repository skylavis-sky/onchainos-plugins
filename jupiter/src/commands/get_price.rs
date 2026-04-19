/// get-price: Get real-time USD price for a token via Jupiter Price API v3.
use anyhow::Result;
use clap::Args;

use crate::api;
use crate::config::{resolve_mint, USDC_MINT};

#[derive(Args, Debug)]
pub struct GetPriceArgs {
    /// Token symbol (SOL, USDC, JUP) or raw mint address to price
    #[arg(long)]
    pub token: String,

    /// Denominator token symbol or mint (default: USDC)
    #[arg(long, default_value = "USDC")]
    pub vs_token: String,
}

pub async fn execute(args: &GetPriceArgs) -> Result<()> {
    let token_mint = resolve_mint(&args.token).to_string();
    let vs_mint = resolve_mint(&args.vs_token).to_string();

    // Use USDC mint as default vs_token if the resolved value is the raw "USDC" symbol
    let vs_mint_resolved = if vs_mint == "USDC" {
        USDC_MINT.to_string()
    } else {
        vs_mint.clone()
    };

    let resp = api::get_price(&token_mint, &vs_mint_resolved).await?;

    // Jupiter Price API v3 response shape: { "<mint>": { "usdPrice": ..., "priceChange24h": ..., "liquidity": ... } }
    // NOTE: no "data" wrapper — the mint key is at the top level
    let token_data = &resp[&token_mint];

    let price = token_data["usdPrice"]
        .as_f64()
        .map(|p| format!("{:.6}", p))
        .unwrap_or_else(|| token_data["usdPrice"].to_string());

    let change_24h = token_data["priceChange24h"]
        .as_f64()
        .map(|c| format!("{:.2}%", c))
        .unwrap_or_else(|| "N/A".to_string());

    let output = serde_json::json!({
        "token": args.token.to_uppercase(),
        "mint": token_mint,
        "price": price,
        "vs": args.vs_token.to_uppercase(),
        "vs_mint": vs_mint_resolved,
        "price_change_24h": change_24h
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
