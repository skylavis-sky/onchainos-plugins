// commands/borrow.rs — Borrow on Loopscale (two-step: create + borrow)
//
// CRITICAL two-step flow:
//   1. POST /v1/markets/creditbook/create → tx1 (collateral deposit + loan PDA init)
//   2. Submit tx1; get loanAddress from response
//   3. POST /v1/markets/creditbook/borrow → tx2 (draw down principal)
//   4. Submit tx2
//
// The loanAddress from step 1 is REQUIRED for step 2. tx1 must confirm first.
use anyhow::Result;
use serde_json::json;

use crate::api;
use crate::config::{cbps_to_pct, to_lamports, token_to_mint};
use crate::onchainos;

pub async fn run(
    principal_token: String,
    principal_amount: f64,
    collateral_token: String,
    collateral_amount: f64,
    duration: u64,
    duration_type: u8,
    dry_run: bool,
) -> Result<()> {
    // Resolve wallet
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet address. Ensure onchainos is logged in.");
    }

    let principal_mint = token_to_mint(&principal_token);
    let collateral_mint = token_to_mint(&collateral_token);
    let principal_lamports = to_lamports(principal_amount, &principal_token);
    let collateral_lamports = to_lamports(collateral_amount, &collateral_token);

    // Step 0: Get best quote to find strategy address and expected APY
    eprintln!("Fetching best borrow quote for {}/{} pair...", principal_token, collateral_token);
    let quote_resp = api::get_best_quote(
        &wallet,
        principal_mint,
        collateral_mint,
        collateral_lamports,
        duration,
        duration_type,
    ).await?;

    // Handle empty/no-match response from quote API
    let quote_is_empty = quote_resp.as_array().map(|a| a.is_empty()).unwrap_or(false)
        || quote_resp.is_null();

    if quote_is_empty {
        anyhow::bail!(
            "No borrow orders available for {}/{} pair with duration={} durationType={}. \
            Loopscale is an order-book protocol — lenders must post matching offers first. \
            Try different collateral, principal, or duration parameters.",
            principal_token, collateral_token, duration, duration_type
        );
    }

    let strategy = quote_resp["strategy"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No strategy in quote response: {}", quote_resp))?
        .to_string();

    let expected_apy_cbps = quote_resp["apy"].as_u64().unwrap_or(0);
    let ltv_cbps = quote_resp["ltv"].as_u64().unwrap_or(0);

    let preview = json!({
        "operation": "borrow",
        "principal_token": principal_token,
        "principal_amount": principal_amount,
        "principal_lamports": principal_lamports,
        "collateral_token": collateral_token,
        "collateral_amount": collateral_amount,
        "collateral_lamports": collateral_lamports,
        "strategy": strategy,
        "expected_apy": format!("{:.2}%", cbps_to_pct(expected_apy_cbps)),
        "ltv": format!("{:.2}%", cbps_to_pct(ltv_cbps)),
        "duration": duration,
        "duration_type": duration_type,
        "wallet": wallet,
        "steps": "Two-step: tx1 creates loan + deposits collateral; tx2 draws down principal",
        "note": "Amounts are in human-readable units; plugin converts to lamports internally"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // --- STEP 1: Create loan PDA + deposit collateral ---
    eprintln!("Step 1/2: Creating loan and depositing collateral...");
    let create_resp = api::build_borrow_create_tx(
        &wallet,
        collateral_mint,
        collateral_lamports,
        principal_mint,
        principal_lamports,
        &strategy,
        duration,
        duration_type,
    ).await?;

    let b64_tx1 = create_resp["transaction"]["message"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No transaction.message in creditbook/create response: {}", create_resp))?;

    let loan_address = create_resp["loanAddress"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No loanAddress in creditbook/create response: {}", create_resp))?
        .to_string();

    // Submit tx1 — MUST confirm before step 2 (Solana blockhash ~60s TTL)
    let result1 = onchainos::submit_solana_tx(b64_tx1, &strategy, false).await?;
    let tx_hash1 = onchainos::extract_tx_hash_or_err(&result1)?;
    eprintln!("Step 1 confirmed: txHash={}", tx_hash1);

    // --- STEP 2: Draw down principal ---
    eprintln!("Step 2/2: Borrowing principal from loan {}...", loan_address);
    let borrow_resp = api::build_borrow_principal_tx(
        &wallet,
        &loan_address,
        principal_lamports,
        &strategy,
        duration,
        duration_type,
        expected_apy_cbps,
    ).await?;

    let b64_tx2 = borrow_resp["transaction"]["message"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No transaction.message in creditbook/borrow response: {}", borrow_resp))?;

    let result2 = onchainos::submit_solana_tx(b64_tx2, &strategy, false).await?;
    let tx_hash2 = onchainos::extract_tx_hash_or_err(&result2)?;

    println!("{}", json!({
        "ok": true,
        "data": {
            "operation": "borrow",
            "loan_address": loan_address,
            "principal_token": principal_token,
            "principal_borrowed": principal_amount,
            "collateral_token": collateral_token,
            "collateral_deposited": collateral_amount,
            "apy": format!("{:.2}%", cbps_to_pct(expected_apy_cbps)),
            "duration_days": duration,
            "strategy": strategy,
            "tx_create": tx_hash1,
            "tx_borrow": tx_hash2,
            "create_solscan": format!("https://solscan.io/tx/{}", tx_hash1),
            "borrow_solscan": format!("https://solscan.io/tx/{}", tx_hash2)
        }
    }));
    Ok(())
}
