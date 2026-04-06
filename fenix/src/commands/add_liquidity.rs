// commands/add_liquidity.rs — Approve tokens + NFPM mint for concentrated liquidity
use crate::{abi, config, onchainos, rpc};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

pub async fn run(
    token0: String,
    token1: String,
    amount0: u128,
    amount1: u128,
    tick_lower: i32,
    tick_upper: i32,
    deadline_secs: u64,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::RPC_URL;

    let token0_addr = config::resolve_token(&token0);
    let token1_addr = config::resolve_token(&token1);

    // Deadline = now + deadline_secs
    let deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u128
        + deadline_secs as u128;

    // Resolve wallet address
    let wallet_addr = if dry_run {
        abi::ZERO_ADDR.to_string()
    } else {
        let w = onchainos::resolve_wallet(config::CHAIN_ID)?;
        if w.is_empty() {
            anyhow::bail!(
                "Cannot determine wallet address. Ensure onchainos is logged in."
            );
        }
        w
    };

    // Build mint calldata (amount0Min=0, amount1Min=0 for simplicity)
    let calldata = abi::encode_nfpm_mint(
        &token0_addr,
        &token1_addr,
        tick_lower,
        tick_upper,
        amount0,
        amount1,
        0, // amount0Min
        0, // amount1Min
        &wallet_addr,
        deadline,
    );

    if dry_run {
        let decimals0 = rpc::get_decimals(&token0_addr, rpc_url).await.unwrap_or(18);
        let decimals1 = rpc::get_decimals(&token1_addr, rpc_url).await.unwrap_or(18);
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain": "blast",
                "chain_id": 81457,
                "token0": { "symbol": token0, "address": token0_addr, "decimals": decimals0 },
                "token1": { "symbol": token1, "address": token1_addr, "decimals": decimals1 },
                "amount0_raw": amount0.to_string(),
                "amount1_raw": amount1.to_string(),
                "tick_lower": tick_lower,
                "tick_upper": tick_upper,
                "deadline": deadline.to_string(),
                "nfpm": config::NFPM,
                "calldata": calldata
            })
        );
        return Ok(());
    }

    // Approve token0 for NFPM if needed
    let allowance0 = rpc::get_allowance(&token0_addr, &wallet_addr, config::NFPM, rpc_url).await?;
    if allowance0 < amount0 {
        eprintln!("Approving {} for NFPM ({})...", token0, config::NFPM);
        let approve_result =
            onchainos::erc20_approve(config::CHAIN_ID, &token0_addr, config::NFPM, false).await?;
        let approve_hash = onchainos::extract_tx_hash(&approve_result);
        eprintln!("Approve {} tx: {}", token0, approve_hash);
        // Wait 5 seconds after approval before next operation
        sleep(Duration::from_secs(5)).await;
    }

    // Approve token1 for NFPM if needed
    let allowance1 = rpc::get_allowance(&token1_addr, &wallet_addr, config::NFPM, rpc_url).await?;
    if allowance1 < amount1 {
        eprintln!("Approving {} for NFPM ({})...", token1, config::NFPM);
        let approve_result =
            onchainos::erc20_approve(config::CHAIN_ID, &token1_addr, config::NFPM, false).await?;
        let approve_hash = onchainos::extract_tx_hash(&approve_result);
        eprintln!("Approve {} tx: {}", token1, approve_hash);
        // Wait 5 seconds after approval before mint
        sleep(Duration::from_secs(5)).await;
    }

    // Execute NFPM mint
    let result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        config::NFPM,
        &calldata,
        Some(&wallet_addr),
        true, // --force required
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let explorer = config::explorer_url(&tx_hash);

    let decimals0 = rpc::get_decimals(&token0_addr, rpc_url).await.unwrap_or(18);
    let decimals1 = rpc::get_decimals(&token1_addr, rpc_url).await.unwrap_or(18);
    let amount0_human = amount0 as f64 / 10f64.powi(decimals0 as i32);
    let amount1_human = amount1 as f64 / 10f64.powi(decimals1 as i32);

    // Try to extract tokenId from logs (result may contain it)
    let token_id = result["data"]["tokenId"]
        .as_str()
        .or_else(|| result["tokenId"].as_str())
        .unwrap_or("see tx logs");

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": "blast",
            "chain_id": 81457,
            "token0": {
                "symbol": token0,
                "address": token0_addr,
                "amount_raw": amount0.to_string(),
                "amount_human": format!("{:.6}", amount0_human)
            },
            "token1": {
                "symbol": token1,
                "address": token1_addr,
                "amount_raw": amount1.to_string(),
                "amount_human": format!("{:.6}", amount1_human)
            },
            "tick_lower": tick_lower,
            "tick_upper": tick_upper,
            "position_nft_token_id": token_id,
            "nfpm": config::NFPM,
            "tx_hash": tx_hash,
            "explorer": explorer
        })
    );
    Ok(())
}
