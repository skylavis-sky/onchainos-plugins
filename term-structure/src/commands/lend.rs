use serde_json::{json, Value};

use crate::config::{get_chain_config, token_decimals_by_symbol, DEFAULT_SLIPPAGE_BPS};
use crate::onchainos;
use crate::rpc;

/// Lend (supply) tokens to a TermMax market to earn fixed-rate yield.
///
/// Flow:
///   1. Resolve wallet address
///   2. Fetch market tokens (underlying address, FT address, order list)
///   3. Approve RouterV1 to spend underlying token
///   4. Call Router.swapExactTokenToToken(underlying, FT, wallet, [market], [amount], minOut, deadline)
///
/// User receives FT tokens representing their fixed-rate bond position.
/// At maturity, call `redeem` to convert FT back to underlying + interest.
///
/// Selector: swapExactTokenToToken(...) = 0x1ac100a4
pub async fn run(
    chain_id: u64,
    market_addr: &str,
    amount: f64,
    token_symbol: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    // Resolve wallet — after dry-run guard if there's no early exit
    let wallet = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else {
        match from {
            Some(addr) => addr.to_string(),
            None => onchainos::resolve_wallet(chain_id)?,
        }
    };

    // Fetch market token addresses
    let tokens = rpc::market_tokens(market_addr, cfg.rpc_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch market tokens: {}", e))?;

    let underlying_addr = &tokens.underlying;
    let ft_addr = &tokens.ft;

    // Resolve decimals
    let decimals = token_decimals_by_symbol(token_symbol);
    let amount_raw = rpc::human_to_minimal(amount, decimals);

    // Calculate min FT out with slippage (0.5% default)
    let slippage_factor = 1.0 - (DEFAULT_SLIPPAGE_BPS as f64 / 10_000.0);
    let min_ft_out = (amount_raw as f64 * slippage_factor) as u128;

    // Deadline: now + 20 minutes
    let deadline = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 1200;

    // Build swapExactTokenToToken calldata
    // Signature: swapExactTokenToToken(address tokenIn, address tokenOut, address recipient,
    //             address[] orders, uint128[] tradingAmts, uint128 minTokenOut, uint256 deadline)
    // Selector: 0x1ac100a4
    // ABI encoding for arrays requires offset pointers
    let calldata = encode_swap_exact_token_to_token(
        underlying_addr,
        ft_addr,
        &wallet,
        market_addr, // market address used as the order
        amount_raw,
        min_ft_out,
        deadline,
    )?;

    // Build approve calldata
    let approve_calldata = encode_approve(cfg.router_v1, amount_raw)?;

    if dry_run {
        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, underlying_addr, approve_calldata, wallet
        );
        let lend_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, cfg.router_v1, calldata, wallet
        );
        eprintln!("[dry-run] step 1 approve: {}", approve_cmd);
        eprintln!("[dry-run] step 2 lend (swapExactTokenToToken): {}", lend_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market_addr,
            "underlying": token_symbol,
            "underlying_address": underlying_addr,
            "ft_address": ft_addr,
            "amount": amount,
            "amount_raw": amount_raw.to_string(),
            "min_ft_out": min_ft_out.to_string(),
            "deadline": deadline,
            "steps": [
                {"step": 1, "action": "approve", "simulatedCommand": approve_cmd},
                {"step": 2, "action": "swapExactTokenToToken (lend)", "simulatedCommand": lend_cmd}
            ],
            "note": "User confirmation required before executing lend. Check market liquidity first with get-markets."
        }));
    }

    // Step 1: ERC-20 approve RouterV1 to spend underlying
    eprintln!("Step 1/2: Approving RouterV1 to spend {} {}...", amount, token_symbol);
    let approve_result = onchainos::erc20_approve(
        chain_id,
        underlying_addr,
        cfg.router_v1,
        amount_raw,
        Some(&wallet),
        false,
    )?;

    let approve_tx = onchainos::extract_tx_hash_or_err(&approve_result)
        .map_err(|e| anyhow::anyhow!("Approve failed: {}", e))?;

    // Wait for approve to confirm
    if approve_tx.starts_with("0x") && approve_tx.len() == 66 {
        rpc::wait_for_tx(cfg.rpc_url, &approve_tx)
            .await
            .map_err(|e| anyhow::anyhow!("Approve tx confirmation failed: {}", e))?;
    }

    // Step 2: Router.swapExactTokenToToken (buy FT with underlying)
    eprintln!(
        "Step 2/2: Lending {} {} via TermMax Router...",
        amount, token_symbol
    );
    let lend_result =
        onchainos::wallet_contract_call(chain_id, cfg.router_v1, &calldata, Some(&wallet), false)?;

    let lend_tx = onchainos::extract_tx_hash_or_err(&lend_result)
        .map_err(|e| anyhow::anyhow!("Lend (swapExactTokenToToken) failed: {}", e))?;

    Ok(json!({
        "ok": true,
        "market": market_addr,
        "underlying": token_symbol,
        "underlying_address": underlying_addr,
        "ft_address": ft_addr,
        "amount": amount,
        "amount_raw": amount_raw.to_string(),
        "min_ft_out": min_ft_out.to_string(),
        "approve_tx_hash": approve_tx,
        "lend_tx_hash": lend_tx,
        "note": "FT tokens credited to your wallet. Hold until market maturity, then call 'redeem' to receive underlying + fixed interest."
    }))
}

