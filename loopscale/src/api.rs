// src/api.rs — HTTP client for tars.loopscale.com
use anyhow::Context;
use serde_json::{json, Value};

const API_BASE: &str = "https://tars.loopscale.com";

/// Build a reqwest client that respects the HTTPS_PROXY env var (required in onchainos sandbox).
pub fn build_client() -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder();
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY") {
        builder = builder.proxy(
            reqwest::Proxy::https(&proxy_url)
                .context("Invalid HTTPS_PROXY value")?,
        );
    }
    Ok(builder.build()?)
}

// ---------------------------------------------------------------------------
// Vault / market reads
// ---------------------------------------------------------------------------

/// GET /v1/markets/lending_vaults/deposits — list all vaults with depositor data.
/// Pass `principal_mints` to filter by token (empty = all).
pub async fn get_vaults(principal_mints: Vec<&str>) -> anyhow::Result<Value> {
    let client = build_client()?;
    let mut body = json!({});
    if !principal_mints.is_empty() {
        body["principalMints"] = json!(principal_mints);
    }
    let resp = client
        .post(format!("{}/v1/markets/lending_vaults/deposits", API_BASE))
        .json(&body)
        .send()
        .await
        .context("GET /v1/markets/lending_vaults/deposits failed")?
        .json::<Value>()
        .await
        .context("Failed to parse vaults response")?;
    Ok(resp)
}

/// POST /v1/markets/quote — list all available borrow quotes for a principal/collateral pair.
#[allow(dead_code)]
pub async fn get_quotes(
    principal_mint: &str,
    collateral_mints: Vec<&str>,
    duration: u64,
    duration_type: u8,
    limit: usize,
    offset: usize,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let collateral: Vec<Value> = collateral_mints.iter().map(|m| json!(m)).collect();
    let body = json!({
        "principal": principal_mint,
        "collateral": collateral,
        "durationType": duration_type,
        "duration": duration,
        "limit": limit,
        "offset": offset
    });
    let resp = client
        .post(format!("{}/v1/markets/quote", API_BASE))
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/quote failed")?
        .json::<Value>()
        .await
        .context("Failed to parse quote response")?;
    Ok(resp)
}

/// POST /v1/markets/quote/max — get the best single quote for a collateral/principal pair.
pub async fn get_best_quote(
    wallet: &str,
    principal_mint: &str,
    collateral_mint: &str,
    collateral_lamports: u64,
    duration: u64,
    duration_type: u8,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "principalMint": principal_mint,
        "collateralFilter": [
            {
                "amount": collateral_lamports,
                "assetData": { "Spl": { "mint": collateral_mint } }
            }
        ],
        "durationType": duration_type,
        "duration": duration
    });
    let resp = client
        .post(format!("{}/v1/markets/quote/max", API_BASE))
        .header("user-wallet", wallet)
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/quote/max failed")?
        .json::<Value>()
        .await
        .context("Failed to parse best-quote response")?;
    Ok(resp)
}

/// POST /v1/markets/loans/info — list active loans for a borrower wallet.
pub async fn get_loans(wallet: &str, filter_type: u8, page_size: usize, page: usize) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "borrowers": [wallet],
        "filterType": filter_type,
        "pageSize": page_size,
        "page": page
    });
    let resp = client
        .post(format!("{}/v1/markets/loans/info", API_BASE))
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/loans/info failed")?
        .json::<Value>()
        .await
        .context("Failed to parse loans response")?;
    Ok(resp)
}

/// POST /v1/markets/loans/info — get info for specific loan addresses.
pub async fn get_loan_by_address(loan_address: &str) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "loanAddresses": [loan_address],
        "filterType": 0
    });
    let resp = client
        .post(format!("{}/v1/markets/loans/info", API_BASE))
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/loans/info (by address) failed")?
        .json::<Value>()
        .await
        .context("Failed to parse loan-by-address response")?;
    Ok(resp)
}

// ---------------------------------------------------------------------------
// Write operation TX builders
// ---------------------------------------------------------------------------

