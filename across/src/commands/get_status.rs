use crate::api;

pub struct GetStatusArgs {
    pub deposit_txn_ref: Option<String>,
    pub deposit_id: Option<u64>,
    pub origin_chain_id: Option<u64>,
    pub relay_data_hash: Option<String>,
}

pub async fn run(args: GetStatusArgs) -> anyhow::Result<()> {
    // Validate that at least one lookup param is provided
    if args.deposit_txn_ref.is_none()
        && args.deposit_id.is_none()
        && args.relay_data_hash.is_none()
    {
        anyhow::bail!(
            "Must provide at least one of: --tx-hash, --deposit-id (with --origin-chain-id), or --relay-data-hash"
        );
    }

    let status = api::get_deposit_status(
        args.deposit_txn_ref.as_deref(),
        args.deposit_id,
        args.origin_chain_id,
        args.relay_data_hash.as_deref(),
    )
    .await?;

    print_status(&status);
    Ok(())
}

pub fn print_status(status: &serde_json::Value) {
    let fill_status = status["status"].as_str().unwrap_or("unknown");
    let deposit_id = status["depositId"]
        .as_u64()
        .map(|id| id.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let origin_chain = status["originChainId"].as_u64().unwrap_or(0);
    let dest_chain = status["destinationChainId"].as_u64().unwrap_or(0);
    let deposit_tx = status["depositTxnHash"].as_str().unwrap_or("N/A");
    let fill_tx = status["fillTxnHash"].as_str().unwrap_or("pending");
    let refund_tx = status["depositRefundTxnHash"].as_str().unwrap_or("none");

    println!("=== Deposit Status ===");
    println!("Status:             {}", fill_status);
    println!("Deposit ID:         {}", deposit_id);
    println!("Origin chain:       {}", origin_chain);
    println!("Destination chain:  {}", dest_chain);
    println!("Deposit tx:         {}", deposit_tx);
    println!("Fill tx:            {}", fill_tx);
    println!("Refund tx:          {}", refund_tx);

    match fill_status {
        "filled" => println!("\nBridge complete. Funds delivered on destination chain."),
        "pending" => println!("\nBridge in progress. Check again in a few seconds."),
        "expired" => println!("\nDeposit expired. Refund tx: {}", refund_tx),
        _ => {}
    }
}
