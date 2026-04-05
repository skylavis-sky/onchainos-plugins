use anyhow::Context;
use tokio::time::{sleep, Duration};

use crate::api::{self, CreateTxParams};
use crate::commands::get_quote;
use crate::config::{
    self, is_solana, onchainos_to_debridge_chain, APPROVE_DELAY_SECS, DLN_SOURCE_EVM,
    DLN_SOURCE_SOLANA, NATIVE_EVM,
};
use crate::onchainos;

pub struct BridgeArgs {
    pub src_chain_id: u64,
    pub dst_chain_id: u64,
    pub src_token: String,
    pub dst_token: String,
    pub amount: String,
    pub recipient: Option<String>,
    pub dry_run: bool,
}

pub async fn run(args: BridgeArgs) -> anyhow::Result<()> {
    let src_is_solana = is_solana(args.src_chain_id);
    let dst_is_solana = is_solana(args.dst_chain_id);

    let src_db_id = onchainos_to_debridge_chain(args.src_chain_id);
    let dst_db_id = onchainos_to_debridge_chain(args.dst_chain_id);

    // -------------------------------------------------------------------
    // Step 1: Resolve wallet addresses
    // -------------------------------------------------------------------
    let src_wallet = if src_is_solana {
        onchainos::resolve_wallet_solana()
            .context("Failed to resolve Solana source wallet")?
    } else {
        let addr = onchainos::resolve_wallet_evm(args.src_chain_id)
            .context("Failed to resolve EVM source wallet")?;
        if addr.is_empty() {
            anyhow::bail!(
                "Could not determine wallet address for chain {}. Is onchainos configured?",
                args.src_chain_id
            );
        }
        addr
    };

    // Destination wallet (for authority + recipient)
    let dst_wallet = if let Some(ref rec) = args.recipient {
        rec.clone()
    } else if dst_is_solana {
        onchainos::resolve_wallet_solana()
            .context("Failed to resolve Solana destination wallet")?
    } else {
        let addr = onchainos::resolve_wallet_evm(args.dst_chain_id)
            .context("Failed to resolve EVM destination wallet")?;
        if addr.is_empty() {
            // Fall back to source wallet (same user, different chain)
            src_wallet.clone()
        } else {
            addr
        }
    };

    println!("Source wallet:      {}", src_wallet);
    println!("Destination wallet: {}", dst_wallet);
    println!("src chain:          {} (deBridge ID: {})", args.src_chain_id, src_db_id);
    println!("dst chain:          {} (deBridge ID: {})", args.dst_chain_id, dst_db_id);

    // -------------------------------------------------------------------
    // Step 2: Fetch quote (estimation only — no tx data yet)
    // -------------------------------------------------------------------
    println!("\nFetching quote...");
    let quote_params = CreateTxParams {
        src_chain_id: &src_db_id,
        src_token: &args.src_token,
        src_amount: &args.amount,
        dst_chain_id: &dst_db_id,
        dst_token: &args.dst_token,
        src_authority: None,
        dst_authority: None,
        dst_recipient: None,
        skip_solana_recipient_validation: false,
    };
    let quote_resp = api::create_tx(&quote_params)
        .await
        .context("Failed to fetch deBridge quote")?;
    get_quote::print_quote(&quote_resp);

    // -------------------------------------------------------------------
    // Step 3: Build full tx (with authority + recipient addresses)
    // Tx expires ~30s — get quote first, then build+submit immediately
    // -------------------------------------------------------------------
    println!("\nBuilding transaction...");
    let skip_sol_validation = dst_is_solana;
    let tx_params = CreateTxParams {
        src_chain_id: &src_db_id,
        src_token: &args.src_token,
        src_amount: &args.amount,
        dst_chain_id: &dst_db_id,
        dst_token: &args.dst_token,
        src_authority: Some(&src_wallet),
        dst_authority: Some(&dst_wallet),
        dst_recipient: Some(&dst_wallet),
        skip_solana_recipient_validation: skip_sol_validation,
    };
    let tx_resp = api::create_tx(&tx_params)
        .await
        .context("Failed to build deBridge transaction")?;

    let order_id = tx_resp["orderId"].as_str().unwrap_or("").to_string();
    println!("Order ID: {}", order_id);

    // -------------------------------------------------------------------
    // EVM source chain flow
    // -------------------------------------------------------------------
    if !src_is_solana {
        run_evm_bridge(&args, &src_wallet, &tx_resp, order_id).await?;
    } else {
        // -------------------------------------------------------------------
        // Solana source chain flow
        // -------------------------------------------------------------------
        run_solana_bridge(&args, &tx_resp, order_id).await?;
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// EVM source bridge
// ---------------------------------------------------------------------------

async fn run_evm_bridge(
    args: &BridgeArgs,
    src_wallet: &str,
    tx_resp: &serde_json::Value,
    order_id: String,
) -> anyhow::Result<()> {
    let tx = &tx_resp["tx"];
    let to = tx["to"].as_str().unwrap_or(DLN_SOURCE_EVM);
    let data = tx["data"].as_str().context("Missing tx.data in API response")?;
    let value_str = tx["value"].as_str().unwrap_or("0");
    let tx_value: u128 = value_str.parse().unwrap_or(0);

    println!("\nTransaction details:");
    println!("  to:    {}", to);
    println!("  value: {} wei", tx_value);
    println!("  data:  {}...{}", &data[..data.len().min(20)], &data[data.len().saturating_sub(8)..]);

    let is_native = args.src_token.to_lowercase() == NATIVE_EVM.to_lowercase();

    // -------------------------------------------------------------------
    // Step 4a: ERC-20 approve if needed
    // -------------------------------------------------------------------
    if !is_native {
        let rpc = config::rpc_url(args.src_chain_id);
        let allowance = api::get_erc20_allowance(rpc, &args.src_token, src_wallet, to)
            .await
            .unwrap_or(0);
        let amount_needed: u128 = args.amount.parse().unwrap_or(u128::MAX);

        println!("\nCurrent allowance: {}", allowance);
        if allowance >= amount_needed {
            println!("Allowance sufficient — skipping approve.");
        } else {
            println!("Approving {} (spender={})...", args.src_token, to);
            // Use max uint256 for approval: ffffffff...ffffffff
            let max_hex = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
            let approve_calldata = onchainos::encode_approve(to, max_hex);
            println!("Approve calldata: {}", approve_calldata);

            let approve_result = onchainos::wallet_contract_call_evm(
                args.src_chain_id,
                &args.src_token,
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
                        "Approve failed: {}",
                        approve_result["error"].as_str().unwrap_or("unknown error")
                    );
                }
                let approve_tx = onchainos::extract_tx_hash(&approve_result);
                println!("Approve tx: {}", approve_tx);
                println!("Waiting {}s for approval to confirm...", APPROVE_DELAY_SECS);
                sleep(Duration::from_secs(APPROVE_DELAY_SECS)).await;
            }
        }
    } else {
        println!("\nNative token — skipping ERC-20 approve.");
    }

    // -------------------------------------------------------------------
    // Step 4b: Submit createOrder via API-provided calldata
    // -------------------------------------------------------------------
    println!("\nSubmitting createOrder to DlnSource...");
    println!("  to:     {}", to);
    println!("  value:  {} wei", tx_value);

    let order_result = onchainos::wallet_contract_call_evm(
        args.src_chain_id,
        to,
        data,
        if tx_value > 0 { Some(tx_value) } else { None },
        args.dry_run,
    )
    .await
    .context("Failed to submit createOrder transaction")?;

    if args.dry_run {
        println!("\n=== DRY RUN COMPLETE ===");
        println!("No on-chain transactions were submitted.");
        println!("Order ID: {}", order_id);
        println!("Simulated txHash: {}", onchainos::extract_tx_hash(&order_result));
        return Ok(());
    }

    let ok = order_result["ok"].as_bool().unwrap_or(false);
    if !ok {
        anyhow::bail!(
            "createOrder transaction failed: {}",
            order_result["error"].as_str().unwrap_or("unknown error")
        );
    }

    let tx_hash = onchainos::extract_tx_hash(&order_result);
    println!("\nOrder submitted!");
    println!("  txHash:  {}", tx_hash);
    println!("  orderId: {}", order_id);
    if !order_id.is_empty() {
        println!("\nCheck status with:");
        println!("  debridge get-status --order-id {}", order_id);
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Solana source bridge
// ---------------------------------------------------------------------------

async fn run_solana_bridge(
    args: &BridgeArgs,
    tx_resp: &serde_json::Value,
    order_id: String,
) -> anyhow::Result<()> {
    let tx = &tx_resp["tx"];
    let hex_data = tx["data"].as_str().context("Missing tx.data in API response for Solana")?;

    println!("\nSolana VersionedTransaction (hex len={})", hex_data.len());

    // Convert hex → bytes → base58 for onchainos --unsigned-tx
    let tx_base58 = onchainos::hex_to_base58(hex_data)
        .context("Failed to convert Solana tx from hex to base58")?;

    println!("base58 length: {}", tx_base58.len());
    println!("Submitting Solana transaction to DlnSource program...");

    let solana_result = onchainos::wallet_contract_call_solana(
        DLN_SOURCE_SOLANA,
        &tx_base58,
        args.dry_run,
    )
    .await
    .context("Failed to submit Solana createOrder transaction")?;

    if args.dry_run {
        println!("\n=== DRY RUN COMPLETE ===");
        println!("No on-chain transactions were submitted.");
        println!("Order ID: {}", order_id);
        return Ok(());
    }

    let ok = solana_result["ok"].as_bool().unwrap_or(false);
    if !ok {
        anyhow::bail!(
            "Solana createOrder transaction failed: {}",
            solana_result["error"].as_str().unwrap_or("unknown error")
        );
    }

    let tx_hash = onchainos::extract_tx_hash(&solana_result);
    println!("\nOrder submitted!");
    println!("  txHash:  {}", tx_hash);
    println!("  orderId: {}", order_id);
    if !order_id.is_empty() {
        println!("\nCheck status with:");
        println!("  debridge get-status --order-id {}", order_id);
    }

    Ok(())
}
