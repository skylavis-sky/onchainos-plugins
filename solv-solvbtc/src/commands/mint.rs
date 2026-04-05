use anyhow::Result;
use crate::abi::{encode_approve, encode_router_deposit, wbtc_to_raw};
use crate::config::*;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash, is_ok};

/// Mint SolvBTC by depositing WBTC via SolvBTCRouterV2.
///
/// Flow:
///   1. Approve WBTC -> RouterV2
///   2. Wait 3s (nonce safety)
///   3. RouterV2.deposit(solvbtc, wbtc, amount, 0, now+300)
pub async fn run(amount: f64, chain_id: u64, dry_run: bool) -> Result<()> {
    let (solvbtc_addr, wbtc_addr, router_addr) = chain_contracts(chain_id)?;
    let wallet = resolve_wallet(chain_id)?;

    let raw_amount = wbtc_to_raw(amount);
    if raw_amount == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let expire_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs()
        + 300;

    println!("=== SolvBTC Mint ===");
    println!("Chain:        {} ({})", chain_id, chain_name(chain_id));
    println!("Wallet:       {}", wallet);
    println!("Input:        {} WBTC  ({} raw, 8 decimals)", amount, raw_amount);
    println!("SolvBTC addr: {}", solvbtc_addr);
    println!("WBTC addr:    {}", wbtc_addr);
    println!("RouterV2:     {}", router_addr);
    println!("Expire time:  {} (now + 300s)", expire_time);
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    // Step 1: Approve WBTC -> RouterV2
    let approve_data = encode_approve(router_addr, raw_amount);
    println!("Step 1: Approve WBTC -> RouterV2");
    println!("  to:         {}", wbtc_addr);
    println!("  input-data: {}", approve_data);

    let approve_result =
        wallet_contract_call(chain_id, wbtc_addr, &approve_data, None, dry_run).await?;

    if !is_ok(&approve_result) && !dry_run {
        anyhow::bail!(
            "Approve failed: {}",
            approve_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let approve_tx = extract_tx_hash(&approve_result);
    println!("  approve tx: {}", approve_tx);
    println!();

    // Wait 3 seconds between approve and deposit (nonce safety)
    if !dry_run {
        println!("Waiting 3 seconds before deposit (nonce safety)...");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: RouterV2.deposit(targetToken, currency, currencyAmount, minTargetAmount, expireTime)
    let deposit_data = encode_router_deposit(
        solvbtc_addr,
        wbtc_addr,
        raw_amount,
        0,          // minimumTargetTokenAmount = 0 (no slippage protection)
        expire_time,
    );
    println!("Step 2: RouterV2.deposit()");
    println!("  to:         {}", router_addr);
    println!("  input-data: {}", deposit_data);

    let deposit_result =
        wallet_contract_call(chain_id, router_addr, &deposit_data, None, dry_run).await?;

    if !is_ok(&deposit_result) && !dry_run {
        anyhow::bail!(
            "Deposit failed: {}",
            deposit_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let deposit_tx = extract_tx_hash(&deposit_result);
    println!("  deposit tx: {}", deposit_tx);
    println!();

    println!("=== Mint Summary ===");
    println!("WBTC deposited:     {} WBTC", amount);
    println!("Estimated SolvBTC:  ~{:.8} SolvBTC (1:1 ratio, minus any fees)", amount);
    println!("Approve tx:         {}", approve_tx);
    println!("Deposit tx:         {}", deposit_tx);
    if dry_run {
        println!("[DRY-RUN] Transactions were simulated, not broadcast.");
    } else {
        println!("SolvBTC will appear in your wallet after on-chain confirmation.");
    }

    Ok(())
}

fn chain_name(chain_id: u64) -> &'static str {
    match chain_id {
        CHAIN_ARBITRUM => "Arbitrum",
        CHAIN_ETHEREUM => "Ethereum",
        _ => "Unknown",
    }
}
