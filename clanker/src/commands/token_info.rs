// src/commands/token_info.rs — query on-chain token info + price for a Clanker token
use crate::onchainos;
use anyhow::Result;

pub fn run(chain_id: u64, token_address: &str) -> Result<()> {
    let info = onchainos::token_info(chain_id, token_address)?;
    let price = onchainos::token_price_info(chain_id, token_address)?;

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "token_address": token_address,
            "chain_id": chain_id,
            "info": info["data"],
            "price": price["data"],
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
