/// withdraw: Withdraw assets from Exactly Protocol (floating or fixed-rate).
///
/// For floating-rate: Market.withdraw(uint256 assets, address receiver, address owner)
///   selector: 0xb460af94
///   NOTE: Reverts if outstanding debt would make health factor < 1. Clear all borrows first.
///
/// For fixed-rate: Market.withdrawAtMaturity(uint256 maturity, uint256 positionAssets, uint256 minAssetsRequired, address receiver, address owner)
///   selector: 0xa05a091a
///   PITFALL: withdrawing before maturity applies a discount (fewer assets returned).
///            Call previewWithdrawAtMaturity first and inform user of penalty.
///
/// With --all flag: use floatingDepositShares for floating withdrawal via ERC-4626 redeem.

use serde_json::{json, Value};

use crate::config::{apply_slippage_min, get_chain_config, human_to_minimal, resolve_market, SLIPPAGE_BPS};
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    market_sym: &str,
    amount: Option<f64>,
    maturity: Option<u64>,    // None = floating, Some(ts) = fixed
    all: bool,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let market = resolve_market(chain_id, market_sym)?;

    if amount.is_none() && !all {
        anyhow::bail!("Must specify either --amount or --all for withdraw");
    }

    // Resolve wallet address (after dry-run guard)
    let wallet = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    let amount_val = amount.unwrap_or(0.0);
    let amount_min = human_to_minimal(amount_val, market.decimals);

    let (withdraw_calldata, mode) = if let Some(ts) = maturity {
        // Fixed-rate: withdrawAtMaturity
        // PITFALL: before maturity, a discount is applied. Inform user.
        let min_assets = if amount_min > 0 {
            apply_slippage_min(amount_min, SLIPPAGE_BPS)
        } else {
            0
        };
        let position_assets = amount_min; // use provided amount as positionAssets
        let calldata = encode_withdraw_at_maturity(ts, position_assets, min_assets, &wallet, &wallet)?;
        (calldata, format!("fixed (maturity={})", ts))
    } else {
        // Floating-rate: withdraw(assets, receiver, owner)
        // If --all: use uint256.max (but warn about health factor)
        let assets = if all { u128::MAX } else { amount_min };
        if all {
            eprintln!("WARNING: --all uses uint256.max. Ensure all borrows are cleared first to avoid health factor revert.");
        }
        let calldata = encode_withdraw_floating(assets, &wallet, &wallet)?;
        (calldata, if all { "floating (all)".to_string() } else { "floating".to_string() })
    };

    if dry_run {
        eprintln!("[dry-run] withdraw {} {} ({}) on chain {}", amount_val, market.symbol, mode, cfg.name);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market.symbol,
            "amount": amount_val,
            "amountMinimal": amount_min.to_string(),
            "mode": mode,
            "all": all,
            "step": {
                "action": "withdraw",
                "to": market.market_address,
                "calldata": withdraw_calldata
            },
            "warning": "Fixed-rate early withdrawal incurs a discount. Floating withdrawal requires zero borrows."
        }));
    }

    eprintln!("Withdrawing {} {} ({}) on chain {}...", amount_val, market.symbol, mode, cfg.name);

    let withdraw_result = onchainos::wallet_contract_call(
        chain_id,
        market.market_address,
        &withdraw_calldata,
        Some(&wallet),
        false,
    )?;
    let withdraw_tx = onchainos::extract_tx_hash_or_err(&withdraw_result)?;

    Ok(json!({
        "ok": true,
        "dryRun": false,
        "market": market.symbol,
        "marketAddress": market.market_address,
        "amount": amount_val,
        "amountMinimal": amount_min.to_string(),
        "mode": mode,
        "all": all,
        "withdrawTxHash": withdraw_tx,
        "warning": "Fixed-rate early withdrawal may return fewer assets than deposited due to discount."
    }))
}

// ── Calldata encoders ────────────────────────────────────────────────────────

/// Market.withdraw(uint256 assets, address receiver, address owner): selector 0xb460af94
fn encode_withdraw_floating(assets: u128, receiver: &str, owner: &str) -> anyhow::Result<String> {
    let assets_hex = format!("{:064x}", assets);
    let receiver_clean = receiver.strip_prefix("0x").unwrap_or(receiver);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    let owner_clean = owner.strip_prefix("0x").unwrap_or(owner);
    let owner_padded = format!("{:0>64}", owner_clean);
    Ok(format!("0xb460af94{}{}{}", assets_hex, receiver_padded, owner_padded))
}

/// Market.withdrawAtMaturity(uint256 maturity, uint256 positionAssets, uint256 minAssetsRequired, address receiver, address owner)
/// selector: 0xa05a091a
fn encode_withdraw_at_maturity(
    maturity: u64,
    position_assets: u128,
    min_assets: u128,
    receiver: &str,
    owner: &str,
) -> anyhow::Result<String> {
    let maturity_hex = format!("{:064x}", maturity);
    let position_hex = format!("{:064x}", position_assets);
    let min_hex = format!("{:064x}", min_assets);
    let receiver_clean = receiver.strip_prefix("0x").unwrap_or(receiver);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    let owner_clean = owner.strip_prefix("0x").unwrap_or(owner);
    let owner_padded = format!("{:0>64}", owner_clean);
    Ok(format!(
        "0xa05a091a{}{}{}{}{}",
        maturity_hex, position_hex, min_hex, receiver_padded, owner_padded
    ))
}
