/// deposit: Deposit USDC collateral into Vertex Edge via on-chain transaction.
///
/// This is a two-step on-chain operation:
///   1. ERC-20 approve(endpoint_contract, amount) on the USDC token
///   2. Endpoint.depositCollateral(bytes12 subaccount_name, uint32 product_id, uint128 amount)
///      selector: 0x8e5d588c
///
/// NOTE: Vertex order operations (place-order, cancel-order) require EIP-712 signing
/// which is not supported in v0.1. Use the Vertex web UI for order operations.
/// This command handles collateral deposits only.
///
/// The subaccount_name arg is bytes12: "default" right-padded to 12 bytes.
/// product_id for USDC spot = 0.
/// amount is USDC amount scaled by 10^6 (USDC has 6 decimals).

use anyhow::Context;
use serde_json::{json, Value};

use crate::config::{get_chain_config, DEFAULT_SUBACCOUNT_NAME, USDC_PRODUCT_ID};
use crate::onchainos::{erc20_approve, extract_tx_hash_or_err, resolve_wallet, wallet_contract_call};

/// Encode depositCollateral(bytes12,uint32,uint128) calldata.
/// selector: 0x8e5d588c
/// Args:
///   subaccount_name: bytes12 (right-padded "default" with null bytes), ABI-padded to 32 bytes (left-aligned in first slot)
///   product_id: uint32, ABI-padded to 32 bytes
///   amount: uint128, ABI-padded to 32 bytes (USDC scaled by 10^6)
fn encode_deposit_collateral(subaccount_name: &str, product_id: u32, amount: u128) -> String {
    // Encode subaccount name as bytes12: UTF-8 right-padded with null bytes to 12 bytes
    // ABI encoding for bytes12: left-aligned in a 32-byte slot (padded right with zeros)
    let name_bytes = subaccount_name.as_bytes();
    let mut name_padded = [0u8; 12];
    let copy_len = name_bytes.len().min(12);
    name_padded[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

    // ABI bytes12 is left-aligned in 32 bytes (padded right with zeros)
    let mut slot0 = [0u8; 32];
    slot0[..12].copy_from_slice(&name_padded);
    let slot0_hex = hex::encode(slot0);

    // uint32 product_id: right-aligned in 32 bytes
    let slot1_hex = format!("{:064x}", product_id as u64);

    // uint128 amount: right-aligned in 32 bytes
    let slot2_hex = format!("{:064x}", amount);

    format!("0x8e5d588c{}{}{}", slot0_hex, slot1_hex, slot2_hex)
}

pub async fn run(
    chain_id: u64,
    amount_usdc: f64,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;

    if amount_usdc <= 0.0 {
        anyhow::bail!("Amount must be positive. Got: {}", amount_usdc);
    }

    // Resolve wallet address
    let wallet_addr = match from {
        Some(addr) => addr.to_string(),
        None => {
            if dry_run {
                "0x0000000000000000000000000000000000000000".to_string()
            } else {
                resolve_wallet(chain_id).context("Failed to resolve wallet address")?
            }
        }
    };

    // USDC has 6 decimals
    let amount_raw = (amount_usdc * 1_000_000.0) as u128;

    // Step 1: ERC-20 approve(endpoint_contract, amount)
    // Ask user to confirm before broadcasting the approval
    eprintln!(
        "Step 1/2: Approving USDC transfer to Vertex Endpoint ({}) for {} USDC ({} units)...",
        cfg.endpoint_contract, amount_usdc, amount_raw
    );
    eprintln!("Please confirm this transaction in your wallet.");

    let approve_result = erc20_approve(
        chain_id,
        cfg.usdc_address,
        cfg.endpoint_contract,
        amount_raw,
        Some(&wallet_addr),
        dry_run,
    )
    .context("ERC-20 approve failed")?;

    let approve_tx = if dry_run {
        "dry-run-approve-tx".to_string()
    } else {
        extract_tx_hash_or_err(&approve_result)
            .context("Failed to extract approve txHash")?
    };

    eprintln!("Approve tx: {}", approve_tx);

    // Step 2: depositCollateral(bytes12 subaccount_name, uint32 product_id, uint128 amount)
    let calldata = encode_deposit_collateral(DEFAULT_SUBACCOUNT_NAME, USDC_PRODUCT_ID, amount_raw);

    eprintln!(
        "Step 2/2: Depositing {} USDC ({} units) to Vertex Endpoint...",
        amount_usdc, amount_raw
    );
    eprintln!("Please confirm this transaction in your wallet.");

    let deposit_result = wallet_contract_call(
        chain_id,
        cfg.endpoint_contract,
        &calldata,
        Some(&wallet_addr),
        dry_run,
    )
    .context("depositCollateral contract call failed")?;

    let deposit_tx = if dry_run {
        "dry-run-deposit-tx".to_string()
    } else {
        extract_tx_hash_or_err(&deposit_result)
            .context("Failed to extract deposit txHash")?
    };

    Ok(json!({
        "ok": true,
        "chain": cfg.name,
        "chain_id": chain_id,
        "address": wallet_addr,
        "subaccount_name": DEFAULT_SUBACCOUNT_NAME,
        "product_id": USDC_PRODUCT_ID,
        "amount_usdc": amount_usdc,
        "amount_raw": amount_raw.to_string(),
        "approve_txHash": approve_tx,
        "deposit_txHash": deposit_tx,
        "note": "Collateral is now deposited into your Vertex default subaccount. Use the Vertex web UI or API to place/cancel orders (requires EIP-712 signing)."
    }))
}
