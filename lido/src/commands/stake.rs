// commands/stake.rs — Stake ETH to get stETH via Lido submit()
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
    // Resolve wallet address
    let wallet = from.unwrap_or_else(|| {
        onchainos::resolve_wallet(config::CHAIN_ETHEREUM).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Provide --from or ensure onchainos is logged in.");
    }

    // Pre-check 1: getCurrentStakeLimit() to ensure staking is not paused
    // selector: 0x609c4c6c
    let stake_limit_hex = rpc::eth_call(
        config::STETH_ADDRESS,
        "0x609c4c6c",
        config::RPC_ETHEREUM,
    )
    .await?;
    let stake_limit = rpc::decode_uint256(&stake_limit_hex);
    if stake_limit == 0 && !dry_run {
        anyhow::bail!("Lido staking is currently paused (stake limit = 0).");
    }
    if stake_limit > 0 && amount_wei > stake_limit {
        anyhow::bail!(
            "Amount {} wei exceeds current stake limit {} wei.",
            amount_wei,
            stake_limit
        );
    }

    // Pre-check 2: Get current APR for display
    let apr = crate::api::get_apr_sma().await.unwrap_or(0.0);

    // Pre-check 3: Estimate stETH to receive via getSharesByPooledEth
    // selector: 0x19208451
    let shares_hex = rpc::eth_call(
        config::STETH_ADDRESS,
        &format!("0x19208451{}", rpc::encode_uint256(amount_wei)),
        config::RPC_ETHEREUM,
    )
    .await
    .unwrap_or_else(|_| "0x".to_string());
    let _shares = rpc::decode_uint256(&shares_hex);

    // Build calldata: submit(address _referral) with zero address referral
    // selector: 0xa1903eab
    let calldata = "0xa1903eab0000000000000000000000000000000000000000000000000000000000000000";

    let preview = json!({
        "operation": "stake",
        "from": wallet,
        "ethAmountWei": amount_wei.to_string(),
        "ethFormatted": rpc::format_18dec(amount_wei),
        "expectedStETH": rpc::format_18dec(amount_wei), // stETH is ~1:1 with ETH
        "currentApr": format!("{}%", apr),
        "protocolFee": "10% of staking rewards",
        "contract": config::STETH_ADDRESS,
        "calldata": calldata,
        "note": "Ask user to confirm before submitting the stake transaction"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Execute: onchainos wallet contract-call with ETH value attached
    // Ask user to confirm is embedded in the agent flow per SKILL.md
    let result = onchainos::wallet_contract_call(
        config::CHAIN_ETHEREUM,
        config::STETH_ADDRESS,
        calldata,
        Some(&wallet),
        Some(amount_wei),
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;
    println!(
        "{}",
        json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "operation": "stake",
                "ethStaked": rpc::format_18dec(amount_wei),
                "expectedStETH": rpc::format_18dec(amount_wei),
                "apr": format!("{}%", apr)
            }
        })
    );
    Ok(())
}
