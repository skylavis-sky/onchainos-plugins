use anyhow::Context;

use crate::api;

pub async fn run() -> anyhow::Result<()> {
    println!("Fetching supported chains from deBridge DLN...");

    let resp = api::get_supported_chains()
        .await
        .context("Failed to fetch supported chains")?;

    println!("\n=== deBridge DLN Supported Chains ===");

    // API returns {"chains": [...]} envelope
    let chains_value = if resp.is_object() {
        resp.get("chains").cloned().unwrap_or(resp.clone())
    } else {
        resp.clone()
    };
    if let Some(chains) = chains_value.as_array() {
        println!("{:<12} {}", "Chain ID", "Chain Name");
        println!("{}", "-".repeat(40));
        for chain in chains {
            let id = chain["chainId"]
                .as_u64()
                .map(|n| n.to_string())
                .or_else(|| chain["chainId"].as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "?".to_string());
            let name = chain["chainName"].as_str().unwrap_or("Unknown");
            // Mark Solana special case
            let note = if id == "7565164" {
                " (Solana — onchainos chain ID: 501)"
            } else {
                ""
            };
            println!("{:<12} {}{}", id, name, note);
        }
        println!("\nTotal: {} chains", chains.len());
    } else {
        println!("No chains found in response.");
        println!("{}", serde_json::to_string_pretty(&chains_value).unwrap_or_default());
    }

    Ok(())
}
