/// open-account: Open a Gearbox V3 Credit Account with leveraged position.
///
/// Flow:
///   1. Resolve wallet address
///   2. Check debtLimits() to validate borrow amount
///   3. ERC-20 approve(CreditManagerV3, collateral_amount)
///   4. openCreditAccount(onBehalfOf, [increaseDebt(borrow_amt), addCollateral(token, collateral_amt)], 0)
///
/// ⚠️  IMPORTANT:
///   - Approve must go to CreditManagerV3 (NOT CreditFacadeV3) — Gearbox gotcha G2
///   - increaseDebt must come BEFORE addCollateral in the multicall array
///   - Borrow amount must be >= minDebt from debtLimits()
///   - Only underlying token as collateral is supported in v0.1 (no quota management)

use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::{
    encode_add_collateral, encode_increase_debt, encode_open_credit_account,
    human_to_minimal, infer_decimals,
};
use crate::config::{get_chain_config, REFERRAL_CODE};
use crate::onchainos::{erc20_approve, extract_tx_hash_or_err, resolve_wallet, wallet_contract_call};
use crate::rpc::{get_debt_limits, wait_for_tx};

pub async fn run(
    chain_id: u64,
    facade: &str,
    manager: &str,
    token: &str,
    token_addr: &str,
    collateral_amount: f64,
    borrow_amount: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;

    // Resolve wallet address AFTER dry-run early-return to avoid wallet resolution errors
    // when running dry-run without an active wallet session.
    let user_addr = if dry_run {
        from.unwrap_or("0x0000000000000000000000000000000000000000").to_string()
    } else {
        match from {
            Some(addr) => addr.to_string(),
            None => resolve_wallet(chain_id).context("Failed to resolve wallet address")?,
        }
    };

    let decimals = infer_decimals(token);
    let collateral_raw = human_to_minimal(collateral_amount, decimals);
    let borrow_raw = human_to_minimal(borrow_amount, decimals);

    // Validate borrow amount against debtLimits
    if !dry_run {
        let (min_debt, max_debt) = get_debt_limits(facade, rpc)
            .await
            .context("Failed to fetch debt limits from CreditFacadeV3")?;

        if borrow_raw < min_debt {
            let factor = 10u128.pow(decimals as u32) as f64;
            return Err(anyhow::anyhow!(
                "Borrow amount {:.2} {} is below minimum debt {:.2} {} for this Credit Manager. \
                 Increase your borrow amount or choose a different Credit Manager.",
                borrow_amount,
                token,
                min_debt as f64 / factor,
                token
            ));
        }
        if borrow_raw > max_debt {
            let factor = 10u128.pow(decimals as u32) as f64;
            return Err(anyhow::anyhow!(
                "Borrow amount {:.2} {} exceeds maximum debt {:.2} {}.",
                borrow_amount,
                token,
                max_debt as f64 / factor,
                token
            ));
        }
    }

    // Build inner calls: increaseDebt must come BEFORE addCollateral (Gearbox requirement)
    let increase_debt_calldata = encode_increase_debt(borrow_raw)
        .context("Failed to encode increaseDebt calldata")?;
    let add_collateral_calldata = encode_add_collateral(token_addr, collateral_raw)
        .context("Failed to encode addCollateral calldata")?;

    // Inner calls target the CreditFacadeV3 address
    let inner_calls: Vec<(&str, Vec<u8>)> = vec![
        (facade, increase_debt_calldata),
        (facade, add_collateral_calldata),
    ];

    // Encode the outer openCreditAccount call
    let open_calldata = encode_open_credit_account(&user_addr, &inner_calls, REFERRAL_CODE)
        .context("Failed to encode openCreditAccount calldata")?;
    let open_calldata_hex = format!("0x{}", hex::encode(&open_calldata));

    if dry_run {
        let approve_calldata = crate::abi::encode_erc20_approve(manager, collateral_raw)
            .context("Failed to encode approve calldata")?;
        let approve_hex = format!("0x{}", hex::encode(&approve_calldata));
        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "chain": chain_id,
            "facade": facade,
            "manager": manager,
            "token": token,
            "tokenAddress": token_addr,
            "collateralAmount": collateral_amount,
            "collateralRaw": collateral_raw.to_string(),
            "borrowAmount": borrow_amount,
            "borrowRaw": borrow_raw.to_string(),
            "totalPosition": collateral_amount + borrow_amount,
            "steps": [
                {
                    "step": 1,
                    "action": "approve",
                    "description": format!("ERC-20 approve {} {} to CreditManagerV3 ({})", collateral_amount, token, manager),
                    "to": token_addr,
                    "inputData": approve_hex
                },
                {
                    "step": 2,
                    "action": "openCreditAccount",
                    "description": format!("Open Credit Account: borrow {} {}, deposit {} {} collateral", borrow_amount, token, collateral_amount, token),
                    "to": facade,
                    "inputData": open_calldata_hex
                }
            ]
        }));
    }

    // Step 1: ERC-20 approve CreditManagerV3 (NOT facade)
    let approve_result = erc20_approve(chain_id, token_addr, manager, collateral_raw, Some(&user_addr), false)
        .context("ERC-20 approve failed")?;
    let approve_tx = extract_tx_hash_or_err(&approve_result)?;

    // Wait for approve to confirm before opening account
    if approve_tx.starts_with("0x") && approve_tx != "0x" {
        let _ = wait_for_tx(rpc, &approve_tx).await;
    }

    // Step 2: openCreditAccount
    let open_result = wallet_contract_call(
        chain_id,
        facade,
        &open_calldata_hex,
        Some(&user_addr),
        false,
    )
    .context("openCreditAccount failed")?;

    let open_tx = extract_tx_hash_or_err(&open_result)?;

    Ok(json!({
        "ok": true,
        "chain": chain_id,
        "facade": facade,
        "manager": manager,
        "token": token,
        "tokenAddress": token_addr,
        "collateralAmount": collateral_amount,
        "borrowAmount": borrow_amount,
        "totalPosition": collateral_amount + borrow_amount,
        "leverage": (collateral_amount + borrow_amount) / collateral_amount,
        "approveTxHash": approve_tx,
        "openAccountTxHash": open_tx,
        "warning": "Monitor your health factor closely. Positions can be liquidated by third parties when HF < 1.0.",
        "note": "Your Credit Account address can be found via get-account command once the transaction confirms."
    }))
}