/// POST /v1/markets/lending_vaults/deposit — build a vault deposit transaction.
pub async fn build_lend_tx(
    wallet: &str,
    vault: &str,
    principal_lamports: u64,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "principalAmount": principal_lamports,
        "minLpAmount": 0,
        "vault": vault
    });
    let resp = client
        .post(format!("{}/v1/markets/lending_vaults/deposit", API_BASE))
        .header("user-wallet", wallet)
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/lending_vaults/deposit failed")?
        .json::<Value>()
        .await
        .context("Failed to parse deposit response")?;
    Ok(resp)
}

/// POST /v1/markets/lending_vaults/withdraw — build a vault withdrawal transaction.
pub async fn build_withdraw_tx(
    wallet: &str,
    vault: &str,
    amount_lamports: u64,
    withdraw_all: bool,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "amountPrincipal": amount_lamports,
        "maxAmountLp": 0,
        "vault": vault,
        "withdrawAll": withdraw_all
    });
    let resp = client
        .post(format!("{}/v1/markets/lending_vaults/withdraw", API_BASE))
        .header("user-wallet", wallet)
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/lending_vaults/withdraw failed")?
        .json::<Value>()
        .await
        .context("Failed to parse withdraw response")?;
    Ok(resp)
}

/// POST /v1/markets/creditbook/create — Step 1 of borrow: create loan PDA + deposit collateral.
pub async fn build_borrow_create_tx(
    wallet: &str,
    collateral_mint: &str,
    collateral_lamports: u64,
    principal_mint: &str,
    principal_lamports: u64,
    strategy: &str,
    duration: u64,
    duration_type: u8,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "depositCollateral": [
            {
                "amount": collateral_lamports,
                "assetData": { "Spl": { "mint": collateral_mint } }
            }
        ],
        "borrower": wallet,
        "principalRequested": [
            {
                "ledger": 0,
                "amount": principal_lamports,
                "mint": principal_mint,
                "strategy": strategy,
                "duration": duration,
                "durationType": duration_type
            }
        ]
    });
    let resp = client
        .post(format!("{}/v1/markets/creditbook/create", API_BASE))
        .header("payer", wallet)
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/creditbook/create failed")?
        .json::<Value>()
        .await
        .context("Failed to parse creditbook/create response")?;
    Ok(resp)
}

/// POST /v1/markets/creditbook/borrow — Step 2 of borrow: draw down principal.
pub async fn build_borrow_principal_tx(
    wallet: &str,
    loan_address: &str,
    principal_lamports: u64,
    strategy: &str,
    duration: u64,
    duration_type: u8,
    expected_apy_cbps: u64,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "loan": loan_address,
        "borrowParams": {
            "amount": principal_lamports,
            "duration": {
                "duration": duration,
                "durationType": duration_type
            },
            "expectedLoanValues": {
                "apy": expected_apy_cbps
            }
        },
        "strategy": strategy
    });
    let resp = client
        .post(format!("{}/v1/markets/creditbook/borrow", API_BASE))
        .header("payer", wallet)
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/creditbook/borrow failed")?
        .json::<Value>()
        .await
        .context("Failed to parse creditbook/borrow response")?;
    Ok(resp)
}

/// POST /v1/markets/creditbook/repay — build repayment transaction(s).
/// Returns array of transactions that must be submitted sequentially.
pub async fn build_repay_txs(
    loan_address: &str,
    repay_amount_lamports: u64,
    repay_all: bool,
    collateral_mint: &str,
    collateral_withdraw_lamports: u64,
) -> anyhow::Result<Value> {
    let client = build_client()?;
    let body = json!({
        "loan": loan_address,
        "repayParams": [
            {
                "amount": repay_amount_lamports,
                "ledgerIndex": 0,
                "repayAll": repay_all
            }
        ],
        "collateralWithdrawalParams": [
            {
                "amount": collateral_withdraw_lamports,
                "mint": collateral_mint
            }
        ],
        "closeIfPossible": true
    });
    let resp = client
        .post(format!("{}/v1/markets/creditbook/repay", API_BASE))
        .json(&body)
        .send()
        .await
        .context("POST /v1/markets/creditbook/repay failed")?
        .json::<Value>()
        .await
        .context("Failed to parse repay response")?;
    Ok(resp)
}
