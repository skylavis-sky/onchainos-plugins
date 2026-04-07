use serde_json::{json, Value};

use crate::config::{get_chain_config, token_decimals_by_symbol};
use crate::onchainos;
use crate::rpc;

/// Repay a borrow position (identified by GT NFT loanId) on TermMax.
///
/// Uses Router.repayByTokenThroughFt to buy FT and repay in one step.
/// Selector: repayByTokenThroughFt(address,address,uint256,address[],uint128[],uint128,uint256) = 0x84e09091
///
/// Flow:
///   1. Resolve wallet address
///   2. Fetch market tokens (get underlying address)
///   3. Approve RouterV1 to spend underlying (repayment token)
///   4. Call Router.repayByTokenThroughFt(recipient, market, gtId, orders[], ftAmts[], maxTokenIn, deadline)
pub async fn run(
    chain_id: u64,
    market_addr: &str,
    loan_id: u64,
    max_repay_amount: f64,
    underlying_symbol: &str,
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

    // Fetch market tokens
    let tokens = rpc::market_tokens(market_addr, cfg.rpc_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch market tokens: {}", e))?;

    let underlying_addr = &tokens.underlying;
    let decimals = token_decimals_by_symbol(underlying_symbol);
    let max_repay_raw = rpc::human_to_minimal(max_repay_amount, decimals);

    // Deadline: now + 20 minutes
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 1200;

    // Build repayByTokenThroughFt calldata
    // Signature: repayByTokenThroughFt(address recipient, address market, uint256 gtId,
    //             address[] orders, uint128[] ftAmtsWantBuy, uint128 maxTokenIn, uint256 deadline)
    // Selector: 0x84e09091
    let calldata = encode_repay_by_token_through_ft(
        &wallet,
        market_addr,
        loan_id,
        market_addr, // market as order
        max_repay_raw,
        max_repay_raw,
        deadline,
    )?;

    let approve_calldata = encode_approve(cfg.router_v1, max_repay_raw)?;

    if dry_run {
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, underlying_addr, approve_calldata, wallet
        );
        let repay_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, cfg.router_v1, calldata, wallet
        );
        eprintln!("[dry-run] step 1 approve underlying: {}", approve_cmd);
        eprintln!("[dry-run] step 2 repay: {}", repay_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market_addr,
            "loan_id": loan_id,
            "underlying_symbol": underlying_symbol,
            "underlying_address": underlying_addr,
            "max_repay_amount": max_repay_amount,
            "max_repay_raw": max_repay_raw.to_string(),
            "steps": [
                {"step": 1, "action": "approve underlying", "simulatedCommand": approve_cmd},
                {"step": 2, "action": "repayByTokenThroughFt", "simulatedCommand": repay_cmd}
            ],
            "note": "User confirmation required. After repayment, your collateral will be returned and GT NFT burned."
        }));
    }

    // Step 1: Approve underlying for repayment
    eprintln!(
        "Step 1/2: Approving RouterV1 to spend up to {} {}...",
        max_repay_amount, underlying_symbol
    );
    let approve_result = onchainos::erc20_approve(
        chain_id,
        underlying_addr,
        cfg.router_v1,
        max_repay_raw,
        Some(&wallet),
        false,
    )?;

    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)
        .map_err(|e| anyhow::anyhow!("Approve failed: {}", e))?;

    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        rpc::wait_for_tx(cfg.rpc_url, &approve_tx)
            .await
            .map_err(|e| anyhow::anyhow!("Approve tx confirmation failed: {}", e))?;
    }

    // Step 2: Repay
    eprintln!("Step 2/2: Repaying loan #{} on TermMax...", loan_id);
    let repay_result = onchainos::wallet_contract_call(
        chain_id,
        cfg.router_v1,
        &calldata,
        Some(&wallet),
        false,
    )?;

    let repay_tx = onchainos::extract_tx_hash_or_err(&repay_result)
        .map_err(|e| anyhow::anyhow!("Repay failed: {}", e))?;

    Ok(json!({
        "ok": true,
        "market": market_addr,
        "loan_id": loan_id,
        "underlying_symbol": underlying_symbol,
        "max_repay_amount": max_repay_amount,
        "approve_tx_hash": approve_tx,
        "repay_tx_hash": repay_tx,
        "note": "Loan repaid. GT NFT burned, collateral returned. Use get-position to verify."
    }))
}

/// Encode approve(spender, amount). Selector: 0x095ea7b3
fn encode_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender);
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    Ok(format!("0x095ea7b3{}{}", spender_padded, amount_hex))
}

/// Encode repayByTokenThroughFt calldata.
/// Selector: 0x84e09091
///
/// ABI: repayByTokenThroughFt(address recipient, address market, uint256 gtId,
///       address[] orders, uint128[] ftAmtsWantBuy, uint128 maxTokenIn, uint256 deadline)
///
/// Dynamic layout (1 element in each array):
///   [0]  recipient         (address)
///   [1]  market            (address)
///   [2]  gtId              (uint256)
///   [3]  offset(orders)    = 7 * 32 = 224
///   [4]  offset(ftAmts)    = 9 * 32 = 288
///   [5]  maxTokenIn        (uint128)
///   [6]  deadline          (uint256)
///   [7]  orders.length = 1
///   [8]  orders[0]
///   [9]  ftAmts.length = 1
///   [10] ftAmts[0]
fn encode_repay_by_token_through_ft(
    recipient: &str,
    market: &str,
    gt_id: u64,
    order_addr: &str,
    ft_amt_want_buy: u128,
    max_token_in: u128,
    deadline: u64,
) -> anyhow::Result<String> {
    let selector = "84e09091";
    let clean = |s: &str| -> String {
        let c = s.strip_prefix("0x").unwrap_or(s);
        format!("{:0>64}", c)
    };
    let u256_hex = |v: u128| -> String { format!("{:064x}", v) };
    let u64_hex = |v: u64| -> String { format!("{:064x}", v) };

    let offset_orders: u128 = 7 * 32;
    let offset_ft_amts: u128 = 9 * 32;

    Ok(format!(
        "0x{}{}{}{}{}{}{}{}{}{}{}{}",
        selector,
        clean(recipient),
        clean(market),
        u64_hex(gt_id),
        u256_hex(offset_orders),
        u256_hex(offset_ft_amts),
        u256_hex(max_token_in),
        u64_hex(deadline),
        // orders array
        u256_hex(1),
        clean(order_addr),
        // ftAmts array
        u256_hex(1),
        u256_hex(ft_amt_want_buy),
    ))
}
