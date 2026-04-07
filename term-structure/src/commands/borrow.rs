use serde_json::{json, Value};

use crate::config::{get_chain_config, token_decimals_by_symbol};
use crate::onchainos;
use crate::rpc;

/// Borrow tokens from a TermMax market by posting collateral.
///
/// Flow:
///   1. Resolve wallet address
///   2. Approve RouterV1 to spend collateral token
///   3. Call Router.borrowTokenFromCollateral(recipient, market, collInAmt, borrowAmt)
///
/// User receives borrowed underlying tokens; a GT NFT (loanId) is minted representing
/// the debt position. Repay before or at maturity to avoid liquidation.
///
/// Simple borrow selector: borrowTokenFromCollateral(address,address,uint256,uint256) = 0x95320fd0
pub async fn run(
    chain_id: u64,
    market_addr: &str,
    collateral_amount: f64,
    collateral_symbol: &str,
    borrow_amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve wallet
    let wallet = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else {
        match from {
            Some(addr) => addr.to_string(),
            None => onchainos::resolve_wallet(chain_id)?,
        }
    };

    // Fetch market tokens to get collateral address
    let tokens = rpc::market_tokens(market_addr, cfg.rpc_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch market tokens: {}", e))?;

    let collateral_addr = &tokens.collateral;
    let underlying_addr = &tokens.underlying;

    // Determine decimals
    let collateral_decimals = token_decimals_by_symbol(collateral_symbol);

    // Infer underlying symbol from market config (best effort)
    let underlying_symbol = infer_underlying_symbol(market_addr);

    let underlying_decimals = token_decimals_by_symbol(underlying_symbol);

    let coll_amt_raw = rpc::human_to_minimal(collateral_amount, collateral_decimals);
    let borrow_amt_raw = rpc::human_to_minimal(borrow_amount, underlying_decimals);

    // Build calldata for borrowTokenFromCollateral(recipient, market, collInAmt, borrowAmt)
    // Selector: 0x95320fd0
    let calldata = encode_borrow_from_collateral(
        &wallet,
        market_addr,
        coll_amt_raw,
        borrow_amt_raw,
    )?;

    let approve_calldata = encode_approve(cfg.router_v1, coll_amt_raw)?;

    if dry_run {
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, collateral_addr, approve_calldata, wallet
        );
        let borrow_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, cfg.router_v1, calldata, wallet
        );
        eprintln!("[dry-run] step 1 approve collateral: {}", approve_cmd);
        eprintln!("[dry-run] step 2 borrow: {}", borrow_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market_addr,
            "collateral_symbol": collateral_symbol,
            "collateral_address": collateral_addr,
            "underlying_address": underlying_addr,
            "collateral_amount": collateral_amount,
            "collateral_amount_raw": coll_amt_raw.to_string(),
            "borrow_amount": borrow_amount,
            "borrow_amount_raw": borrow_amt_raw.to_string(),
            "steps": [
                {"step": 1, "action": "approve collateral", "simulatedCommand": approve_cmd},
                {"step": 2, "action": "borrowTokenFromCollateral", "simulatedCommand": borrow_cmd}
            ],
            "note": "User confirmation required. A GT NFT (loanId) will be minted. Use 'repay --loan-id <id>' before maturity."
        }));
    }

    // Step 1: Approve collateral
    eprintln!(
        "Step 1/2: Approving RouterV1 to spend {} {}...",
        collateral_amount, collateral_symbol
    );
    let approve_result = onchainos::erc20_approve(
        chain_id,
        collateral_addr,
        cfg.router_v1,
        coll_amt_raw,
        Some(&wallet),
        false,
    )?;

    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)
        .map_err(|e| anyhow::anyhow!("Collateral approve failed: {}", e))?;

    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        rpc::wait_for_tx(cfg.rpc_url, &approve_tx)
            .await
            .map_err(|e| anyhow::anyhow!("Approve tx confirmation failed: {}", e))?;
    }

    // Step 2: Borrow
    eprintln!(
        "Step 2/2: Borrowing {} {} from TermMax market...",
        borrow_amount, underlying_symbol
    );
    let borrow_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.router_v1,
        &calldata,
        Some(&wallet),
        false,
    )?;

    let borrow_tx = onchainos::extract_tx_hash_or_err(&borrow_result)
        .map_err(|e| anyhow::anyhow!("Borrow failed: {}", e))?;

    Ok(json!({
        "ok": true,
        "market": market_addr,
        "collateral_symbol": collateral_symbol,
        "collateral_address": collateral_addr,
        "underlying_address": underlying_addr,
        "collateral_amount": collateral_amount,
        "borrow_amount": borrow_amount,
        "approve_tx_hash": approve_tx,
        "borrow_tx_hash": borrow_tx,
        "note": "GT NFT (loanId) minted. Use 'get-position' to view loanId. Repay before maturity to avoid liquidation."
    }))
}

/// Encode approve(spender, amount) calldata. Selector: 0x095ea7b3
fn encode_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender);
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    Ok(format!("0x095ea7b3{}{}", spender_padded, amount_hex))
}

/// Encode borrowTokenFromCollateral(address recipient, address market, uint256 collInAmt, uint256 borrowAmt)
/// Selector: 0x95320fd0
fn encode_borrow_from_collateral(
    recipient: &str,
    market: &str,
    coll_amt: u128,
    borrow_amt: u128,
) -> anyhow::Result<String> {
    let selector = "95320fd0";
    let clean_addr = |s: &str| -> String {
        let c = s.strip_prefix("0x").unwrap_or(s);
        format!("{:0>64}", c)
    };
    let u256_hex = |v: u128| -> String { format!("{:064x}", v) };

    Ok(format!(
        "0x{}{}{}{}{}",
        selector,
        clean_addr(recipient),
        clean_addr(market),
        u256_hex(coll_amt),
        u256_hex(borrow_amt),
    ))
}

/// Best-effort infer underlying symbol from known market list.
fn infer_underlying_symbol(market_addr: &str) -> &'static str {
    let lower = market_addr.to_lowercase();
    for m in crate::config::KNOWN_MARKETS {
        if m.address.to_lowercase() == lower {
            return m.underlying_symbol;
        }
    }
    "USDC" // most common underlying
}
