use anyhow::{anyhow, Context};
use reqwest::Client;
use std::time::Duration;

use crate::api::{get_quote, get_swap_evm, get_swap_solana, pick_best_route, route_type_to_deposit_mode};
use crate::commands::get_quote::infer_decimals;
use crate::config::{
    chain_id_to_mayan_name, is_evm_chain, is_native_token, APPROVE_FORWARDER_CALLDATA,
    MAYAN_FORWARDER_CONTRACT, SOLANA_CHAIN_ID,
};
use crate::onchainos::{
    base64_to_base58, extract_tx_hash_or_err, resolve_wallet_evm, resolve_wallet_solana,
    wallet_contract_call_evm, wallet_contract_call_solana,
};

pub struct SwapArgs {
    pub from_chain: u64,
    pub to_chain: u64,
    pub from_token: String,
    pub to_token: String,
    pub amount: f64,
    pub slippage: Option<u32>,
    pub dry_run: bool,
}

pub async fn run(args: SwapArgs) -> anyhow::Result<()> {
    let client = Client::new();

    let from_chain_name = chain_id_to_mayan_name(args.from_chain)
        .ok_or_else(|| anyhow!("Unsupported from-chain: {}", args.from_chain))?;
    let to_chain_name = chain_id_to_mayan_name(args.to_chain)
        .ok_or_else(|| anyhow!("Unsupported to-chain: {}", args.to_chain))?;

    let slippage_bps = args.slippage.unwrap_or(100u32);

    // Convert amount to base units
    let decimals = infer_decimals(&args.from_token, from_chain_name);
    let multiplier = 10u64.pow(decimals) as f64;
    let amount_in64_u64 = (args.amount * multiplier).round() as u64;
    let amount_in64 = amount_in64_u64.to_string();

    println!("Mayan Cross-Chain Swap");
    println!("{:-<60}", "");
    println!("  From: {} on {} (chain {})", args.from_token, from_chain_name, args.from_chain);
    println!("  To:   {} on {} (chain {})", args.to_token, to_chain_name, args.to_chain);
    println!("  Amount: {} (base units: {})", args.amount, amount_in64);
    println!("  Slippage: {} bps", slippage_bps);
    if args.dry_run {
        println!("  [DRY RUN - no transactions will be broadcast]");
    }
    println!();

    // -----------------------------------------------------------------------
    // Step 1: Resolve wallet addresses
    // -----------------------------------------------------------------------
    println!("[1/4] Resolving wallet addresses...");

    let from_wallet: String;
    let to_wallet: String;

    if is_evm_chain(args.from_chain) {
        from_wallet = resolve_wallet_evm(args.from_chain)
            .context("Failed to resolve EVM wallet address")?;
    } else {
        from_wallet = resolve_wallet_solana()
            .context("Failed to resolve Solana wallet address")?;
    }

    if is_evm_chain(args.to_chain) {
        to_wallet = resolve_wallet_evm(args.to_chain)
            .context("Failed to resolve EVM destination wallet address")?;
    } else {
        to_wallet = resolve_wallet_solana()
            .context("Failed to resolve Solana destination wallet address")?;
    }

    println!("  From wallet: {}", from_wallet);
    println!("  To wallet:   {}", to_wallet);
    println!();

    // -----------------------------------------------------------------------
    // Step 2: Get quote and pick best route
    // -----------------------------------------------------------------------
    println!("[2/4] Fetching quote...");

    let quotes = get_quote(
        &client,
        &amount_in64,
        &args.from_token,
        from_chain_name,
        &args.to_token,
        to_chain_name,
        slippage_bps,
        Some(&to_wallet),
        false,
    )
    .await
    .context("Failed to fetch quote")?;

    if quotes.is_empty() {
        return Err(anyhow!("No routes available for this swap pair."));
    }

    let best_route = pick_best_route(&quotes)
        .ok_or_else(|| anyhow!("No valid route found"))?;

    let route_type = best_route["type"].as_str().unwrap_or("SWIFT");
    let expected_out = best_route["expectedAmountOut"].as_str().unwrap_or("?");
    let min_received = best_route["minReceived"]
        .as_str()
        .or_else(|| best_route["minAmountOut"].as_str())
        .unwrap_or("?");
    let eta = best_route["etaSeconds"].as_u64().unwrap_or(0);
    let to_symbol = best_route["toToken"]["symbol"].as_str().unwrap_or("?");
    let middle_token = best_route["middleToken"]
        .as_str()
        .or_else(|| best_route["middleToken"]["contract"].as_str())
        .unwrap_or("")
        .to_string();
    let min_middle_amount = best_route["minMiddleAmount"].as_f64();

    println!("  Route: {}", route_type);
    println!("  Expected output: {} {}", expected_out, to_symbol);
    println!("  Min received:    {} {}", min_received, to_symbol);
    println!("  ETA: ~{}s", eta);
    println!();

    // -----------------------------------------------------------------------
    // Step 3: Build and submit transaction
    // -----------------------------------------------------------------------
    println!("[3/4] Building and submitting swap transaction...");

    let tx_hash: String;

    if args.from_chain == SOLANA_CHAIN_ID {
        tx_hash = swap_from_solana(
            &client,
            &amount_in64,
            &args.from_token,
            &args.to_token,
            &from_wallet,
            &to_wallet,
            slippage_bps,
            to_chain_name,
            route_type,
            &middle_token,
            min_middle_amount,
            args.dry_run,
        )
        .await?;
    } else {
        tx_hash = swap_from_evm(
            &client,
            args.from_chain,
            &amount_in64,
            amount_in64_u64,
            &args.from_token,
            &args.to_token,
            from_chain_name,
            to_chain_name,
            &from_wallet,
            &to_wallet,
            slippage_bps,
            &middle_token,
            args.dry_run,
        )
        .await?;
    }

    // -----------------------------------------------------------------------
    // Step 4: Print result and status hint
    // -----------------------------------------------------------------------
    println!("[4/4] Transaction submitted!");
    println!();
    println!("  Source Tx Hash: {}", tx_hash);
    println!();
    println!("Check swap status with:");
    println!("  mayan get-status --tx-hash {}", tx_hash);
    println!();
    println!(
        "Or track on Mayan Explorer: https://explorer.mayan.finance/swap/{}",
        tx_hash
    );

    Ok(())
}

