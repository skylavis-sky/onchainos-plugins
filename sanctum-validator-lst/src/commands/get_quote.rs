// get-quote: quote a swap between two LSTs via Sanctum Router v2.

use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::api;
use crate::config::{self, LST_DECIMALS};

#[derive(Args)]
pub struct GetQuoteArgs {
    /// Input LST symbol (e.g. jitoSOL) or mint address
    #[arg(long)]
    pub from: String,

    /// Output LST symbol (e.g. bSOL) or mint address
    #[arg(long)]
    pub to: String,

    /// Input amount in UI units (e.g. 0.1 for 0.1 jitoSOL)
    #[arg(long)]
    pub amount: f64,

    /// Slippage tolerance in percent (default: 0.5)
    #[arg(long, default_value_t = 0.5)]
    pub slippage: f64,
}

pub async fn run(args: GetQuoteArgs) -> Result<Value> {
    if args.amount <= 0.0 {
        anyhow::bail!("Amount must be positive");
    }

    let input_mint = config::resolve_mint(&args.from);
    let output_mint = config::resolve_mint(&args.to);

    let amount_atomics = api::ui_to_atomics(args.amount, LST_DECIMALS);

    let client = reqwest::Client::new();
    let quote = api::get_swap_quote(&client, input_mint, output_mint, amount_atomics).await?;

    let out_amount: u64 = quote.out_amount.parse().unwrap_or(0);
    let in_amount: u64 = quote.in_amount.parse().unwrap_or(0);
    let min_out = api::apply_slippage(out_amount, args.slippage);

    let in_ui = api::atomics_to_ui(in_amount, LST_DECIMALS);
    let out_ui = api::atomics_to_ui(out_amount, LST_DECIMALS);
    let min_out_ui = api::atomics_to_ui(min_out, LST_DECIMALS);

    let rate = if in_ui > 0.0 { out_ui / in_ui } else { 0.0 };

    // Fees summary
    let fee_summary: Vec<Value> = quote.fees.iter().map(|f| {
        let fee_ui = f.amt.parse::<u64>().ok()
            .map(|a| api::atomics_to_ui(a, LST_DECIMALS));
        serde_json::json!({
            "code": f.code,
            "rate": f.rate,
            "amount_ui": fee_ui.map(|v| format!("{:.9}", v)).unwrap_or_else(|| f.amt.clone()),
            "mint": f.mint
        })
    }).collect();

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "from": args.from,
            "to": args.to,
            "input_mint": input_mint,
            "output_mint": output_mint,
            "in_amount_ui": format!("{:.9}", in_ui),
            "out_amount_ui": format!("{:.9}", out_ui),
            "min_out_ui": format!("{:.9}", min_out_ui),
            "rate": format!("{:.8}", rate),
            "slippage_pct": args.slippage,
            "swap_src": quote.swap_src,
            "fees": fee_summary
        }
    }))
}
