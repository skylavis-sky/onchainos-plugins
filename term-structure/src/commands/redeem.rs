use serde_json::{json, Value};

use crate::config::{get_chain_config, token_decimals_by_symbol};
use crate::onchainos;
use crate::rpc;

/// Redeem FT tokens after market maturity to receive underlying + fixed interest.
///
/// Flow:
///   1. Resolve wallet address
///   2. Check market has matured (maturity < now)
///   3. Fetch FT balance (or use provided amount)
///   4. Call market.redeem(ftAmount, recipient) directly on the TermMaxMarket contract
///
/// Selector: redeem(uint256,address) = 0x7bde82f2
///
/// NOTE: redeem() is called on the MARKET contract directly, NOT the Router.
pub async fn run(
    chain_id: u64,
    market_addr: &str,
    amount: Option<f64>,
    redeem_all: bool,
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

    // Check maturity
    let (_, maturity) = rpc::market_config(market_addr, cfg.rpc_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch market config: {}", e))?;

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if maturity > now_ts && !dry_run {
        let remaining_secs = maturity - now_ts;
        let remaining_days = remaining_secs / 86400;
        return Err(anyhow::anyhow!(
            "Market has not matured yet. {} days remaining until maturity (ts={}).",
            remaining_days,
            maturity
        ));
    }

    // Fetch FT token address
    let tokens = rpc::market_tokens(market_addr, cfg.rpc_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch market tokens: {}", e))?;

    let ft_addr = &tokens.ft;
    let underlying_addr = &tokens.underlying;

    // Infer underlying symbol from known markets
    let underlying_symbol = infer_underlying_symbol(market_addr);
    let decimals = token_decimals_by_symbol(underlying_symbol);

    // Determine FT amount to redeem
    let ft_amount_raw: u128 = if redeem_all || amount.is_none() {
        if dry_run {
            // Use placeholder in dry-run mode
            1_000_000u128 // 1 USDC equivalent placeholder
        } else {
            rpc::erc20_balance(ft_addr, &wallet, cfg.rpc_url)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fetch FT balance: {}", e))?
        }
    } else {
        rpc::human_to_minimal(amount.unwrap(), decimals)
    };

    if ft_amount_raw == 0 && !dry_run {
        return Ok(json!({
            "ok": true,
            "market": market_addr,
            "wallet": wallet,
            "ft_balance": "0",
            "note": "No FT tokens to redeem in this wallet for this market."
        }));
    }

    let ft_human = ft_amount_raw as f64 / 10f64.powi(decimals as i32);

    // Encode redeem(uint256 ftAmount, address recipient)
    // Selector: 0x7bde82f2
    let calldata = encode_redeem(ft_amount_raw, &wallet)?;

    if dry_run {
        let redeem_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {}",
            chain_id, market_addr, calldata, wallet
        );
        eprintln!("[dry-run] redeem: {}", redeem_cmd);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market_addr,
            "ft_address": ft_addr,
            "underlying_address": underlying_addr,
            "ft_amount": ft_human,
            "ft_amount_raw": ft_amount_raw.to_string(),
            "maturity_ts": maturity,
            "maturity_passed": maturity < now_ts || dry_run,
            "steps": [
                {"step": 1, "action": "redeem FT for underlying", "simulatedCommand": redeem_cmd}
            ],
            "note": "User confirmation required. Redeem burns FT tokens and returns underlying + fixed interest."
        }));
    }

    // Execute redeem
    eprintln!(
        "Redeeming {:.6} {} of FT tokens from market {}...",
        ft_human, underlying_symbol, market_addr
    );
    let redeem_result =
        onchainos::wallet_contract_call(chain_id, market_addr, &calldata, Some(&wallet), false)?;

    let redeem_tx = onchainos::extract_tx_hash_or_err(&redeem_result)
        .map_err(|e| anyhow::anyhow!("Redeem failed: {}", e))?;

    Ok(json!({
        "ok": true,
        "market": market_addr,
        "ft_address": ft_addr,
        "underlying_address": underlying_addr,
        "ft_amount": ft_human,
        "ft_amount_raw": ft_amount_raw.to_string(),
        "underlying_symbol": underlying_symbol,
        "redeem_tx_hash": redeem_tx,
        "note": "FT tokens burned. Underlying + fixed interest returned to your wallet."
    }))
}

/// Encode redeem(uint256 ftAmount, address recipient).
/// Selector: 0x7bde82f2
fn encode_redeem(ft_amount: u128, recipient: &str) -> anyhow::Result<String> {
    let selector = "7bde82f2";
    let amount_hex = format!("{:064x}", ft_amount);
    let recipient_clean = recipient.strip_prefix("0x").unwrap_or(recipient);
    let recipient_padded = format!("{:0>64}", recipient_clean);
    Ok(format!("0x{}{}{}", selector, amount_hex, recipient_padded))
}

/// Best-effort infer underlying symbol from known market list.
fn infer_underlying_symbol(market_addr: &str) -> &'static str {
    let lower = market_addr.to_lowercase();
    for m in crate::config::KNOWN_MARKETS {
        if m.address.to_lowercase() == lower {
            return m.underlying_symbol;
        }
    }
    "USDC"
}
