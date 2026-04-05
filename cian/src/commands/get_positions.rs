use anyhow::Result;
use crate::config::{VAULTS, chain_display_name, rpc_url};
use crate::rpc;

/// Query user position in a CIAN vault using on-chain reads.
pub async fn run(chain_id: u64, vault_addr: &str, wallet_addr: &str) -> Result<()> {
    let rpc = rpc_url(chain_id);

    // Look up vault metadata
    let vault_meta = VAULTS.iter().find(|v| {
        v.chain_id == chain_id && v.address.to_lowercase() == vault_addr.to_lowercase()
    });

    let vault_name = vault_meta.map(|v| v.name).unwrap_or("Unknown Vault");
    let strategy = vault_meta.map(|v| v.strategy).unwrap_or("Unknown");

    // Read shares balance
    let shares_raw = rpc::get_balance_of(vault_addr, wallet_addr, rpc).await
        .map_err(|e| anyhow::anyhow!("Failed to read balanceOf: {e}"))?;

    // Read vault decimals
    let decimals = rpc::get_decimals(vault_addr, rpc).await.unwrap_or(18) as u32;
    let divisor = 10u128.pow(decimals);

    // Convert shares to assets (ERC4626: convertToAssets)
    let assets_raw = if shares_raw > 0 {
        rpc::convert_to_assets(vault_addr, shares_raw, rpc).await.unwrap_or(shares_raw)
    } else {
        0
    };

    let shares_human = shares_raw as f64 / divisor as f64;
    let assets_human = assets_raw as f64 / divisor as f64;

    println!("=== CIAN Yield Layer Position ===");
    println!("Chain:    {} ({})", chain_id, chain_display_name(chain_id));
    println!("Vault:    {}", vault_addr);
    println!("Name:     {}", vault_name);
    println!("Strategy: {}", strategy);
    println!("Wallet:   {}", wallet_addr);
    println!();

    if shares_raw == 0 {
        println!("No position found. Shares balance: 0.");
        println!();
        println!(
            "Use 'cian deposit --chain {} --vault {} --token <token_addr> --amount <amount>' to open a position.",
            chain_id, vault_addr
        );
        return Ok(());
    }

    println!("Shares:       {:.6} yl-tokens", shares_human);
    println!("Asset Value:  {:.6} (underlying)", assets_human);
    println!();
    println!("Note: requestRedeem initiates a queued withdrawal (hours to days, not instant).");
    println!(
        "Use 'cian request-withdraw --chain {} --vault {} --shares {:.6}' to withdraw.",
        chain_id, vault_addr, shares_human
    );

    Ok(())
}
