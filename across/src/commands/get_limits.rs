use crate::api;

pub struct GetLimitsArgs {
    pub input_token: String,
    pub output_token: String,
    pub origin_chain_id: u64,
    pub destination_chain_id: u64,
}

pub async fn run(args: GetLimitsArgs) -> anyhow::Result<()> {
    let limits = api::get_limits(
        &args.input_token,
        &args.output_token,
        args.origin_chain_id,
        args.destination_chain_id,
    )
    .await?;

    println!("=== Across Protocol Transfer Limits ===");
    println!(
        "Route: Chain {} -> Chain {}",
        args.origin_chain_id, args.destination_chain_id
    );
    println!(
        "Input token:                {}",
        args.input_token
    );
    println!(
        "Output token:               {}",
        args.output_token
    );
    println!();
    println!(
        "Min deposit:                {}",
        limits["minDeposit"].as_str().unwrap_or("N/A")
    );
    println!(
        "Max deposit:                {}",
        limits["maxDeposit"].as_str().unwrap_or("N/A")
    );
    println!(
        "Max deposit (instant):      {}",
        limits["maxDepositInstant"].as_str().unwrap_or("N/A")
    );
    println!(
        "Max deposit (short delay):  {}",
        limits["maxDepositShortDelay"].as_str().unwrap_or("N/A")
    );
    println!(
        "Recommended instant:        {}",
        limits["recommendedDepositInstant"].as_str().unwrap_or("N/A")
    );
    println!(
        "Liquid reserves:            {}",
        limits["liquidReserves"].as_str().unwrap_or("N/A")
    );
    println!(
        "Utilized reserves:          {}",
        limits["utilizedReserves"].as_str().unwrap_or("N/A")
    );

    println!("\n--- Raw JSON ---");
    println!("{}", serde_json::to_string_pretty(&limits).unwrap_or_default());

    Ok(())
}
