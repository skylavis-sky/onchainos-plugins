// commands/wrap.rs — Wrap stETH to wstETH
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

    // Pre-check: stETH balance
    let wallet_clean = wallet.trim_start_matches("0x");
    let bal_data = format!("0x70a08231{:0>64}", wallet_clean);
    let bal_hex = rpc::eth_call(config::STETH_ADDRESS, &bal_data, config::RPC_ETHEREUM).await?;
    let steth_balance = rpc::decode_uint256(&bal_hex);
    if steth_balance < amount_wei && !dry_run {
        anyhow::bail!(
            "Insufficient stETH balance: have {} wei, need {} wei",
            steth_balance,
            amount_wei
        );
    }

    // Check allowance for wstETH contract
    let allowance = onchainos::erc20_allowance(
        config::CHAIN_ETHEREUM,
        config::STETH_ADDRESS,
        &wallet,
        config::WSTETH_ETH_ADDRESS,
        config::RPC_ETHEREUM,
    )
    .await
    .unwrap_or(0);

    // Estimate wstETH output via getWstETHByStETH(amount)
    // selector: 0xb0e38900
    let wsteth_out_hex = rpc::eth_call(
        config::WSTETH_ETH_ADDRESS,
        &format!("0xb0e38900{}", rpc::encode_uint256(amount_wei)),
        config::RPC_ETHEREUM,
    )
    .await
    .unwrap_or_else(|_| "0x".to_string());
    let wsteth_out = rpc::decode_uint256(&wsteth_out_hex);

    // Build wrap calldata: wrap(uint256) selector: 0xea598cb0
    let wrap_calldata = format!("0xea598cb0{}", rpc::encode_uint256(amount_wei));

    let preview = json!({
        "operation": "wrap",
        "from": wallet,
        "stETHAmountWei": amount_wei.to_string(),
        "stETHFormatted": rpc::format_18dec(amount_wei),
        "expectedWstETHWei": wsteth_out.to_string(),
        "expectedWstETH": rpc::format_18dec(wsteth_out),
        "needsApprove": allowance < amount_wei,
        "calldata": wrap_calldata,
        "note": "Ask user to confirm before wrapping stETH to wstETH"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Step 1: approve if needed — ask user to confirm approve
    if allowance < amount_wei {
        let approve_result = onchainos::erc20_approve(
            config::CHAIN_ETHEREUM,
            config::STETH_ADDRESS,
            config::WSTETH_ETH_ADDRESS,
            amount_wei,
            Some(&wallet),
            false,
        )
        .await?;
        let approve_tx = onchainos::extract_tx_hash(&approve_result);
        eprintln!("Approve txHash: {}", approve_tx);
    }

    // Step 2: wrap — ask user to confirm wrap transaction
    let result = onchainos::wallet_contract_call(
        config::CHAIN_ETHEREUM,
        config::WSTETH_ETH_ADDRESS,
        &wrap_calldata,
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
                "operation": "wrap",
                "stETHWrapped": rpc::format_18dec(amount_wei),
                "expectedWstETH": rpc::format_18dec(wsteth_out)
            }
        })
    );
    Ok(())
}
