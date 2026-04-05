use anyhow::Result;
use crate::abi::{encode_approve, encode_xpool_withdraw, solvbtc_to_raw};
use crate::config::*;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash, is_ok};

/// Unwrap xSolvBTC back to SolvBTC via XSolvBTCPool.withdraw().
///
/// ONLY available on Ethereum mainnet (chain 1).
/// Withdraw fee: 0.05% (5/10000).
///
/// Flow:
///   1. Approve xSolvBTC -> XSolvBTCPool
///   2. Wait 3s
///   3. XSolvBTCPool.withdraw(xSolvBtcAmount)
pub async fn run(amount: f64, dry_run: bool) -> Result<()> {
    let chain_id = CHAIN_ETHEREUM;
    let wallet = resolve_wallet(chain_id)?;

    let raw_amount = solvbtc_to_raw(amount);
    if raw_amount == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // Calculate estimated SolvBTC after withdraw fee
    let fee_rate = XSOLVBTC_WITHDRAW_FEE_RATE as f64 / XSOLVBTC_WITHDRAW_FEE_DENOM as f64;
    let nav = fetch_nav().await.unwrap_or(1.034);
    let solvbtc_before_fee = amount * nav;
    let solvbtc_after_fee = solvbtc_before_fee * (1.0 - fee_rate);
    let fee_amount = solvbtc_before_fee - solvbtc_after_fee;

    println!("=== xSolvBTC -> SolvBTC Unwrap ===");
    println!("Chain:              Ethereum (1) [xSolvBTC is Ethereum-only]");
    println!("Wallet:             {}", wallet);
    println!("xSolvBTC input:     {} xSolvBTC  ({} raw)", amount, raw_amount);
    println!("Current NAV:        {:.4} SolvBTC per xSolvBTC", nav);
    println!("SolvBTC before fee: {:.8} SolvBTC", solvbtc_before_fee);
    println!("Withdraw fee:       {:.8} SolvBTC (0.05%)", fee_amount);
    println!("Estimated output:   {:.8} SolvBTC", solvbtc_after_fee);
    println!("xSolvBTC addr:      {}", ETH_XSOLVBTC_TOKEN);
    println!("XSolvBTCPool:       {}", ETH_XSOLVBTC_POOL);
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    // Step 1: Approve xSolvBTC -> XSolvBTCPool
    let approve_data = encode_approve(ETH_XSOLVBTC_POOL, raw_amount);
    println!("Step 1: Approve xSolvBTC -> XSolvBTCPool");
    println!("  to:         {}", ETH_XSOLVBTC_TOKEN);
    println!("  input-data: {}", approve_data);

    let approve_result =
        wallet_contract_call(chain_id, ETH_XSOLVBTC_TOKEN, &approve_data, None, dry_run).await?;

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
        println!("Waiting 3 seconds before withdraw (nonce safety)...");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: XSolvBTCPool.withdraw(xSolvBtcAmount)
    let withdraw_data = encode_xpool_withdraw(raw_amount);
    println!("Step 2: XSolvBTCPool.withdraw()");
    println!("  to:         {}", ETH_XSOLVBTC_POOL);
    println!("  input-data: {}", withdraw_data);

    let withdraw_result =
        wallet_contract_call(chain_id, ETH_XSOLVBTC_POOL, &withdraw_data, None, dry_run).await?;

    if !is_ok(&withdraw_result) && !dry_run {
        anyhow::bail!(
            "XSolvBTCPool.withdraw failed: {}",
            withdraw_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let withdraw_tx = extract_tx_hash(&withdraw_result);
    println!("  withdraw tx: {}", withdraw_tx);
    println!();

    println!("=== Unwrap Summary ===");
    println!("xSolvBTC unwrapped: {} xSolvBTC", amount);
    println!("Estimated SolvBTC:  {:.8} SolvBTC (after 0.05% withdraw fee)", solvbtc_after_fee);
    println!("Approve tx:         {}", approve_tx);
    println!("Withdraw tx:        {}", withdraw_tx);
    if dry_run {
        println!("[DRY-RUN] Transactions were simulated, not broadcast.");
    }

    Ok(())
}

/// Fetch current xSolvBTC NAV (xSolvBTC price / SolvBTC price) from DeFiLlama.
async fn fetch_nav() -> anyhow::Result<f64> {
    let client = crate::onchainos::build_client()?;
    let url = format!(
        "https://coins.llama.fi/prices/current/{},{}",
        DEFI_LLAMA_SOLVBTC_ETH, DEFI_LLAMA_XSOLVBTC_ETH
    );
    let resp = client
        .get(&url)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let solvbtc_price = resp["coins"][DEFI_LLAMA_SOLVBTC_ETH]["price"]
        .as_f64()
        .unwrap_or(0.0);
    let xsolvbtc_price = resp["coins"][DEFI_LLAMA_XSOLVBTC_ETH]["price"]
        .as_f64()
        .unwrap_or(0.0);

    if solvbtc_price > 0.0 && xsolvbtc_price > 0.0 {
        Ok(xsolvbtc_price / solvbtc_price)
    } else {
        anyhow::bail!("Could not fetch prices from DeFiLlama")
    }
}
