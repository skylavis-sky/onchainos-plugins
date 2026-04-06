use anyhow::Result;
use crate::abi::{encode_request_redeem_eth, encode_request_redeem_btc, to_raw};
use crate::config::{chain_display_name, is_btc_class_vault, ZERO_ADDRESS};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash, is_ok};

/// Request withdrawal from a CIAN vault.
///
/// Detects vault type:
/// - BTC-class (pumpBTC): requestRedeem(uint256)           selector 0xaa2f892d
/// - ETH-class (all others): requestRedeem(uint256,address) selector 0x107703ab
///
/// Note: requestRedeem is non-instant. The request enters a queue and is
/// processed by the rebalancer asynchronously (hours to days).
pub async fn run(
    chain_id: u64,
    vault_addr: &str,
    shares: f64,
    token_addr: Option<&str>,
    decimals: u32,
    dry_run: bool,
) -> Result<()> {
    if shares <= 0.0 {
        anyhow::bail!("Shares amount must be greater than 0");
    }

    let wallet = resolve_wallet(chain_id).unwrap_or_else(|_| {
        if dry_run { "0x0000000000000000000000000000000000000000".to_string() } else { "".to_string() }
    });
    if !dry_run && wallet.is_empty() {
        anyhow::bail!("Could not resolve wallet address for chain {}.", chain_id);
    }
    let raw_shares = to_raw(shares, decimals);
    if raw_shares == 0 {
        anyhow::bail!("Shares amount is too small (rounds to 0 at {} decimals)", decimals);
    }

    // Detect vault type: BTC-class uses single-param requestRedeem
    let btc_class = is_btc_class_vault(vault_addr);

    println!("=== CIAN Yield Layer Request Withdraw ===");
    println!("Chain:      {} ({})", chain_id, chain_display_name(chain_id));
    println!("Wallet:     {}", wallet);
    println!("Vault:      {}", vault_addr);
    println!("Shares:     {} ({} raw, {} decimals)", shares, raw_shares, decimals);
    println!(
        "Vault Type: {}",
        if btc_class { "BTC-class (pumpBTC) — requestRedeem(uint256)" }
        else { "ETH-class — requestRedeem(uint256,address)" }
    );
    println!();
    println!("IMPORTANT: This is a QUEUED withdrawal, not instant.");
    println!("  - Assets are released only after the rebalancer processes the request.");
    println!("  - Processing time: typically hours to a few days.");
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    // Build calldata based on vault type
    let redeem_data = if btc_class {
        // requestRedeem(uint256 _shares)
        encode_request_redeem_btc(raw_shares)
    } else {
        // requestRedeem(uint256 _shares, address _token)
        // Use provided token_addr or fall back to zero address
        let token = token_addr.unwrap_or(ZERO_ADDRESS);
        encode_request_redeem_eth(raw_shares, token)
    };

    println!("requestRedeem()");
    println!("  to:         {}", vault_addr);
    println!("  input-data: {}", redeem_data);

    let result = wallet_contract_call(chain_id, vault_addr, &redeem_data, dry_run).await?;

    if !is_ok(&result) && !dry_run {
        anyhow::bail!(
            "requestRedeem failed: {}",
            result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let tx = extract_tx_hash(&result);
    println!("  redeem request tx: {}", tx);
    println!();

    println!("=== Request Withdraw Summary ===");
    println!("Vault:           {}", vault_addr);
    println!("Shares submitted: {} ({} raw)", shares, raw_shares);
    println!("Tx hash:         {}", tx);
    println!();
    println!("Your withdrawal request has been submitted.");
    println!("Assets will be unlocked after the rebalancer processes the queue.");
    if dry_run {
        println!("[DRY-RUN] Transaction was simulated, not broadcast.");
    } else {
        println!("Check status at: https://yieldlayer.cian.app");
    }

    Ok(())
}
