use anyhow::Context;
use tokio::time::{sleep, Duration};

use crate::abi;
use crate::api;
use crate::config::{
    self, APPROVE_DELAY_SECS, ETH_ADDRESS, STATUS_MAX_RETRIES, STATUS_POLL_INTERVAL_SECS,
};
use crate::onchainos;

pub struct BridgeArgs {
    pub input_token: String,
    pub output_token: String,
    pub origin_chain_id: u64,
    pub destination_chain_id: u64,
    pub amount: String,
    pub recipient: Option<String>,
    pub dry_run: bool,
}

pub async fn run(args: BridgeArgs) -> anyhow::Result<()> {
    // Step 1: Resolve wallet address
    let wallet = onchainos::resolve_wallet(args.origin_chain_id)
        .context("Failed to resolve wallet address")?;
    if wallet.is_empty() {
        anyhow::bail!("Could not determine wallet address for chain {}. Is onchainos configured?", args.origin_chain_id);
    }
    let recipient = args.recipient.as_deref().unwrap_or(&wallet).to_string();

    println!("Wallet:    {}", wallet);
    println!("Recipient: {}", recipient);

    // Step 2: Get quote from suggested-fees
    println!("\nFetching quote...");
    let quote = api::get_suggested_fees(
        &args.input_token,
        &args.output_token,
        args.origin_chain_id,
        args.destination_chain_id,
        &args.amount,
        Some(&wallet),
        Some(&recipient),
    )
    .await
    .context("Failed to fetch suggested-fees quote")?;

    // Step 3: Check isAmountTooLow
    if quote["isAmountTooLow"].as_bool().unwrap_or(false) {
        let min = quote["limits"]["minDeposit"].as_str().unwrap_or("unknown");
        anyhow::bail!(
            "Amount too low to bridge. Minimum deposit is {} (in token base units). \
             Please increase your transfer amount.",
            min
        );
    }

    let output_amount = quote["outputAmount"]
        .as_str()
        .context("Missing outputAmount in quote")?
        .to_string();
    let timestamp_str = quote["timestamp"]
        .as_str()
        .context("Missing timestamp in quote")?;
    let fill_deadline_str = quote["fillDeadline"]
        .as_str()
        .context("Missing fillDeadline in quote")?;
    let exclusive_relayer = quote["exclusiveRelayer"]
        .as_str()
        .unwrap_or("0x0000000000000000000000000000000000000000");
    let exclusivity_deadline = quote["exclusivityDeadline"]
        .as_u64()
        .unwrap_or(0) as u32;
    let fill_time = quote["estimatedFillTimeSec"].as_u64().unwrap_or(0);

    // Resolve SpokePool address: prefer API response, fall back to config
    let spoke_pool = quote["spokePoolAddress"]
        .as_str()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| config::get_spoke_pool(args.origin_chain_id));
    if spoke_pool.is_empty() {
        anyhow::bail!(
            "Unsupported origin chain {}. Supported chains: 1, 10, 137, 8453, 42161",
            args.origin_chain_id
        );
    }

    let quote_timestamp: u32 = timestamp_str
        .parse()
        .with_context(|| format!("Invalid timestamp: {}", timestamp_str))?;
    let fill_deadline: u32 = fill_deadline_str
        .parse()
        .with_context(|| format!("Invalid fillDeadline: {}", fill_deadline_str))?;

    println!("\n=== Bridge Quote ===");
    println!("Input amount:      {} (base units)", args.amount);
    println!("Output amount:     {} (base units, after fees)", output_amount);
    println!("Total relay fee:   {}", quote["totalRelayFee"]["total"].as_str().unwrap_or("N/A"));
    println!("Est. fill time:    {} seconds", fill_time);
    println!("SpokePool:         {}", spoke_pool);
    println!("Quote timestamp:   {}", quote_timestamp);
    println!("Fill deadline:     {}", fill_deadline);

    let is_native_eth = args.input_token.to_lowercase() == ETH_ADDRESS.to_lowercase();

    // Step 4: Approve ERC-20 if needed (skip for ETH)
    if !is_native_eth {
        println!("\nApproving {} for SpokePool {}...", args.input_token, spoke_pool);
        let approve_calldata = abi::encode_approve(spoke_pool, u128::MAX);
        println!("Approve calldata: {}", approve_calldata);

        let approve_result = onchainos::wallet_contract_call(
            args.origin_chain_id,
            &args.input_token,
            &approve_calldata,
            None,
            args.dry_run,
        )
        .await
        .context("Failed to submit approve transaction")?;

        if args.dry_run {
            println!("[DRY RUN] Approve skipped.");
        } else {
            let ok = approve_result["ok"].as_bool().unwrap_or(false);
            if !ok {
                anyhow::bail!(
                    "Approve transaction failed: {}",
                    approve_result["error"].as_str().unwrap_or("unknown error")
                );
            }
            let approve_tx = onchainos::extract_tx_hash(&approve_result);
            println!("Approve tx: {}", approve_tx);
            println!("Waiting {}s for approval to confirm...", APPROVE_DELAY_SECS);
            sleep(Duration::from_secs(APPROVE_DELAY_SECS)).await;
        }
    } else {
        println!("\nNative ETH bridge — skipping ERC-20 approve.");
    }

    // Step 5: Encode and submit depositV3
    let deposit_calldata = abi::encode_deposit_v3(
        &wallet,
        &recipient,
        &args.input_token,
        &args.output_token,
        &args.amount,
        &output_amount,
        args.destination_chain_id,
        exclusive_relayer,
        quote_timestamp,
        fill_deadline,
        exclusivity_deadline,
    );

    println!("\nSubmitting depositV3...");
    println!("To (SpokePool): {}", spoke_pool);
    println!("Calldata: {}", deposit_calldata);

    let eth_value = if is_native_eth {
        let v: u64 = args
            .amount
            .parse()
            .with_context(|| format!("Invalid amount for ETH bridge: {}", args.amount))?;
        Some(v)
    } else {
        None
    };

    let deposit_result = onchainos::wallet_contract_call(
        args.origin_chain_id,
        spoke_pool,
        &deposit_calldata,
        eth_value,
        args.dry_run,
    )
    .await
    .context("Failed to submit depositV3 transaction")?;

    if args.dry_run {
        println!("\n=== DRY RUN COMPLETE ===");
        println!("No on-chain transactions were submitted.");
        println!("Bridge calldata: {}", deposit_calldata);
        println!("ETH value (wei): {:?}", eth_value);
        println!("Simulated txHash: {}", onchainos::extract_tx_hash(&deposit_result));
        return Ok(());
    }

    let ok = deposit_result["ok"].as_bool().unwrap_or(false);
    if !ok {
        anyhow::bail!(
            "depositV3 transaction failed: {}",
            deposit_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let deposit_tx_hash = onchainos::extract_tx_hash(&deposit_result);
    println!("\nDeposit submitted! tx: {}", deposit_tx_hash);
    println!("Estimated fill time: {} seconds", fill_time);

    // Step 6: Poll status (up to STATUS_MAX_RETRIES * STATUS_POLL_INTERVAL_SECS seconds)
    println!("\nPolling bridge status...");
    for attempt in 1..=STATUS_MAX_RETRIES {
        sleep(Duration::from_secs(STATUS_POLL_INTERVAL_SECS)).await;
        println!("Check {}/{}...", attempt, STATUS_MAX_RETRIES);

        match api::get_deposit_status(Some(&deposit_tx_hash), None, Some(args.origin_chain_id), None).await {
            Ok(status) => {
                let fill_status = status["status"].as_str().unwrap_or("unknown");
                match fill_status {
                    "filled" => {
                        let fill_tx = status["fillTxnHash"].as_str().unwrap_or("N/A");
                        println!("\nBridge COMPLETE!");
                        println!("Source tx:      {}", deposit_tx_hash);
                        println!("Destination tx: {}", fill_tx);
                        println!("Chain {} -> {} complete.", args.origin_chain_id, args.destination_chain_id);
                        return Ok(());
                    }
                    "expired" => {
                        let refund_tx = status["depositRefundTxnHash"].as_str().unwrap_or("N/A");
                        println!("\nDeposit EXPIRED. Refund tx: {}", refund_tx);
                        return Ok(());
                    }
                    _ => {
                        println!("  Status: {} — waiting...", fill_status);
                    }
                }
            }
            Err(e) => {
                println!("  Status check error: {} — will retry", e);
            }
        }
    }

    // Timed out waiting for fill
    println!("\nTimed out waiting for fill ({}s).", STATUS_MAX_RETRIES as u64 * STATUS_POLL_INTERVAL_SECS);
    println!("Deposit tx: {}", deposit_tx_hash);
    println!("Check status later with:");
    println!("  across get-status --tx-hash {} --origin-chain-id {}", deposit_tx_hash, args.origin_chain_id);

    Ok(())
}
