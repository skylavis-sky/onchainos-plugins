use crate::api;

pub struct GetRoutesArgs {
    pub origin_chain_id: Option<u64>,
    pub destination_chain_id: Option<u64>,
    pub origin_token: Option<String>,
    pub destination_token: Option<String>,
}

pub async fn run(args: GetRoutesArgs) -> anyhow::Result<()> {
    let routes = api::get_available_routes(
        args.origin_chain_id,
        args.destination_chain_id,
        args.origin_token.as_deref(),
        args.destination_token.as_deref(),
    )
    .await?;

    if let Some(arr) = routes.as_array() {
        if arr.is_empty() {
            println!("No routes found matching the given filters.");
            return Ok(());
        }
        println!("=== Available Routes ({} found) ===", arr.len());
        for (i, route) in arr.iter().enumerate() {
            let origin_chain = route["originChainId"].as_u64().unwrap_or(0);
            let dest_chain = route["destinationChainId"].as_u64().unwrap_or(0);
            let origin_sym = route["originTokenSymbol"].as_str().unwrap_or("?");
            let dest_sym = route["destinationTokenSymbol"].as_str().unwrap_or("?");
            let origin_tok = route["originToken"].as_str().unwrap_or("?");
            let dest_tok = route["destinationToken"].as_str().unwrap_or("?");
            let is_native = route["isNative"].as_bool().unwrap_or(false);
            println!(
                "[{}] Chain {} ({}) -> Chain {} ({}) | Native: {}",
                i + 1,
                origin_chain,
                origin_sym,
                dest_chain,
                dest_sym,
                is_native
            );
            println!("     Origin token:      {}", origin_tok);
            println!("     Destination token: {}", dest_tok);
        }
    } else {
        println!("Unexpected response format:");
        println!("{}", serde_json::to_string_pretty(&routes).unwrap_or_default());
    }

    Ok(())
}
