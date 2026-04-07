/// borrow: Borrow assets from Exactly Protocol (floating or fixed-rate).
///
/// For floating-rate: Market.borrow(uint256 assets, address receiver, address borrower)
/// For fixed-rate:    Market.borrowAtMaturity(uint256 maturity, uint256 assets, uint256 maxAssets, address receiver, address borrower)
///
/// IMPORTANT: borrower must have called enterMarket on their collateral first.
/// No ERC-20 approve required for borrows.
///
/// Selectors (from design.md):
///   borrow(uint256,address,address):                        0xd5164184
///   borrowAtMaturity(uint256,uint256,uint256,address,address): 0x1a5b9e62

use serde_json::{json, Value};

use crate::config::{apply_slippage_max, get_chain_config, human_to_minimal, resolve_market, SLIPPAGE_BPS};
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    market_sym: &str,
    amount: f64,
    maturity: Option<u64>,    // None = floating, Some(ts) = fixed
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

    let (borrow_calldata, mode) = if let Some(ts) = maturity {
        // Fixed-rate: borrowAtMaturity(uint256 maturity, uint256 assets, uint256 maxAssets, address receiver, address borrower)
        // selector: 0x1a5b9e62
        let max_assets = apply_slippage_max(amount_min, SLIPPAGE_BPS);
        let calldata = encode_borrow_at_maturity(ts, amount_min, max_assets, &wallet, &wallet)?;
        (calldata, format!("fixed (maturity={})", ts))
    } else {
        // Floating-rate: borrow(uint256 assets, address receiver, address borrower)
        // selector: 0xd5164184
        let calldata = encode_borrow_floating(amount_min, &wallet, &wallet)?;
        (calldata, "floating".to_string())
    };

    if dry_run {
        eprintln!("[dry-run] borrow {} {} ({}) on chain {}", amount, market.symbol, mode, cfg.name);
        eprintln!("[dry-run] WARNING: enterMarket must be called on collateral market first!");
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market.symbol,
            "amount": amount,
            "amountMinimal": amount_min.to_string(),
            "mode": mode,
            "step": {
                "action": "borrow",
                "to": market.market_address,
                "calldata": borrow_calldata
            },
            "warning": "You must have called enter-market on your collateral first. No ERC-20 approve needed for borrow."
        }));
    }

    eprintln!("Borrowing {} {} ({}) on chain {}...", amount, market.symbol, mode, cfg.name);
    eprintln!("Note: ensure you have collateral enabled via enter-market before borrowing.");

    let borrow_result = onchainos::wallet_contract_call(
        chain_id,
        market.market_address,
        &borrow_calldata,
        Some(&wallet),
        false,
    )?;
    let borrow_tx = onchainos::extract_tx_hash_or_err(&borrow_result)?;

    Ok(json!({
        "ok": true,
        "dryRun": false,
        "market": market.symbol,
        "marketAddress": market.market_address,
        "amount": amount,
        "amountMinimal": amount_min.to_string(),
        "mode": mode,
        "borrowTxHash": borrow_tx,
        "warning": "Repay before maturity to avoid penalty fees. Use repay command to repay."
    }))
}

// ── Calldata encoders ────────────────────────────────────────────────────────

/// Market.borrow(uint256 assets, address receiver, address borrower): selector 0xd5164184
fn encode_borrow_floating(assets: u128, receiver: &str, borrower: &str) -> anyhow::Result<String> {
    let assets_hex = format!("{:064x}", assets);
    let receiver_clean = receiver.strip_prefix("0x").unwrap_or(receiver);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    let borrower_clean = borrower.strip_prefix("0x").unwrap_or(borrower);
    let borrower_padded = format!("{:0>64}", borrower_clean);
    Ok(format!("0xd5164184{}{}{}", assets_hex, receiver_padded, borrower_padded))
}

/// Market.borrowAtMaturity(uint256 maturity, uint256 assets, uint256 maxAssets, address receiver, address borrower)
/// selector: 0x1a5b9e62
fn encode_borrow_at_maturity(
    maturity: u64,
    assets: u128,
    max_assets: u128,
    receiver: &str,
    borrower: &str,
) -> anyhow::Result<String> {
    let maturity_hex = format!("{:064x}", maturity);
    let assets_hex = format!("{:064x}", assets);
    let max_assets_hex = format!("{:064x}", max_assets);
    let receiver_clean = receiver.strip_prefix("0x").unwrap_or(receiver);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    let borrower_clean = borrower.strip_prefix("0x").unwrap_or(borrower);
    let borrower_padded = format!("{:0>64}", borrower_clean);
    Ok(format!(
        "0x1a5b9e62{}{}{}{}{}",
        maturity_hex, assets_hex, max_assets_hex, receiver_padded, borrower_padded
    ))
}
