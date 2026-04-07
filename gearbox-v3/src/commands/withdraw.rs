/// withdraw: Withdraw collateral from a Credit Account.
///
/// Uses: multicall(creditAccount, [withdrawCollateral(token, amount, to)])
///
/// ⚠️  Health factor is checked on-chain after the call.
///     If HF < 1.0, the transaction will revert.
///     Partial withdrawals are allowed as long as HF >= 1.0 post-withdrawal.

use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::{encode_withdraw_collateral, encode_multicall, human_to_minimal, infer_decimals};
use crate::config::get_chain_config;
use crate::onchainos::{extract_tx_hash_or_err, resolve_wallet, wallet_contract_call};

pub async fn run(
    chain_id: u64,
    facade: &str,
    credit_account: &str,
    token: &str,
    token_addr: &str,
    amount: Option<f64>,   // None = withdraw all (u128::MAX)
    to_addr: Option<&str>,
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

    let recipient = to_addr.unwrap_or(&user_addr);

    let decimals = infer_decimals(token);
    let (amount_raw, amount_display) = match amount {
        Some(a) => (human_to_minimal(a, decimals), format!("{} {}", a, token)),
        None => (u128::MAX, format!("all {}", token)),
    };

    // Encode withdrawCollateral inner call (target = facade)
    let withdraw_data = encode_withdraw_collateral(token_addr, amount_raw, recipient)
        .context("Failed to encode withdrawCollateral calldata")?;

    let inner_calls: Vec<(&str, Vec<u8>)> = vec![(facade, withdraw_data)];

    let mc_data = encode_multicall(credit_account, &inner_calls)
        .context("Failed to encode multicall calldata")?;
    let mc_hex = format!("0x{}", hex::encode(&mc_data));

    if dry_run {
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "chain": chain_id,
            "creditAccount": credit_account,
            "facade": facade,
            "token": token,
            "tokenAddress": token_addr,
            "amount": amount_display,
            "recipient": recipient,
            "steps": [
                {
                    "step": 1,
                    "action": "multicall",
                    "description": format!("withdrawCollateral: {} from account {} to {}", amount_display, credit_account, recipient),
                    "to": facade,
                    "inputData": mc_hex
                }
            ],
            "warning": "Transaction will revert if health factor drops below 1.0 after withdrawal."
        }));
    }

    let mc_result = wallet_contract_call(
        chain_id,
        facade,
        &mc_hex,
        Some(&user_addr),
        false,
    )
    .context("multicall (withdrawCollateral) failed")?;

    let mc_tx = extract_tx_hash_or_err(&mc_result)
        .unwrap_or_else(|_| {
            mc_result["data"]["txHash"]
                .as_str()
                .or_else(|| mc_result["txHash"].as_str())
                .unwrap_or("pending")
                .to_string()
        });

    Ok(json!({
        "ok": true,
        "chain": chain_id,
        "creditAccount": credit_account,
        "token": token,
        "amount": amount_display,
        "recipient": recipient,
        "txHash": mc_tx
    }))
}