/// Execute Solana-sourced swap
async fn swap_from_solana(
    client: &Client,
    amount_in64: &str,
    from_token: &str,
    to_token: &str,
    user_wallet: &str,
    _destination_address: &str,
    slippage_bps: u32,
    to_chain_name: &str,
    route_type: &str,
    middle_token: &str,
    min_middle_amount: Option<f64>,
    dry_run: bool,
) -> anyhow::Result<String> {
    let deposit_mode = route_type_to_deposit_mode(route_type);

    println!("  Building Solana swap tx (route: {}, mode: {})...", route_type, deposit_mode);

    let swap_resp = get_swap_solana(
        client,
        amount_in64,
        from_token,
        user_wallet,
        slippage_bps,
        to_chain_name,
        deposit_mode,
        if middle_token.is_empty() { None } else { Some(middle_token) },
        min_middle_amount,
        to_token,
        None,
    )
    .await
    .context("Failed to build Solana swap transaction")?;

    // The API may return a serialized transaction (base64) in different fields.
    // Common patterns: response["transaction"], response["serializedTx"], or a full tx object.
    let tx_base64 = extract_solana_tx_base64(&swap_resp)?;

    println!("  Converting base64 tx to base58...");
    let tx_base58 = base64_to_base58(&tx_base64)
        .context("Failed to convert transaction from base64 to base58")?;

    // Determine program ID based on route type
    let program_id = route_type_to_solana_program(route_type);
    println!("  Broadcasting to program: {}", program_id);

    let result = wallet_contract_call_solana(program_id, &tx_base58, dry_run).await?;

    if !result["ok"].as_bool().unwrap_or(false) && !dry_run {
        let err = result["error"].as_str().unwrap_or("unknown error");
        return Err(anyhow!("Solana contract call failed: {}", err));
    }

    Ok(extract_tx_hash_or_err(&result)?)
}

