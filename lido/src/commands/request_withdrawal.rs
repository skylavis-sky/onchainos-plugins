// commands/request_withdrawal.rs — Request ETH withdrawal from Lido (stETH → ETH)
// Only supported on Ethereum mainnet.
use anyhow::Result;
use serde_json::json;

use crate::config;
use crate::onchainos;
use crate::rpc;

pub async fn run(
    amount_wei: u128,
    from: Option<String>,
    dry_run: bool,
) -> Result<()> {
    // Validate amount
    if amount_wei < config::MIN_WITHDRAWAL_AMOUNT {
        anyhow::bail!(
            "Amount {} wei is below minimum withdrawal amount of {} wei (100 gwei)",
            amount_wei,
            config::MIN_WITHDRAWAL_AMOUNT
        );
    }

    // Resolve wallet
    let wallet = from.unwrap_or_else(|| {
        onchainos::resolve_wallet(config::CHAIN_ETHEREUM).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Provide --from or ensure onchainos is logged in.");
    }

    // Pre-check: stETH balance
    let wallet_clean = wallet.trim_start_matches("0x");
    let bal_data = format!("0x70a08231{:0>64}", wallet_clean);
    let bal_hex = rpc::eth_call(config::STETH_ADDRESS, &bal_data, config::RPC_ETHEREUM).await?;
    let steth_balance = rpc::decode_uint256(&bal_hex);
    if steth_balance < amount_wei && !dry_run {
        anyhow::bail!(
            "Insufficient stETH: have {} wei, need {} wei",
            steth_balance,
            amount_wei
        );
    }

    // Check if we need to split into multiple requests (max 1000 stETH per request)
    let max_per_req = config::MAX_WITHDRAWAL_PER_REQUEST;
    let mut amounts: Vec<u128> = Vec::new();
    let mut remaining = amount_wei;
    while remaining > 0 {
        let chunk = remaining.min(max_per_req);
        amounts.push(chunk);
        remaining -= chunk;
    }

    // Get estimated wait time
    let wait_time_info = crate::api::get_request_time(&[]).await.unwrap_or(json!({}));

    // Check allowance for WithdrawalQueue
    let allowance = onchainos::erc20_allowance(
        config::CHAIN_ETHEREUM,
        config::STETH_ADDRESS,
        &wallet,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        config::RPC_ETHEREUM,
    )
    .await
    .unwrap_or(0);

    // Build requestWithdrawals(uint256[],address) calldata
    // selector: 0xd6681042
    // ABI encoding: dynamic array + address (owner)
    let calldata = build_request_withdrawals_calldata(&amounts, &wallet);

    let preview = json!({
        "operation": "request-withdrawal",
        "from": wallet,
        "totalAmountWei": amount_wei.to_string(),
        "totalAmountFormatted": rpc::format_18dec(amount_wei),
        "requests": amounts.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
        "numRequests": amounts.len(),
        "needsApprove": allowance < amount_wei,
        "estimatedWait": wait_time_info,
        "calldata": calldata,
        "note": "Ask user to confirm before submitting withdrawal request(s). Withdrawal typically takes 1-5 days.",
        "warning": "Withdrawal is a 2-step process: request now, then claim when finalized"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Step 1: approve stETH to WithdrawalQueue if needed — ask user to confirm
    if allowance < amount_wei {
        let approve_result = onchainos::erc20_approve(
            config::CHAIN_ETHEREUM,
            config::STETH_ADDRESS,
            config::WITHDRAWAL_QUEUE_ADDRESS,
            amount_wei,
            Some(&wallet),
            false,
        )
        .await?;
        let approve_tx = onchainos::extract_tx_hash(&approve_result);
        eprintln!("Approve txHash: {}", approve_tx);
    }

    // Step 2: submit withdrawal request(s) — ask user to confirm
    let result = onchainos::wallet_contract_call(
        config::CHAIN_ETHEREUM,
        config::WITHDRAWAL_QUEUE_ADDRESS,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    println!(
        "{}",
        json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "operation": "request-withdrawal",
                "amountRequested": rpc::format_18dec(amount_wei),
                "numRequests": amounts.len(),
                "message": "Withdrawal request submitted as NFT. Track with get-withdrawal-status --request-ids <id>. Claim when isFinalized=true.",
                "estimatedWait": "Typically 1-5 days"
            }
        })
    );
    Ok(())
}

/// Build ABI calldata for requestWithdrawals(uint256[] _amounts, address _owner)
/// selector: 0xd6681042
fn build_request_withdrawals_calldata(amounts: &[u128], owner: &str) -> String {
    // ABI encoding for (uint256[], address):
    // slot 0: offset to array data = 0x40 (64 bytes)
    // slot 1: owner address (padded)
    // slot 2: array length
    // slot 3+: array elements
    let owner_clean = owner.trim_start_matches("0x");
    let owner_padded = format!("{:0>64}", owner_clean);

    // offset to array: 2 * 32 = 64 = 0x40
    let array_offset = format!("{:064x}", 64u64);
    let array_len = format!("{:064x}", amounts.len());
    let elements: String = amounts
        .iter()
        .map(|a| format!("{:064x}", a))
        .collect();

    format!(
        "0xd6681042{}{}{}{}",
        array_offset, owner_padded, array_len, elements
    )
}
