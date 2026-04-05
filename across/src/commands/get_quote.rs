use anyhow::Context;
use serde_json::Value;

use crate::api;

pub struct GetQuoteArgs {
    pub input_token: String,
    pub output_token: String,
    pub origin_chain_id: u64,
    pub destination_chain_id: u64,
    pub amount: String,
    pub depositor: Option<String>,
    pub recipient: Option<String>,
}

pub async fn run(args: GetQuoteArgs) -> anyhow::Result<()> {
    let quote = api::get_suggested_fees(
        &args.input_token,
        &args.output_token,
        args.origin_chain_id,
        args.destination_chain_id,
        &args.amount,
        args.depositor.as_deref(),
        args.recipient.as_deref(),
    )
    .await
    .context("Failed to fetch quote")?;

    // Check isAmountTooLow
    if quote["isAmountTooLow"].as_bool().unwrap_or(false) {
        let min = quote["limits"]["minDeposit"].as_str().unwrap_or("unknown");
        anyhow::bail!(
            "Amount too low. Minimum deposit is {} (in token base units). \
             Please increase your transfer amount.",
            min
        );
    }

    print_quote(&quote);
    Ok(())
}

pub fn print_quote(quote: &Value) {
    let output_amount = quote["outputAmount"].as_str().unwrap_or("N/A");
    let input_sym = quote["inputToken"]["symbol"].as_str().unwrap_or("?");
    let output_sym = quote["outputToken"]["symbol"].as_str().unwrap_or("?");
    let total_fee = quote["totalRelayFee"]["total"].as_str().unwrap_or("N/A");
    let fill_time = quote["estimatedFillTimeSec"]
        .as_u64()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let timestamp = quote["timestamp"].as_str().unwrap_or("N/A");
    let fill_deadline = quote["fillDeadline"].as_str().unwrap_or("N/A");
    let spoke_pool = quote["spokePoolAddress"].as_str().unwrap_or("N/A");

    let lp_fee_pct = quote["lpFee"]["pct"].as_str().unwrap_or("0");
    let gas_fee_total = quote["relayerGasFee"]["total"].as_str().unwrap_or("0");
    let capital_fee_total = quote["relayerCapitalFee"]["total"].as_str().unwrap_or("0");

    println!("=== Across Protocol Quote ===");
    println!("Input token:        {} ({})", input_sym, quote["inputToken"]["address"].as_str().unwrap_or("N/A"));
    println!("Output token:       {} ({})", output_sym, quote["outputToken"]["address"].as_str().unwrap_or("N/A"));
    println!("Output amount:      {} {} (after fees)", output_amount, output_sym);
    println!("--- Fee Breakdown ---");
    println!("Total relay fee:    {} {}", total_fee, input_sym);
    println!("  Capital fee:      {}", capital_fee_total);
    println!("  Gas fee:          {}", gas_fee_total);
    println!("  LP fee pct:       {} (1e18 = 100%)", lp_fee_pct);
    println!("--- Timing ---");
    println!("Est. fill time:     {} seconds", fill_time);
    println!("Quote timestamp:    {}", timestamp);
    println!("Fill deadline:      {}", fill_deadline);
    println!("SpokePool address:  {}", spoke_pool);
    println!("Is amount too low:  {}", quote["isAmountTooLow"].as_bool().unwrap_or(false));

    // Print full JSON for programmatic use
    println!("\n--- Raw JSON ---");
    println!("{}", serde_json::to_string_pretty(quote).unwrap_or_default());
}
