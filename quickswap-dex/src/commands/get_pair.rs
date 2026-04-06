use crate::config::{resolve_token_address, FACTORY_V2, POLYGON_RPC, CHAIN_ID};
use crate::rpc::factory_get_pair;

/// Get the pair contract address for two tokens from QuickSwap V2 Factory.
pub async fn run(token_a: &str, token_b: &str) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = POLYGON_RPC;

    let addr_a = resolve_token_address(token_a, chain_id);
    let addr_b = resolve_token_address(token_b, chain_id);

    let pair = factory_get_pair(FACTORY_V2, &addr_a, &addr_b, rpc).await?;

    if pair == "0x0000000000000000000000000000000000000000" {
        println!("No pair found for {} / {}", token_a.to_uppercase(), token_b.to_uppercase());
    } else {
        println!("QuickSwap V2 Pair");
        println!("  tokenA: {} ({})", token_a.to_uppercase(), addr_a);
        println!("  tokenB: {} ({})", token_b.to_uppercase(), addr_b);
        println!("  pair:   {}", pair);
    }

    Ok(())
}
