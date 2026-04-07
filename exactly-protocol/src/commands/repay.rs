/// repay: Repay borrowed assets on Exactly Protocol (floating or fixed-rate).
///
/// For floating-rate: ERC-20 approve → Market.refund(uint256 borrowShares, address borrower)
///   NOTE: uses SHARES not assets. Pass borrowShares from get-position output.
///   selector: 0x7ad226dc
///
/// For fixed-rate: ERC-20 approve → Market.repayAtMaturity(uint256 maturity, uint256 positionAssets, uint256 maxAssets, address borrower)
///   PITFALL: Do NOT pass uint256.max as positionAssets — contract pulls full debt which may exceed balance.
///   Pass positionAssets from get-position, with a 0.1% buffer on maxAssets.
///   selector: 0x3c6f317f
///
/// ERC-20 approve IS required for both refund (floating) and repayAtMaturity (fixed).

use serde_json::{json, Value};

use crate::config::{apply_slippage_max, get_chain_config, human_to_minimal, resolve_market, SLIPPAGE_BPS};
use crate::onchainos;
use crate::rpc;

pub async fn run(
    chain_id: u64,
    market_sym: &str,
    amount: f64,
    maturity: Option<u64>,    // None = floating (refund), Some(ts) = fixed (repayAtMaturity)
    borrow_shares: Option<u128>, // For floating repay: borrowShares from get-position
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let market = resolve_market(chain_id, market_sym)?;

    // Resolve wallet address (after dry-run guard)
    let wallet = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let amount_min = human_to_minimal(amount, market.decimals);

    let (approve_calldata, repay_calldata, mode) = if let Some(ts) = maturity {
        // Fixed-rate: repayAtMaturity
        // PITFALL: maxAssets must include a buffer so contract doesn't revert if interest accrued.
        let max_assets = apply_slippage_max(amount_min, 10); // 0.1% buffer (safer than 1%)
        let calldata = encode_repay_at_maturity(ts, amount_min, max_assets, &wallet)?;
        // approve max_assets (with buffer) to cover potential interest accrual
        let approve_cd = encode_erc20_approve(market.market_address, max_assets)?;
        (approve_cd, calldata, format!("fixed (maturity={})", ts))
    } else {
        // Floating-rate: refund(borrowShares, borrower)
        // If borrow_shares not provided, use amount_min as a fallback (user should provide --borrow-shares)
        let shares = borrow_shares.unwrap_or(amount_min);
        let calldata = encode_refund(shares, &wallet)?;
        // For refund, we need to approve the UNDERLYING amount.
        // The actual amount pulled depends on shares * exchange rate.
        // Approve amount_min with 1% buffer as safety margin.
        let approve_amount = apply_slippage_max(amount_min, SLIPPAGE_BPS);
        let approve_cd = encode_erc20_approve(market.market_address, approve_amount)?;
        (approve_cd, calldata, "floating (refund)".to_string())
    };

    if dry_run {
        eprintln!("[dry-run] repay {} {} ({}) on chain {}", amount, market.symbol, mode, cfg.name);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market.symbol,
            "amount": amount,
            "amountMinimal": amount_min.to_string(),
            "mode": mode,
            "borrowShares": borrow_shares.map(|s| s.to_string()),
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "to": market.asset_address,
                    "calldata": approve_calldata
                },
                {
                    "step": 2,
                    "action": "repay",
                    "to": market.market_address,
                    "calldata": repay_calldata
                }
            ],
            "warning": "For floating repay: pass --borrow-shares from get-position output. For fixed repay: positionAssets from get-position."
        }));
    }

    // Step 1: ERC-20 approve
    eprintln!("Step 1/2: Approving {} for repay...", market.symbol);
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        market.asset_address,
        &approve_calldata,
        Some(&wallet),
        false,
    )?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    eprintln!("Approve tx: {}", approve_tx);

    // Wait for approve confirmation
    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        let _ = rpc::wait_for_tx(cfg.rpc_url, &approve_tx).await;
    } else {
        onchainos::sleep_secs(3);
    }

    // Step 2: Repay
    eprintln!("Step 2/2: Repaying {} {} ({})...", amount, market.symbol, mode);
    let repay_result = onchainos::wallet_contract_call(
        chain_id,
        market.market_address,
        &repay_calldata,
        Some(&wallet),
        false,
    )?;
    let repay_tx = onchainos::extract_tx_hash_or_err(&repay_result)?;

    Ok(json!({
        "ok": true,
        "dryRun": false,
        "market": market.symbol,
        "marketAddress": market.market_address,
        "amount": amount,
        "amountMinimal": amount_min.to_string(),
        "mode": mode,
        "borrowShares": borrow_shares.map(|s| s.to_string()),
        "approveTxHash": approve_tx,
        "repayTxHash": repay_tx
    }))
}

// ── Calldata encoders ────────────────────────────────────────────────────────

/// ERC-20 approve(address spender, uint256 amount): selector 0x095ea7b3
fn encode_erc20_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender);
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    Ok(format!("0x095ea7b3{}{}", spender_padded, amount_hex))
}

/// Market.refund(uint256 borrowShares, address borrower): selector 0x7ad226dc
/// NOTE: This is the floating-rate repay function. Takes SHARES not assets.
fn encode_refund(borrow_shares: u128, borrower: &str) -> anyhow::Result<String> {
    let shares_hex = format!("{:064x}", borrow_shares);
    let borrower_clean = borrower.strip_prefix("0x").unwrap_or(borrower);
    let borrower_padded = format!("{:0>64}", borrower_clean);
    Ok(format!("0x7ad226dc{}{}", shares_hex, borrower_padded))
}

/// Market.repayAtMaturity(uint256 maturity, uint256 positionAssets, uint256 maxAssets, address borrower)
/// selector: 0x3c6f317f
/// PITFALL: Do NOT pass uint256.max as positionAssets. Always use actual positionAssets + 0.1% buffer on maxAssets.
fn encode_repay_at_maturity(
    maturity: u64,
    position_assets: u128,
    max_assets: u128,
    borrower: &str,
) -> anyhow::Result<String> {
    let maturity_hex = format!("{:064x}", maturity);
    let position_hex = format!("{:064x}", position_assets);
    let max_hex = format!("{:064x}", max_assets);
    let borrower_clean = borrower.strip_prefix("0x").unwrap_or(borrower);
    let borrower_padded = format!("{:0>64}", borrower_clean);
    Ok(format!(
        "0x3c6f317f{}{}{}{}",
        maturity_hex, position_hex, max_hex, borrower_padded
    ))
}
