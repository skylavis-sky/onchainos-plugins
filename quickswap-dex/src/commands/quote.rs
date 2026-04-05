use crate::config::{resolve_token_address, ROUTER_V2, WMATIC, POLYGON_RPC, CHAIN_ID};
use crate::rpc::get_amounts_out;

/// Quote: get expected output amount for a swap via getAmountsOut.
/// Uses WMATIC as intermediate hop for token→token pairs for best liquidity.
pub async fn run(token_in: &str, token_out: &str, amount_in: u128) -> anyhow::Result<()> {
    let chain_id = CHAIN_ID;
    let rpc = POLYGON_RPC;

    let addr_in = resolve_token_address(token_in, chain_id);
    let addr_out = resolve_token_address(token_out, chain_id);

    // Build path: direct if one of them is WMATIC, otherwise route via WMATIC
    let wmatic = WMATIC.to_lowercase();
    let ai = addr_in.to_lowercase();
    let ao = addr_out.to_lowercase();

    let (amounts, path_desc) = if ai == wmatic || ao == wmatic {
        // Direct path (one of them is WMATIC)
        let path = vec![addr_in.as_str(), addr_out.as_str()];
        let path_desc = format!("{} → {}", token_in.to_uppercase(), token_out.to_uppercase());
        let amounts = get_amounts_out(ROUTER_V2, amount_in, &path, rpc).await?;
        (amounts, path_desc)
    } else {
        // Route through WMATIC for better liquidity
        let path = vec![addr_in.as_str(), WMATIC, addr_out.as_str()];
        let path_desc = format!("{} → WMATIC → {}", token_in.to_uppercase(), token_out.to_uppercase());
        let amounts = get_amounts_out(ROUTER_V2, amount_in, &path, rpc).await?;
        (amounts, path_desc)
    };

    if amounts.is_empty() {
        anyhow::bail!("getAmountsOut returned empty array — pool may not exist");
    }

    let amount_out = *amounts.last().unwrap();

    println!("QuickSwap V2 Quote");
    println!("  Path:       {}", path_desc);
    println!("  Amount in:  {} (raw units)", amount_in);
    println!("  Amount out: {} (raw units)", amount_out);
    println!("  Slippage (0.5%): {} minimum out", amount_out * 995 / 1000);

    Ok(())
}
