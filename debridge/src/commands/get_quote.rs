use anyhow::Context;
use serde_json::Value;

use crate::api::{self, CreateTxParams};
use crate::config::onchainos_to_debridge_chain;

pub struct GetQuoteArgs {
    pub src_chain_id: u64,
    pub dst_chain_id: u64,
    pub src_token: String,
    pub dst_token: String,
    pub amount: String,
}

pub async fn run(args: GetQuoteArgs) -> anyhow::Result<()> {
    let src_db = onchainos_to_debridge_chain(args.src_chain_id);
    let dst_db = onchainos_to_debridge_chain(args.dst_chain_id);

    println!("Fetching quote from deBridge DLN...");
    println!("  src chain: {} (deBridge ID: {})", args.src_chain_id, src_db);
    println!("  dst chain: {} (deBridge ID: {})", args.dst_chain_id, dst_db);
    println!("  src token: {}", args.src_token);
    println!("  dst token: {}", args.dst_token);
    println!("  amount:    {}", args.amount);

    let params = CreateTxParams {
        src_chain_id: &src_db,
        src_token: &args.src_token,
        src_amount: &args.amount,
        dst_chain_id: &dst_db,
        dst_token: &args.dst_token,
        // Quote-only: omit authority/recipient so API returns estimation only
        src_authority: None,
        dst_authority: None,
        dst_recipient: None,
        skip_solana_recipient_validation: false,
    };

    let resp = api::create_tx(&params)
        .await
        .context("Failed to fetch quote from deBridge DLN")?;

    print_quote(&resp);
    Ok(())
}

pub fn print_quote(resp: &Value) {
    println!("\n=== deBridge DLN Quote ===");

    let est = &resp["estimation"];
    let src_in = &est["srcChainTokenIn"];
    let dst_out = &est["dstChainTokenOut"];

    let src_symbol = src_in["symbol"].as_str().unwrap_or("?");
    let src_amount = src_in["amount"].as_str().unwrap_or("N/A");
    let src_usd = src_in["approximateUsdValue"].as_f64().unwrap_or(0.0);
    let src_decimals = src_in["decimals"].as_u64().unwrap_or(0);

    let dst_symbol = dst_out["symbol"].as_str().unwrap_or("?");
    let dst_amount = dst_out["amount"].as_str().unwrap_or("N/A");
    let dst_recommended = dst_out["recommendedAmount"].as_str().unwrap_or(dst_amount);
    let dst_usd = dst_out["approximateUsdValue"].as_f64().unwrap_or(0.0);
    let dst_decimals = dst_out["decimals"].as_u64().unwrap_or(0);

    let fix_fee = resp["fixFee"].as_str().unwrap_or("N/A");
    let fill_delay = resp["order"]["approximateFulfillmentDelay"]
        .as_u64()
        .unwrap_or(0);

    println!(
        "Input:              {} {} (decimals={}, ~${:.4})",
        src_amount, src_symbol, src_decimals, src_usd
    );
    println!(
        "Output (estimated): {} {} (decimals={}, ~${:.4})",
        dst_amount, dst_symbol, dst_decimals, dst_usd
    );
    println!(
        "Output (recommended amount): {}",
        dst_recommended
    );
    println!("Protocol fix fee:   {} wei", fix_fee);
    println!("Est. fill time:     ~{} seconds", fill_delay);

    // Cost details
    if let Some(costs) = est["costsDetails"].as_array() {
        if !costs.is_empty() {
            println!("--- Cost Details ---");
            for cost in costs {
                let chain = cost["chain"].as_str().unwrap_or("?");
                let token_sym = cost["tokenOut"]["symbol"].as_str().unwrap_or("?");
                let percent = cost["payload"]["percent"].as_f64().unwrap_or(0.0);
                println!("  chain {}: {}%  (in {})", chain, percent, token_sym);
            }
        }
    }

    println!("\n--- Raw JSON ---");
    println!("{}", serde_json::to_string_pretty(resp).unwrap_or_default());
}