/// Encode ERC-20 approve(spender, amount) calldata.
/// Selector: 0x095ea7b3
fn encode_approve(spender: &str, amount: u128) -> anyhow::Result<String> {
    let spender_clean = spender.strip_prefix("0x").unwrap_or(spender);
    let spender_padded = format!("{:0>64}", spender_clean);
    let amount_hex = format!("{:064x}", amount);
    Ok(format!("0x095ea7b3{}{}", spender_padded, amount_hex))
}

/// Encode swapExactTokenToToken calldata.
/// Selector: 0x1ac100a4
///
/// ABI signature:
/// swapExactTokenToToken(address tokenIn, address tokenOut, address recipient,
///                       address[] orders, uint128[] tradingAmts,
///                       uint128 minTokenOut, uint256 deadline)
///
/// Dynamic ABI encoding with head/tail layout:
///   [0]  tokenIn      (address, 32 bytes)
///   [1]  tokenOut     (address, 32 bytes)
///   [2]  recipient    (address, 32 bytes)
///   [3]  offset(orders)   -> 7 * 32 = 224 (0xe0)
///   [4]  offset(tradingAmts) -> 9 * 32 = 288 (0x120)
///   [5]  minTokenOut  (uint128, 32 bytes)
///   [6]  deadline     (uint256, 32 bytes)
///   [7]  orders.length = 1
///   [8]  orders[0]    (address)
///   [9]  tradingAmts.length = 1
///   [10] tradingAmts[0] (uint128)
fn encode_swap_exact_token_to_token(
    token_in: &str,
    token_out: &str,
    recipient: &str,
    order_addr: &str,
    trading_amt: u128,
    min_token_out: u128,
    deadline: u64,
) -> anyhow::Result<String> {
    let selector = "1ac100a4";

    let clean = |s: &str| -> String {
        let c = s.strip_prefix("0x").unwrap_or(s);
        format!("{:0>64}", c)
    };

    let u128_hex = |v: u128| -> String { format!("{:064x}", v) };
    let u64_hex = |v: u64| -> String { format!("{:064x}", v) };

    // Static slots 0..6, then dynamic data starts at slot 7
    // Offsets are byte offsets from the start of the ABI-encoded params (after selector)
    // slot 0 = tokenIn     (32 bytes)  offset 0
    // slot 1 = tokenOut    (32 bytes)  offset 32
    // slot 2 = recipient   (32 bytes)  offset 64
    // slot 3 = offset(orders)          offset 96  -> points to slot 7 = 7*32 = 224
    // slot 4 = offset(tradingAmts)     offset 128 -> points to slot 9 = 9*32 = 288
    // slot 5 = minTokenOut (uint128)   offset 160
    // slot 6 = deadline    (uint256)   offset 192
    // slot 7 = orders.length = 1       offset 224
    // slot 8 = orders[0]               offset 256
    // slot 9 = tradingAmts.length = 1  offset 288
    // slot 10 = tradingAmts[0]         offset 320

    let offset_orders: u128 = 7 * 32; // 224 = 0xe0
    let offset_trading_amts: u128 = 9 * 32; // 288 = 0x120

    let encoded = format!(
        "0x{}{}{}{}{}{}{}{}{}{}{}{}",
        selector,
        clean(token_in),
        clean(token_out),
        clean(recipient),
        u128_hex(offset_orders),
        u128_hex(offset_trading_amts),
        u128_hex(min_token_out),
        u64_hex(deadline),
        // orders array: length=1, [0]=order_addr
        u128_hex(1),
        clean(order_addr),
        // tradingAmts array: length=1, [0]=trading_amt
        u128_hex(1),
        u128_hex(trading_amt),
    );

    Ok(encoded)
}
