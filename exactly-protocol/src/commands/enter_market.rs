/// enter-market: Enable an asset as collateral on Exactly Protocol.
///
/// Calls Auditor.enterMarket(address market)
/// selector: 0x3fe5d425
///
/// IMPORTANT: Must be called BEFORE deposits in a market count as collateral for borrowing.
/// Unlike Aave V3, Exactly does NOT auto-enable collateral on deposit.
/// Check isCollateral flag from get-position to see if this is needed.

use serde_json::{json, Value};

use crate::config::{get_chain_config, resolve_market};
use crate::onchainos;

pub async fn run(
    chain_id: u64,
    market_sym: &str,
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

    // Auditor.enterMarket(address market): selector 0x3fe5d425
    let calldata = encode_enter_market(market.market_address)?;

    if dry_run {
        eprintln!("[dry-run] enter-market {} on chain {}", market.symbol, cfg.name);
        eprintln!("[dry-run] Auditor: {}", cfg.auditor);
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "market": market.symbol,
            "marketAddress": market.market_address,
            "auditor": cfg.auditor,
            "step": {
                "action": "enterMarket",
                "to": cfg.auditor,
                "calldata": calldata
            },
            "note": "After this call, your deposits in this market will count as collateral for borrowing."
        }));
    }

    eprintln!("Enabling {} as collateral (enterMarket) on chain {}...", market.symbol, cfg.name);

    let result = onchainos::wallet_contract_call(
        chain_id,
        cfg.auditor,
        &calldata,
        Some(&wallet),
        false,
    )?;
    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    Ok(json!({
        "ok": true,
        "dryRun": false,
        "market": market.symbol,
        "marketAddress": market.market_address,
        "auditor": cfg.auditor,
        "txHash": tx_hash,
        "note": "Your deposits in this market now count as collateral. You can now borrow against them."
    }))
}

// ── Calldata encoders ────────────────────────────────────────────────────────

/// Auditor.enterMarket(address market): selector 0x3fe5d425
fn encode_enter_market(market_addr: &str) -> anyhow::Result<String> {
    let market_clean = market_addr.strip_prefix("0x").unwrap_or(market_addr);
    let market_padded = format!("{:0>64}", market_clean);
    Ok(format!("0x3fe5d425{}", market_padded))
}
