/// add-collateral: Add collateral to an existing Credit Account.
///
/// Flow:
///   1. Resolve wallet
///   2. ERC-20 approve(CreditManagerV3, amount) — approve to MANAGER not facade
///   3. multicall(creditAccount, [addCollateral(token, amount)])

use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::{encode_add_collateral, encode_multicall, human_to_minimal, infer_decimals};
use crate::config::get_chain_config;
use crate::onchainos::{erc20_approve, extract_tx_hash_or_err, resolve_wallet, wallet_contract_call};
use crate::rpc::wait_for_tx;

pub async fn run(
    chain_id: u64,
    facade: &str,
    manager: &str,
    credit_account: &str,
    token: &str,
    token_addr: &str,
    amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    let user_addr = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else {
        match from {
            Some(addr) => addr.to_string(),
            None => resolve_wallet(chain_id).context("Failed to resolve wallet address")?,
        }
    };

    let decimals = infer_decimals(token);
    let amount_raw = human_to_minimal(amount, decimals);

    // Encode addCollateral inner call (target = facade)
    let add_collateral_data = encode_add_collateral(token_addr, amount_raw)
        .context("Failed to encode addCollateral calldata")?;

    let inner_calls: Vec<(&str, Vec<u8>)> = vec![(facade, add_collateral_data)];

    let multicall_data = encode_multicall(credit_account, &inner_calls)
        .context("Failed to encode multicall calldata")?;
    let multicall_hex = format!("0x{}", hex::encode(&multicall_data));

    if dry_run {
        let approve_calldata = crate::abi::encode_erc20_approve(manager, amount_raw)
            .context("Failed to encode approve calldata")?;
        let approve_hex = format!("0x{}", hex::encode(&approve_calldata));
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "chain": chain_id,
            "creditAccount": credit_account,
            "facade": facade,
            "manager": manager,
            "token": token,
            "tokenAddress": token_addr,
            "amount": amount,
            "amountRaw": amount_raw.to_string(),
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "description": format!("ERC-20 approve {} {} to CreditManagerV3 ({})", amount, token, manager),
                    "to": token_addr,
                    "inputData": approve_hex
                },
                {
                    "step": 2,
                    "action": "multicall",
                    "description": format!("addCollateral: {} {} to account {}", amount, token, credit_account),
                    "to": facade,
                    "inputData": multicall_hex
                }
            ]
        }));
    }

    // Step 1: ERC-20 approve CreditManagerV3
    let approve_result = erc20_approve(chain_id, token_addr, manager, amount_raw, Some(&user_addr), false)
        .context("ERC-20 approve failed")?;
    let approve_tx = extract_tx_hash_or_err(&approve_result)?;

    if approve_tx.starts_with("0x") && approve_tx != "0x" {
        let _ = wait_for_tx(rpc, &approve_tx).await;
    }

    // Step 2: multicall with addCollateral
    let mc_result = wallet_contract_call(
        chain_id,
        facade,
        &multicall_hex,
        Some(&user_addr),
        false,
    )
    .context("multicall (addCollateral) failed")?;

    let mc_tx = extract_tx_hash_or_err(&mc_result)?;

    Ok(json!({
        "ok": true,
        "chain": chain_id,
        "creditAccount": credit_account,
        "token": token,
        "amount": amount,
        "amountRaw": amount_raw.to_string(),
        "approveTxHash": approve_tx,
        "multicallTxHash": mc_tx
    }))
}
