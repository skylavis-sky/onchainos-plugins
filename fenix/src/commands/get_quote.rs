// commands/get_quote.rs — Query swap quote via QuoterV2 eth_call
use crate::{abi, config, rpc};
use anyhow::Result;

pub async fn run(
    token_in: String,
    token_out: String,
    amount_in: u128,
) -> Result<()> {
    let rpc_url = config::RPC_URL;

    let token_in_addr = config::resolve_token(&token_in);
    let token_out_addr = config::resolve_token(&token_out);

    // Verify pool exists via Factory poolByPair
    let pool_calldata = abi::encode_pool_by_pair(&token_in_addr, &token_out_addr);
    let pool_hex = rpc::eth_call(config::FACTORY, &pool_calldata, rpc_url).await?;
    let pool_addr = rpc::decode_address(&pool_hex);

    if pool_addr == abi::ZERO_ADDR || pool_addr.to_lowercase() == "0x0000000000000000000000000000000000000000" {
        // Try reversed token order
        let pool_calldata_rev = abi::encode_pool_by_pair(&token_out_addr, &token_in_addr);
        let pool_hex_rev = rpc::eth_call(config::FACTORY, &pool_calldata_rev, rpc_url).await?;
        let pool_addr_rev = rpc::decode_address(&pool_hex_rev);
        if pool_addr_rev == abi::ZERO_ADDR || pool_addr_rev.to_lowercase() == "0x0000000000000000000000000000000000000000" {
            anyhow::bail!(
                "No Fenix pool found for pair {}/{} on Blast",
                token_in, token_out
            );
        }
    }

    // Call QuoterV2 quoteExactInputSingle
    let quote_calldata = abi::encode_quote_exact_input_single(
        &token_in_addr,
        &token_out_addr,
        amount_in,
        0, // limitSqrtPrice = 0
    );

    let result_hex = rpc::eth_call(config::QUOTER_V2, &quote_calldata, rpc_url).await?;

    // QuoterV2 returns (uint256 amountOut, uint16 fee, uint160 sqrtPriceX96After, uint32 initializedTicksCrossed, uint256 gasEstimate)
    // The first 32 bytes (first word) is amountOut
    let amount_out = rpc::decode_uint256_u128(&result_hex);

    if amount_out == 0 {
        anyhow::bail!(
            "Quote returned 0 — pool may have insufficient liquidity for this amount"
        );
    }

    // Get decimals for display
    let decimals_in = rpc::get_decimals(&token_in_addr, rpc_url).await.unwrap_or(18);
    let decimals_out = rpc::get_decimals(&token_out_addr, rpc_url).await.unwrap_or(18);

    let amount_in_human = amount_in as f64 / 10f64.powi(decimals_in as i32);
    let amount_out_human = amount_out as f64 / 10f64.powi(decimals_out as i32);

    // Simple price impact estimate (not exact — would require current price from pool)
    // For display we just report the rate
    let rate = if amount_in_human > 0.0 {
        amount_out_human / amount_in_human
    } else {
        0.0
    };

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": "blast",
            "chain_id": 81457,
            "token_in": {
                "symbol": token_in,
                "address": token_in_addr,
                "decimals": decimals_in,
                "amount_raw": amount_in.to_string(),
                "amount_human": format!("{:.6}", amount_in_human)
            },
            "token_out": {
                "symbol": token_out,
                "address": token_out_addr,
                "decimals": decimals_out,
                "amount_raw": amount_out.to_string(),
                "amount_human": format!("{:.6}", amount_out_human)
            },
            "rate": format!("{:.6}", rate),
            "quoter": config::QUOTER_V2
        })
    );
    Ok(())
}
