// commands/swap.rs — ERC-20 approve + exactInputSingle via Fenix SwapRouter
use crate::{abi, config, onchainos, rpc};
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::time::{sleep, Duration};

pub async fn run(
    token_in: String,
    token_out: String,
    amount_in: u128,
    slippage: f64,
    deadline_secs: u64,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::RPC_URL;

    let token_in_addr = config::resolve_token(&token_in);
    let token_out_addr = config::resolve_token(&token_out);

    // Step 1: Get a quote to determine amountOutMinimum
    let quote_calldata = abi::encode_quote_exact_input_single(
        &token_in_addr,
        &token_out_addr,
        amount_in,
        0,
    );
    let quote_hex = rpc::eth_call(config::QUOTER_V2, &quote_calldata, rpc_url).await?;
    let amount_out = rpc::decode_uint256_u128(&quote_hex);

    if amount_out == 0 {
        anyhow::bail!("Quote returned 0 — pool may have insufficient liquidity");
    }

    let slippage_factor = 1.0 - slippage.clamp(0.0, 1.0);
    let amount_out_minimum = (amount_out as f64 * slippage_factor) as u128;

    // Deadline = now + deadline_secs
    let deadline = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u128
        + deadline_secs as u128;

    // Step 2: Resolve wallet address (needed for recipient + allowance check)
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

    // Step 3: Check allowance and approve if needed
    if !dry_run {
        let allowance = rpc::get_allowance(
            &token_in_addr,
            &wallet_addr,
            config::SWAP_ROUTER,
            rpc_url,
        )
        .await?;

        if allowance < amount_in {
            eprintln!(
                "Approving {} for SwapRouter ({})...",
                token_in, config::SWAP_ROUTER
            );
            let approve_result = onchainos::erc20_approve(
                config::CHAIN_ID,
                &token_in_addr,
                config::SWAP_ROUTER,
                false,
            )
            .await?;
            let approve_hash = onchainos::extract_tx_hash(&approve_result);
            eprintln!("Approve tx: {}", approve_hash);
            // Wait 3 seconds for approval to confirm before swap
            sleep(Duration::from_secs(3)).await;
        }
    }

    // Step 4: Build exactInputSingle calldata
    let calldata = abi::encode_exact_input_single(
        &token_in_addr,
        &token_out_addr,
        &wallet_addr,
        deadline,
        amount_in,
        amount_out_minimum,
    );

    if dry_run {
        let decimals_in = rpc::get_decimals(&token_in_addr, rpc_url).await.unwrap_or(18);
        let decimals_out = rpc::get_decimals(&token_out_addr, rpc_url).await.unwrap_or(18);
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "chain": "blast",
                "chain_id": 81457,
                "token_in": { "symbol": token_in, "address": token_in_addr, "decimals": decimals_in },
                "token_out": { "symbol": token_out, "address": token_out_addr, "decimals": decimals_out },
                "amount_in_raw": amount_in.to_string(),
                "expected_out_raw": amount_out.to_string(),
                "amount_out_minimum_raw": amount_out_minimum.to_string(),
                "slippage_pct": slippage * 100.0,
                "deadline": deadline.to_string(),
                "swap_router": config::SWAP_ROUTER,
                "calldata": calldata
            })
        );
        return Ok(());
    }

    // Step 5: Execute swap via onchainos
    let result = onchainos::wallet_contract_call(
        config::CHAIN_ID,
        config::SWAP_ROUTER,
        &calldata,
        Some(&wallet_addr),
        true, // --force required for DEX swap
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let explorer = config::explorer_url(&tx_hash);

    let decimals_in = rpc::get_decimals(&token_in_addr, rpc_url).await.unwrap_or(18);
    let decimals_out = rpc::get_decimals(&token_out_addr, rpc_url).await.unwrap_or(18);
    let amount_in_human = amount_in as f64 / 10f64.powi(decimals_in as i32);
    let amount_out_human = amount_out as f64 / 10f64.powi(decimals_out as i32);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": "blast",
            "chain_id": 81457,
            "token_in": {
                "symbol": token_in,
                "address": token_in_addr,
                "amount_raw": amount_in.to_string(),
                "amount_human": format!("{:.6}", amount_in_human)
            },
            "token_out": {
                "symbol": token_out,
                "address": token_out_addr,
                "expected_out_raw": amount_out.to_string(),
                "expected_out_human": format!("{:.6}", amount_out_human),
                "minimum_out_raw": amount_out_minimum.to_string()
            },
            "slippage_pct": slippage * 100.0,
            "tx_hash": tx_hash,
            "explorer": explorer
        })
    );
    Ok(())
}
