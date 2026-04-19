/// get-quote: Get a Jupiter swap quote (output amount, price impact, route plan).
/// No on-chain action — read-only API call.
use anyhow::Result;
use clap::Args;

use crate::config::{resolve_mint, to_raw_amount, from_raw_amount, DEFAULT_SLIPPAGE_BPS};
use crate::api;

#[derive(Args, Debug)]
pub struct GetQuoteArgs {
    /// Input token symbol (SOL, USDC, USDT) or raw mint address
    #[arg(long)]
    pub input_mint: String,

    /// Output token symbol (SOL, USDC, USDT) or raw mint address
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in UI units (e.g. 0.1 for 0.1 SOL)
    #[arg(long)]
    pub amount: f64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u32,
}

pub async fn execute(args: &GetQuoteArgs) -> Result<()> {
    let input_mint = resolve_mint(&args.input_mint).to_string();
    let output_mint = resolve_mint(&args.output_mint).to_string();

    let raw_amount = to_raw_amount(args.amount, &input_mint);

    let resp = api::get_order(&input_mint, &output_mint, raw_amount, args.slippage_bps, None).await?;

    // Extract relevant fields for clean output
    let out_amount_raw = resp["outAmount"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| resp["outAmount"].as_u64())
        .unwrap_or(0);

    let out_amount_ui = from_raw_amount(out_amount_raw, &output_mint);

    let price_impact = resp["priceImpactPct"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| resp["priceImpactPct"].to_string());

    // Extract route plan summary
    let route: Vec<String> = resp["routePlan"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|step| {
                    step["swapInfo"]["label"]
                        .as_str()
                        .map(|s| s.to_string())
                        .or_else(|| step["swapInfo"]["ammKey"].as_str().map(|s| s.to_string()))
                })
                .collect()
        })
        .unwrap_or_default();

    let output = serde_json::json!({
        "input": format!("{} {}", args.amount, args.input_mint.to_uppercase()),
        "output": format!("{:.6} {}", out_amount_ui, args.output_mint.to_uppercase()),
        "price_impact": format!("{}%", price_impact),
        "route": route,
        "slippage_bps": args.slippage_bps,
        "raw": {
            "inputMint": input_mint,
            "outputMint": output_mint,
            "inAmount": raw_amount,
            "outAmount": out_amount_raw
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
