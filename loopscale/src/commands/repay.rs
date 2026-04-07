// commands/repay.rs — Repay a Loopscale loan (may require multiple transactions)
//
// The /v1/markets/creditbook/repay endpoint returns an array of transactions.
// ALL must be submitted sequentially — do not skip any.
use anyhow::Result;
use serde_json::json;

use crate::api;
use crate::config::{from_lamports, to_lamports, MINT_USDC, MINT_WSOL};
use crate::onchainos;

pub async fn run(
    loan_address: String,
    amount: Option<f64>,
    repay_all: bool,
    token: Option<String>,
    dry_run: bool,
) -> Result<()> {
    if amount.is_none() && !repay_all {
        anyhow::bail!("Provide --amount <value> or --all to repay everything.");
    }

    // Fetch loan info to determine principal token and collateral
    let loan_info_resp = api::get_loan_by_address(&loan_address).await?;
    let loan_info = loan_info_resp["loanInfos"]
        .get(0)
        .cloned()
        .unwrap_or(json!({}));

    let ledger = loan_info["ledgers"].get(0).cloned().unwrap_or(json!({}));
    let principal_mint = ledger["principalMint"].as_str().unwrap_or(MINT_USDC);
    let token_sym = token.as_deref().unwrap_or(
        if principal_mint == MINT_WSOL { "SOL" } else { "USDC" }
    );

    let principal_due = ledger["principalDue"].as_u64()
        .or_else(|| ledger["principalDue"].as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0);

    let repay_lamports = if repay_all {
        principal_due
    } else {
        to_lamports(amount.unwrap(), token_sym)
    };

    // Get collateral info for withdrawal params
    let collateral = loan_info["collateral"].get(0).cloned().unwrap_or(json!({}));
    let coll_mint = collateral["assetMint"].as_str().unwrap_or(MINT_USDC).to_string();
    let coll_sym = if coll_mint == MINT_WSOL { "SOL" } else { "USDC" };
    let coll_amount = if repay_all {
        collateral["amount"].as_u64()
            .or_else(|| collateral["amount"].as_str().and_then(|s| s.parse().ok()))
            .unwrap_or(0)
    } else {
        0 // partial repay: don't withdraw collateral
    };

    let preview = json!({
        "operation": "repay",
        "loan_address": loan_address,
        "principal_token": token_sym,
        "principal_due": from_lamports(principal_due, token_sym),
        "repay_amount": from_lamports(repay_lamports, token_sym),
        "repay_all": repay_all,
        "collateral_to_withdraw": from_lamports(coll_amount, coll_sym),
        "collateral_token": coll_sym,
        "note": "Repay may require multiple transactions submitted sequentially"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Build repay transaction(s)
    let repay_resp = api::build_repay_txs(
        &loan_address,
        repay_lamports,
        repay_all,
        &coll_mint,
        coll_amount,
    ).await?;

    // Response: { "transactions": [{ "message": "<BASE64>" }, ...] }
    let txs = repay_resp["transactions"]
        .as_array()
        .cloned()
        .unwrap_or_else(|| {
            // Some endpoints may return single tx under "transaction"
            if let Some(msg) = repay_resp["transaction"]["message"].as_str() {
                vec![json!({ "message": msg })]
            } else {
                vec![]
            }
        });

    if txs.is_empty() {
        anyhow::bail!("No transactions returned by repay API: {}", repay_resp);
    }

    let mut tx_hashes: Vec<String> = Vec::new();
    for (i, tx_entry) in txs.iter().enumerate() {
        let b64_tx = tx_entry["message"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No message in tx {} of repay response: {}", i, tx_entry))?;

        eprintln!("Submitting repay tx {}/{}...", i + 1, txs.len());
        let result = onchainos::submit_solana_tx(b64_tx, &loan_address, false).await?;
        let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;
        eprintln!("  txHash={}", tx_hash);
        tx_hashes.push(tx_hash);
    }

    let solscan_links: Vec<String> = tx_hashes.iter()
        .map(|h| format!("https://solscan.io/tx/{}", h))
        .collect();

    println!("{}", json!({
        "ok": true,
        "data": {
            "operation": "repay",
            "loan_address": loan_address,
            "amount_repaid": from_lamports(repay_lamports, token_sym),
            "token": token_sym,
            "repay_all": repay_all,
            "tx_count": tx_hashes.len(),
            "tx_hashes": tx_hashes,
            "solscan_links": solscan_links,
            "loan_closed": repay_all
        }
    }));
    Ok(())
}
