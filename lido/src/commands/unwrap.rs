// commands/unwrap.rs — Unwrap wstETH back to stETH (Ethereum + L2s)
use anyhow::Result;
use serde_json::json;

use crate::config;
use crate::onchainos;
use crate::rpc;

pub async fn run(
    amount_wei: u128,
    from: Option<String>,
    chain_id: u64,
    dry_run: bool,
) -> Result<()> {
    // Get chain config
    let chain_cfg = config::get_chain_config(chain_id)
        .ok_or_else(|| anyhow::anyhow!("Unsupported chain ID: {}. Supported: 1, 42161, 8453, 10", chain_id))?;

    // Resolve wallet address
    let wallet = from.unwrap_or_else(|| {
        onchainos::resolve_wallet(chain_id).unwrap_or_default()
    });
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Provide --from or ensure onchainos is logged in.");
    }

    // Pre-check: wstETH balance
    let wallet_clean = wallet.trim_start_matches("0x");
    let bal_data = format!("0x70a08231{:0>64}", wallet_clean);
    let bal_hex = rpc::eth_call(chain_cfg.wsteth_address, &bal_data, chain_cfg.rpc_url).await?;
    let wsteth_balance = rpc::decode_uint256(&bal_hex);
    if wsteth_balance < amount_wei && !dry_run {
        anyhow::bail!(
            "Insufficient wstETH balance: have {} wei, need {} wei",
            wsteth_balance,
            amount_wei
        );
    }

    // Estimate stETH output (only on Ethereum where we can query getStETHByWstETH)
    let steth_out = if chain_id == 1 {
        // getStETHByWstETH(uint256) selector: 0xbb2952fc
        rpc::eth_call(
            chain_cfg.wsteth_address,
            &format!("0xbb2952fc{}", rpc::encode_uint256(amount_wei)),
            chain_cfg.rpc_url,
        )
        .await
        .map(|h| rpc::decode_uint256(&h))
        .unwrap_or(amount_wei)
    } else {
        amount_wei // approx 1:1 on L2s
    };

    // Build unwrap calldata: unwrap(uint256) selector: 0xde0e9a3e
    let calldata = format!("0xde0e9a3e{}", rpc::encode_uint256(amount_wei));

    let preview = json!({
        "operation": "unwrap",
        "chain": chain_id,
        "from": wallet,
        "wstETHAmountWei": amount_wei.to_string(),
        "wstETHFormatted": rpc::format_18dec(amount_wei),
        "expectedStETHWei": steth_out.to_string(),
        "expectedStETH": rpc::format_18dec(steth_out),
        "wstethContract": chain_cfg.wsteth_address,
        "calldata": calldata,
        "note": "Ask user to confirm before unwrapping wstETH to stETH"
    });

    if dry_run {
        println!("{}", json!({ "ok": true, "dry_run": true, "data": preview }));
        return Ok(());
    }

    // Execute: ask user to confirm unwrap transaction
    let result = onchainos::wallet_contract_call(
        chain_id,
        chain_cfg.wsteth_address,
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
                "operation": "unwrap",
                "chain": chain_id,
                "wstETHUnwrapped": rpc::format_18dec(amount_wei),
                "expectedStETH": rpc::format_18dec(steth_out)
            }
        })
    );
    Ok(())
}
