use anyhow::Result;
use crate::abi::{encode_balance_of, decode_uint256_from_hex, raw_to_solvbtc};
use crate::config::*;
use crate::onchainos::{resolve_wallet, build_client};

/// Query SolvBTC and (if Ethereum) xSolvBTC balances via eth_call JSON-RPC.
pub async fn run(chain_id: u64) -> Result<()> {
    let wallet = resolve_wallet(chain_id)?;
    println!("Wallet:  {}", wallet);
    println!("Chain:   {}", chain_id);
    println!();

    let (solvbtc_addr, _wbtc_addr, _router) = chain_contracts(chain_id)?;

    // eth_call to get balanceOf
    let solvbtc_raw = eth_call_balance_of(chain_id, solvbtc_addr, &wallet).await?;
    println!("SolvBTC balance: {} SolvBTC", raw_to_solvbtc(solvbtc_raw));

    // xSolvBTC only on Ethereum
    if chain_id == CHAIN_ETHEREUM {
        let xsolvbtc_raw =
            eth_call_balance_of(chain_id, ETH_XSOLVBTC_TOKEN, &wallet).await?;
        println!("xSolvBTC balance: {} xSolvBTC", raw_to_solvbtc(xsolvbtc_raw));
    }

    Ok(())
}

async fn eth_call_balance_of(chain_id: u64, token: &str, wallet: &str) -> Result<u128> {
    let rpc_url = match chain_id {
        CHAIN_ARBITRUM => "https://arb1.arbitrum.io/rpc",
        CHAIN_ETHEREUM => "https://ethereum.publicnode.com",
        other => anyhow::bail!("No RPC configured for chain {}", other),
    };

    let calldata = encode_balance_of(wallet);

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "eth_call",
        "params": [
            { "to": token, "data": calldata },
            "latest"
        ]
    });

    let client = build_client()?;
    let resp = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let hex_result = resp["result"]
        .as_str()
        .unwrap_or("0x0");

    decode_uint256_from_hex(hex_result)
}
