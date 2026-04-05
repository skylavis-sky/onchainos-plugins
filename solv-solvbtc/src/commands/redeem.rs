use anyhow::Result;
use crate::abi::{encode_approve, encode_withdraw_request, encode_cancel_withdraw_request, solvbtc_to_raw};
use crate::config::*;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash, is_ok};

/// Redeem SolvBTC back to WBTC via RouterV2.withdrawRequest().
///
/// WARNING: This is NOT instant. It creates an ERC-3525 SFT redemption ticket.
/// The WBTC is released only after the OpenFundMarket queue processes the request.
///
/// Flow:
///   1. Approve SolvBTC -> RouterV2
///   2. Wait 3s
///   3. RouterV2.withdrawRequest(solvbtc, wbtc, amount)
pub async fn run(amount: f64, chain_id: u64, dry_run: bool) -> Result<()> {
    let (solvbtc_addr, wbtc_addr, router_addr) = chain_contracts(chain_id)?;
    let wallet = resolve_wallet(chain_id)?;

    let raw_amount = solvbtc_to_raw(amount);
    if raw_amount == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    println!("=== SolvBTC Redeem (Withdraw Request) ===");
    println!();
    println!("WARNING: Redemption is NOT instant!");
    println!("  - This creates an ERC-3525 SFT redemption ticket.");
    println!("  - WBTC will be released only after OpenFundMarket queue processing.");
    println!("  - You can cancel this request with: solv-solvbtc cancel-redeem");
    println!();
    println!("Chain:        {} ({})", chain_id, chain_name(chain_id));
    println!("Wallet:       {}", wallet);
    println!("Redeem:       {} SolvBTC  ({} raw, 18 decimals)", amount, raw_amount);
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    // Step 1: Approve SolvBTC -> RouterV2
    let approve_data = encode_approve(router_addr, raw_amount);
    println!("Step 1: Approve SolvBTC -> RouterV2");
    println!("  to:         {}", solvbtc_addr);
    println!("  input-data: {}", approve_data);

    let approve_result =
        wallet_contract_call(chain_id, solvbtc_addr, &approve_data, None, dry_run).await?;

    if !is_ok(&approve_result) && !dry_run {
        anyhow::bail!(
            "Approve failed: {}",
            approve_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let approve_tx = extract_tx_hash(&approve_result);
    println!("  approve tx: {}", approve_tx);
    println!();

    // Wait 3 seconds (nonce safety)
    if !dry_run {
        println!("Waiting 3 seconds before withdraw request (nonce safety)...");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: RouterV2.withdrawRequest(targetToken, currency, withdrawAmount)
    let withdraw_data = encode_withdraw_request(solvbtc_addr, wbtc_addr, raw_amount);
    println!("Step 2: RouterV2.withdrawRequest()");
    println!("  to:         {}", router_addr);
    println!("  input-data: {}", withdraw_data);

    let withdraw_result =
        wallet_contract_call(chain_id, router_addr, &withdraw_data, None, dry_run).await?;

    if !is_ok(&withdraw_result) && !dry_run {
        anyhow::bail!(
            "WithdrawRequest failed: {}",
            withdraw_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let withdraw_tx = extract_tx_hash(&withdraw_result);
    println!("  withdraw request tx: {}", withdraw_tx);
    println!();

    println!("=== Redeem Summary ===");
    println!("SolvBTC submitted:  {} SolvBTC", amount);
    println!("Expected WBTC:      ~{:.8} WBTC (after queue processing)", amount);
    println!("Approve tx:         {}", approve_tx);
    println!("Withdraw req tx:    {}", withdraw_tx);
    println!();
    println!("IMPORTANT: Check your wallet for the ERC-3525 SFT redemption ticket.");
    println!("           Run 'solv-solvbtc cancel-redeem' to cancel if needed.");
    if dry_run {
        println!("[DRY-RUN] Transactions were simulated, not broadcast.");
    }

    Ok(())
}

/// Cancel a pending redemption request.
pub async fn cancel(
    redemption_addr: &str,
    redemption_id: u128,
    chain_id: u64,
    dry_run: bool,
) -> Result<()> {
    let (solvbtc_addr, _wbtc_addr, router_addr) = chain_contracts(chain_id)?;
    let wallet = resolve_wallet(chain_id)?;

    println!("=== Cancel SolvBTC Redeem ===");
    println!("Chain:          {} ({})", chain_id, chain_name(chain_id));
    println!("Wallet:         {}", wallet);
    println!("Redemption:     {}", redemption_addr);
    println!("Redemption ID:  {}", redemption_id);
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    let cancel_data =
        encode_cancel_withdraw_request(solvbtc_addr, redemption_addr, redemption_id);
    println!("RouterV2.cancelWithdrawRequest()");
    println!("  to:         {}", router_addr);
    println!("  input-data: {}", cancel_data);

    let result =
        wallet_contract_call(chain_id, router_addr, &cancel_data, None, dry_run).await?;

    if !is_ok(&result) && !dry_run {
        anyhow::bail!(
            "CancelWithdrawRequest failed: {}",
            result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let tx = extract_tx_hash(&result);
    println!("  cancel tx: {}", tx);
    println!();
    println!("Redemption cancelled. SolvBTC will be returned to your wallet.");
    if dry_run {
        println!("[DRY-RUN] Transaction was simulated, not broadcast.");
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
