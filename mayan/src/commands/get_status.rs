use reqwest::Client;

use crate::api::get_swap_status;

pub struct GetStatusArgs {
    pub tx_hash: String,
    pub chain: Option<u64>,
}

pub async fn run(args: GetStatusArgs) -> anyhow::Result<()> {
    let client = Client::new();

    println!("Fetching swap status for tx: {}", args.tx_hash);
    if let Some(chain_id) = args.chain {
        println!("  Source chain ID: {}", chain_id);
    }
    println!();

    let status_json = get_swap_status(&client, &args.tx_hash).await?;

    // Print key fields
    let client_status = status_json["clientStatus"].as_str().unwrap_or("UNKNOWN");
    let status = status_json["status"].as_str().unwrap_or("UNKNOWN");
    let source_chain = status_json["sourceChain"].as_str().unwrap_or("?");
    let dest_chain = status_json["destChain"].as_str().unwrap_or("?");
    let from_amount = status_json["fromAmount"].as_str()
        .or_else(|| status_json["fromAmount"].as_f64().map(|_| "").into())
        .unwrap_or("?");
    let to_amount = status_json["toAmount"].as_str()
        .or_else(|| status_json["toAmount"].as_f64().map(|_| "").into())
        .unwrap_or("?");
    let from_symbol = status_json["fromTokenSymbol"].as_str().unwrap_or("?");
    let to_symbol = status_json["toTokenSymbol"].as_str().unwrap_or("?");
    let initiated_at = status_json["initiatedAt"].as_str().unwrap_or("?");
    let completed_at = status_json["completedAt"].as_str().unwrap_or("(pending)");
    let dest_address = status_json["destAddress"].as_str().unwrap_or("?");
    let source_tx_hash = status_json["sourceTxHash"].as_str().unwrap_or(&args.tx_hash);

    println!("Swap Status");
    println!("{:-<50}", "");
    println!("  Client Status:    {}", client_status);
    println!("  Internal Status:  {}", status);
    println!("  Route:            {} -> {}", source_chain, dest_chain);
    println!("  Amount In:        {} {}", from_amount, from_symbol);
    println!("  Amount Out:       {} {}", to_amount, to_symbol);
    println!("  Destination:      {}", dest_address);
    println!("  Source Tx Hash:   {}", source_tx_hash);
    println!("  Initiated At:     {}", initiated_at);
    println!("  Completed At:     {}", completed_at);

    // Print full JSON for debugging
    println!("\nFull response:");
    println!("{}", serde_json::to_string_pretty(&status_json)?);

    Ok(())
}
