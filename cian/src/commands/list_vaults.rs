use anyhow::Result;
use crate::config::{VAULTS, chain_display_name, rpc_url};
use crate::rpc;

/// List all known CIAN vaults on a given chain with on-chain TVL.
pub async fn run(chain_id: u64) -> Result<()> {
    let vaults: Vec<_> = VAULTS.iter().filter(|v| v.chain_id == chain_id).collect();

    if vaults.is_empty() {
        println!(
            "No CIAN vaults registered for chain {}. Supported chains: 1 (Ethereum), 42161 (Arbitrum), 56 (BSC), 5000 (Mantle).",
            chain_id
        );
        return Ok(());
    }

    let rpc = rpc_url(chain_id);

    println!("{:<45} {:<28} {:<14} {:>20}", "Vault Address", "Name", "Strategy", "Total Assets");
    println!("{}", "-".repeat(115));

    for v in &vaults {
        let assets_raw = rpc::get_total_assets(v.address, rpc).await.unwrap_or(0);
        let decimals = rpc::get_decimals(v.address, rpc).await.unwrap_or(18) as u32;
        let divisor = 10u128.pow(decimals);
        let assets_human = if divisor > 0 {
            format!("{:.4}", assets_raw as f64 / divisor as f64)
        } else {
            assets_raw.to_string()
        };

        println!(
            "{:<45} {:<28} {:<14} {:>20}",
            v.address, v.name, v.strategy, assets_human
        );
    }

    println!();
    println!("Chain: {} ({})", chain_id, chain_display_name(chain_id));
    println!("Total vaults: {}", vaults.len());
    println!();
    println!(
        "Use 'cian get-positions --chain {} --vault <addr> --wallet <addr>' to view your position.",
        chain_id
    );
    println!(
        "Use 'cian deposit --chain {} --vault <addr> --token <addr> --amount <amount>' to deposit.",
        chain_id
    );

    Ok(())
}