/// Execute EVM-sourced swap (handles both native ETH and ERC-20)
async fn swap_from_evm(
    client: &Client,
    from_chain_id: u64,
    amount_in64: &str,
    amount_in64_u64: u64,
    from_token: &str,
    to_token: &str,
    from_chain_name: &str,
    to_chain_name: &str,
    _from_wallet: &str,
    destination_address: &str,
    slippage_bps: u32,
    middle_token: &str,
    dry_run: bool,
) -> anyhow::Result<String> {
    println!(
        "  Building EVM swap calldata (from: {}, to: {})...",
        from_chain_name, to_chain_name
    );

    let swap_resp = get_swap_evm(
        client,
        amount_in64,
        from_token,
        from_chain_name,
        to_token,
        to_chain_name,
        slippage_bps,
        destination_address,
        if middle_token.is_empty() { None } else { Some(middle_token) },
        None,
    )
    .await
    .context("Failed to build EVM swap calldata")?;

    // Extract calldata and target address
    // API may return swapRouterCalldata / swapRouterAddress or tx.data / tx.to
    let calldata = swap_resp["swapRouterCalldata"]
        .as_str()
        .or_else(|| swap_resp["tx"]["data"].as_str())
        .or_else(|| swap_resp["data"].as_str())
        .ok_or_else(|| anyhow!("No calldata found in EVM swap response: {}", swap_resp))?
        .to_string();

    let to_address = swap_resp["swapRouterAddress"]
        .as_str()
        .or_else(|| swap_resp["tx"]["to"].as_str())
        .or_else(|| swap_resp["to"].as_str())
        .unwrap_or(MAYAN_FORWARDER_CONTRACT)
        .to_string();

    // ETH value to send (from tx.value if present, else amount for native ETH)
    let eth_value_str = swap_resp["tx"]["value"].as_str();
    let eth_value: Option<u64> = if is_native_token(from_token) {
        // For native ETH use the amount from the API response or fall back to input amount
        if let Some(v) = eth_value_str {
            v.parse::<u64>().ok().or(Some(amount_in64_u64))
        } else {
            Some(amount_in64_u64)
        }
    } else {
        None
    };

    // ERC-20: check if approve is needed then wait
    if !is_native_token(from_token) {
        println!("  ERC-20 token detected — submitting approve to Mayan Forwarder...");
        let approve_result = wallet_contract_call_evm(
            from_chain_id,
            from_token,
            APPROVE_FORWARDER_CALLDATA,
            None,
            dry_run,
        )
        .await
        .context("ERC-20 approve call failed")?;

        if !approve_result["ok"].as_bool().unwrap_or(false) && !dry_run {
            let err = approve_result["error"].as_str().unwrap_or("unknown error");
            return Err(anyhow!("ERC-20 approve failed: {}", err));
        }

        let approve_hash = extract_tx_hash_or_err(&approve_result)?;
        println!("  Approve tx: {}", approve_hash);
        println!("  Waiting 3s for approve to confirm...");
        if !dry_run {
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }

    // Submit swap transaction
    println!("  Broadcasting swap to: {}", to_address);
    println!("  Calldata prefix: {}...", &calldata.chars().take(18).collect::<String>());
    if let Some(v) = eth_value {
        println!("  ETH value: {} wei", v);
    }

    let result = wallet_contract_call_evm(
        from_chain_id,
        &to_address,
        &calldata,
        eth_value,
        dry_run,
    )
    .await
    .context("EVM swap contract call failed")?;

    if !result["ok"].as_bool().unwrap_or(false) && !dry_run {
        let err = result["error"].as_str().unwrap_or("unknown error");
        return Err(anyhow!("EVM swap failed: {}", err));
    }

    Ok(extract_tx_hash_or_err(&result)?)
}

/// Extract base64-encoded serialized transaction from Solana API response.
/// Handles multiple possible response shapes.
fn extract_solana_tx_base64(resp: &serde_json::Value) -> anyhow::Result<String> {
    // Shape 1: { "transaction": "<base64>" }
    if let Some(tx) = resp["transaction"].as_str() {
        return Ok(tx.to_string());
    }
    // Shape 2: { "serializedTx": "<base64>" }
    if let Some(tx) = resp["serializedTx"].as_str() {
        return Ok(tx.to_string());
    }
    // Shape 3: { "tx": "<base64>" }
    if let Some(tx) = resp["tx"].as_str() {
        return Ok(tx.to_string());
    }
    // Shape 4: { "swapTransaction": "<base64>" }
    if let Some(tx) = resp["swapTransaction"].as_str() {
        return Ok(tx.to_string());
    }
    // Shape 5: { "data": "<base64>" }
    if let Some(tx) = resp["data"].as_str() {
        return Ok(tx.to_string());
    }
    Err(anyhow!(
        "Could not find serialized transaction in Solana swap response. Keys present: {:?}",
        resp.as_object().map(|o| o.keys().collect::<Vec<_>>())
    ))
}

/// Map route type to the appropriate Solana program ID
fn route_type_to_solana_program(route_type: &str) -> &'static str {
    match route_type {
        "SWIFT" => crate::config::SWIFT_V2_PROGRAM_ID,
        "MCTP" => crate::config::MCTP_PROGRAM_ID,
        "WH" => crate::config::MAYAN_PROGRAM_ID,
        _ => crate::config::SWIFT_V2_PROGRAM_ID,
    }
}
