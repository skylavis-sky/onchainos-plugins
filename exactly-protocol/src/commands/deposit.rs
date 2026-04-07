/// deposit: Supply assets to Exactly Protocol (floating or fixed-rate).
///
/// For floating-rate: ERC-20 approve → Market.deposit(assets, receiver)
/// For fixed-rate:    ERC-20 approve → Market.depositAtMaturity(maturity, assets, minAssets, receiver)
///
/// Selectors (from design.md, verified):
///   deposit(uint256,address):                    0x6e553f65
///   depositAtMaturity(uint256,uint256,uint256,address): 0x34f7d1f2

use serde_json::{json, Value};

use crate::config::{apply_slippage_min, get_chain_config, human_to_minimal, resolve_market, SLIPPAGE_BPS};
use crate::onchainos;
use crate::rpc;

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

    // Build calldata
    let (approve_calldata, deposit_calldata, mode) = if let Some(ts) = maturity {
        // Fixed-rate: depositAtMaturity(uint256 maturity, uint256 assets, uint256 minAssetsRequired, address receiver)
        // selector: 0x34f7d1f2
        let min_assets = apply_slippage_min(amount_min, SLIPPAGE_BPS);
        let calldata = encode_deposit_at_maturity(ts, amount_min, min_assets, &wallet)?;
        let approve_cd = encode_erc20_approve(market.market_address, amount_min)?;
        (approve_cd, calldata, format!("fixed (maturity={})", ts))
    } else {
        // Floating-rate: deposit(uint256 assets, address receiver)
        // selector: 0x6e553f65
        let calldata = encode_deposit_floating(amount_min, &wallet)?;
        let approve_cd = encode_erc20_approve(market.market_address, amount_min)?;
        (approve_cd, calldata, "floating".to_string())
    };

    if dry_run {
        eprintln!("[dry-run] deposit {} {} ({}) on chain {}", amount, market.symbol, mode, cfg.name);
        eprintln!("[dry-run] step 1 - approve {} on {}", market.symbol, market.asset_address);
        eprintln!("[dry-run] step 2 - deposit on market {}", market.market_address);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market.symbol,
            "amount": amount,
            "amountMinimal": amount_min.to_string(),
            "mode": mode,
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "to": market.asset_address,
                    "calldata": approve_calldata
                },
                {
                    "step": 2,
                    "action": "deposit",
                    "to": market.market_address,
                    "calldata": deposit_calldata
                }
            ]
        }));
    }

    // Step 1: ERC-20 approve
    eprintln!("Step 1/2: Approving {} {} to market {}...", amount, market.symbol, market.market_address);
    let approve_result = onchainos::wallet_contract_call(
        chain_id,
        market.asset_address,
        &approve_calldata,
        Some(&wallet),
        false,
    )?;
    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)?;
    eprintln!("Approve tx: {}", approve_tx);

    // Wait for approve confirmation before deposit
    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        let _ = rpc::wait_for_tx(cfg.rpc_url, &approve_tx).await;
    } else {
        // Fallback: sleep 3 seconds (nonce collision guard)
        onchainos::sleep_secs(3);
    }

    // Step 2: Deposit
    eprintln!("Step 2/2: Depositing {} {} ({})...", amount, market.symbol, mode);
    let deposit_result = onchainos::wallet_contract_call(
        chain_id,
        market.market_address,
        &deposit_calldata,
        Some(&wallet),
        false,
    )?;
    let deposit_tx = onchainos::extract_tx_hash_or_err(&deposit_result)?;

    Ok(json!({
        "ok": true,
        "dryRun": false,
        "market": market.symbol,
        "marketAddress": market.market_address,
        "asset": market.asset_address,
        "amount": amount,
        "amountMinimal": amount_min.to_string(),
        "mode": mode,
        "approveTxHash": approve_tx,
        "depositTxHash": deposit_tx,
        "warning": "If mode is floating, your deposit earns variable yield. Call enter-market to use as collateral."
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

/// Market.deposit(uint256 assets, address receiver): selector 0x6e553f65
fn encode_deposit_floating(assets: u128, receiver: &str) -> anyhow::Result<String> {
    let assets_hex = format!("{:064x}", assets);
    let receiver_clean = receiver.strip_prefix("0x").unwrap_or(receiver);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    Ok(format!("0x6e553f65{}{}", assets_hex, receiver_padded))
}

/// Market.depositAtMaturity(uint256 maturity, uint256 assets, uint256 minAssetsRequired, address receiver)
/// selector: 0x34f7d1f2
fn encode_deposit_at_maturity(
    maturity: u64,
    assets: u128,
    min_assets: u128,
    receiver: &str,
) -> anyhow::Result<String> {
    let maturity_hex = format!("{:064x}", maturity);
    let assets_hex = format!("{:064x}", assets);
    let min_assets_hex = format!("{:064x}", min_assets);
    let receiver_clean = receiver.strip_prefix("0x").unwrap_or(receiver);
    let receiver_padded = format!("{:0>64}", receiver_clean);
    Ok(format!(
        "0x34f7d1f2{}{}{}{}",
        maturity_hex, assets_hex, min_assets_hex, receiver_padded
    ))
}
