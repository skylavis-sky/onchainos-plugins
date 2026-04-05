use anyhow::Result;
use crate::abi::{encode_approve, encode_xpool_deposit, solvbtc_to_raw};
use crate::config::*;
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash, is_ok};

/// Wrap SolvBTC into yield-bearing xSolvBTC via XSolvBTCPool.deposit().
///
/// ONLY available on Ethereum mainnet (chain 1).
///
/// Flow:
///   1. Approve SolvBTC -> XSolvBTCPool
///   2. Wait 3s
///   3. XSolvBTCPool.deposit(solvBtcAmount)
pub async fn run(amount: f64, dry_run: bool) -> Result<()> {
    let chain_id = CHAIN_ETHEREUM;
    let wallet = resolve_wallet(chain_id)?;

    let raw_amount = solvbtc_to_raw(amount);
    if raw_amount == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // Estimate xSolvBTC received based on current NAV (informational only)
    let nav = fetch_nav().await.unwrap_or(1.034);
    let estimated_xsolvbtc = amount / nav;

    println!("=== SolvBTC -> xSolvBTC Wrap ===");
    println!("Chain:             Ethereum (1) [xSolvBTC is Ethereum-only]");
    println!("Wallet:            {}", wallet);
    println!("SolvBTC input:     {} SolvBTC  ({} raw)", amount, raw_amount);
    println!("Current NAV:       {:.4} SolvBTC per xSolvBTC", nav);
    println!("Estimated output:  {:.8} xSolvBTC", estimated_xsolvbtc);
    println!("SolvBTC addr:      {}", ETH_SOLVBTC_TOKEN);
    println!("XSolvBTCPool:      {}", ETH_XSOLVBTC_POOL);
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    // Step 1: Approve SolvBTC -> XSolvBTCPool
    let approve_data = encode_approve(ETH_XSOLVBTC_POOL, raw_amount);
    println!("Step 1: Approve SolvBTC -> XSolvBTCPool");
    println!("  to:         {}", ETH_SOLVBTC_TOKEN);
    println!("  input-data: {}", approve_data);

    let approve_result =
        wallet_contract_call(chain_id, ETH_SOLVBTC_TOKEN, &approve_data, None, dry_run).await?;

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
        println!("Waiting 3 seconds before deposit (nonce safety)...");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: XSolvBTCPool.deposit(solvBtcAmount)
    let deposit_data = encode_xpool_deposit(raw_amount);
    println!("Step 2: XSolvBTCPool.deposit()");
    println!("  to:         {}", ETH_XSOLVBTC_POOL);
    println!("  input-data: {}", deposit_data);

    let deposit_result =
        wallet_contract_call(chain_id, ETH_XSOLVBTC_POOL, &deposit_data, None, dry_run).await?;

    if !is_ok(&deposit_result) && !dry_run {
        anyhow::bail!(
            "XSolvBTCPool.deposit failed: {}",
            deposit_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let deposit_tx = extract_tx_hash(&deposit_result);
    println!("  deposit tx: {}", deposit_tx);
    println!();

    println!("=== Wrap Summary ===");
    println!("SolvBTC deposited:  {} SolvBTC", amount);
    println!("Estimated xSolvBTC: {:.8} xSolvBTC (at NAV {:.4})", estimated_xsolvbtc, nav);
    println!("Approve tx:         {}", approve_tx);
    println!("Deposit tx:         {}", deposit_tx);
    println!();
    println!("xSolvBTC earns variable yield via Solv Protocol strategies.");
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
