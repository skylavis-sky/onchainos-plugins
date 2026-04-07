/// close-account: Close a Gearbox V3 Credit Account.
///
/// Flow:
///   closeCreditAccount(creditAccount, [decreaseDebt(MAX), withdrawCollateral(underlying, MAX, user)])
///
/// ⚠️  IMPORTANT LIMITATIONS (v0.1):
///   - User must have enough underlying token in their wallet to repay all debt (principal + interest).
///   - Simple close does NOT handle the case where debt > collateral value (needs internal swap).
///   - If you don't have enough underlying to repay, you must add more collateral first,
///     then perform an internal swap via multicall (out of scope for v0.1).
///
/// decreaseDebt(u128::MAX) uses the maximum uint128 value. Gearbox treats any amount
/// >= total debt as full repayment (as documented in §6 G9).

use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::{encode_decrease_debt, encode_withdraw_collateral, encode_close_credit_account};
use crate::config::get_chain_config;
use crate::onchainos::{extract_tx_hash_or_err, resolve_wallet, wallet_contract_call};

pub async fn run(
    chain_id: u64,
    facade: &str,
    credit_account: &str,
    to_addr: Option<&str>,
    underlying_token_addr: &str,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let _cfg = get_chain_config(chain_id)?;

    let user_addr = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else {
        match from {
            Some(addr) => addr.to_string(),
            None => resolve_wallet(chain_id).context("Failed to resolve wallet address")?,
        }
    };

    // Recipient for withdrawn funds (default: user wallet)
    let recipient = to_addr.unwrap_or(&user_addr);

    // Inner calls:
    //   1. decreaseDebt(u128::MAX) — repay all debt
    //   2. withdrawCollateral(underlying, u128::MAX, recipient) — withdraw all
    let decrease_debt_data = encode_decrease_debt(u128::MAX)
        .context("Failed to encode decreaseDebt calldata")?;
    let withdraw_data = encode_withdraw_collateral(underlying_token_addr, u128::MAX, recipient)
        .context("Failed to encode withdrawCollateral calldata")?;

    let inner_calls: Vec<(&str, Vec<u8>)> = vec![
        (facade, decrease_debt_data),
        (facade, withdraw_data),
    ];

    let close_calldata = encode_close_credit_account(credit_account, &inner_calls)
        .context("Failed to encode closeCreditAccount calldata")?;
    let close_calldata_hex = format!("0x{}", hex::encode(&close_calldata));

    if dry_run {
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "chain": chain_id,
            "facade": facade,
            "creditAccount": credit_account,
            "recipient": recipient,
            "underlyingToken": underlying_token_addr,
            "steps": [
                {
                    "step": 1,
                    "action": "closeCreditAccount",
                    "description": format!(
                        "Close account {}: repay all debt, withdraw all collateral to {}",
                        credit_account, recipient
                    ),
                    "to": facade,
                    "inputData": close_calldata_hex
                }
            ],
            "warning": "You must have enough underlying token in your wallet to repay the outstanding debt."
        }));
    }

    let close_result = wallet_contract_call(
        chain_id,
        facade,
        &close_calldata_hex,
        Some(&user_addr),
        false,
    )
    .context("closeCreditAccount failed")?;

    let close_tx = extract_tx_hash_or_err(&close_result)
        .unwrap_or_else(|_| {
            close_result["data"]["txHash"]
                .as_str()
                .or_else(|| close_result["txHash"].as_str())
                .unwrap_or("pending")
                .to_string()
        });

    Ok(json!({
        "ok": true,
        "chain": chain_id,
        "creditAccount": credit_account,
        "recipient": recipient,
        "closeTxHash": close_tx,
        "note": "Account closed. Remaining collateral (after debt repayment) sent to recipient."
    }))
}
