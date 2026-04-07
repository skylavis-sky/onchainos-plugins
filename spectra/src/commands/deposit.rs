use anyhow::Result;
use serde_json::Value;

use crate::config::rpc_url;
use crate::onchainos::{
    decode_uint, encode_address, encode_uint256, erc20_approve, eth_call,
    extract_tx_hash_or_err, resolve_wallet, wallet_contract_call,
};

pub async fn run(
    chain_id: u64,
    pt_address: &str,
    amount: &str,           // in wei, underlying asset amount
    use_ibt: bool,          // if true, amount is IBT units; call depositIBT instead
    receiver: Option<&str>,
    from: Option<&str>,
    slippage: f64,
    dry_run: bool,
) -> Result<Value> {
    let rpc = rpc_url(chain_id);

    // Resolve wallet
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        let w = resolve_wallet(chain_id).unwrap_or_default();
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet. Pass --from or ensure onchainos is logged in.");
        }
        w
    };
    let rcv = receiver.unwrap_or(&wallet);

    // Check maturity — deposit is blocked post-expiry
    let maturity_hex = eth_call(rpc, pt_address, "0x204f83f9").await?;
    let maturity = decode_uint(&maturity_hex) as u64;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    if maturity > 0 && now_ts >= maturity {
        anyhow::bail!(
            "PT has expired (maturity: {}). Cannot deposit into an expired PT. Use redeem instead.",
            maturity
        );
    }

    let amount_u128: u128 = amount
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?;

    // Determine which function to call
    // deposit(uint256 assets, address ptReceiver, address ytReceiver, uint256 minShares) => 0xe4cca4b0
    // depositIBT(uint256 ibts, address ptReceiver, address ytReceiver, uint256 minShares) => 0x2a412806
    let (selector, token_to_approve) = if use_ibt {
        let ibt_addr = eth_call(rpc, pt_address, "0xc644fe94")
            .await
            .map(|h| crate::onchainos::decode_address(&h))
            .unwrap_or_default();
        ("0x2a412806", ibt_addr)
    } else {
        let underlying_addr = eth_call(rpc, pt_address, "0x6f307dc3")
            .await
            .map(|h| crate::onchainos::decode_address(&h))
            .unwrap_or_default();
        ("0xe4cca4b0", underlying_addr)
    };

    // previewDeposit to estimate shares
    // previewDeposit(uint256) => 0xef8b30f7
    let preview_calldata = format!("0xef8b30f7{}", encode_uint256(amount_u128));
    let estimated_shares = eth_call(rpc, pt_address, &preview_calldata)
        .await
        .map(|h| decode_uint(&h))
        .unwrap_or(0);

    // minShares with slippage
    let min_shares = (estimated_shares as f64 * (1.0 - slippage)) as u128;

    // Build calldata: selector + assets + ptReceiver + ytReceiver + minShares
    let calldata = format!(
        "{}{}{}{}{}",
        selector,
        encode_uint256(amount_u128),
        encode_address(rcv),
        encode_address(rcv),
        encode_uint256(min_shares)
    );

    // Step 1: Approve token for PT contract
    let approve_result = erc20_approve(
        chain_id,
        &token_to_approve,
        pt_address,
        Some(&wallet),
        dry_run,
    )
    .await?;
    let approve_hash = extract_tx_hash_or_err(&approve_result)?;

    // Step 2: Deposit
    let deposit_result = wallet_contract_call(
        chain_id,
        pt_address,
        &calldata,
        Some(&wallet),
        None,
        true, // --force required for DEX operations
        dry_run,
    )
    .await?;
    let tx_hash = extract_tx_hash_or_err(&deposit_result)?;

    Ok(serde_json::json!({
        "ok": true,
        "operation": if use_ibt { "depositIBT" } else { "deposit" },
        "chain_id": chain_id,
        "pt": pt_address,
        "amount_in_raw": amount,
        "use_ibt": use_ibt,
        "token_approved": token_to_approve,
        "estimated_pt_shares": estimated_shares.to_string(),
        "min_pt_shares": min_shares.to_string(),
        "slippage": slippage,
        "receiver": rcv,
        "wallet": wallet,
        "approve_tx": approve_hash,
        "tx_hash": tx_hash,
        "calldata": calldata,
        "dry_run": dry_run
    }))
}
