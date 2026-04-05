// commands/get_position.rs — Query Lido stETH/wstETH balances and rate
use anyhow::Result;
use serde_json::json;

use crate::config;
use crate::onchainos;
use crate::rpc;

pub async fn run(wallet: Option<String>, chain_id: u64) -> Result<()> {
    // Resolve wallet address
    let address = match wallet {
        Some(addr) if !addr.is_empty() => addr,
        _ => {
            let addr = onchainos::resolve_wallet(config::CHAIN_ETHEREUM)?;
            if addr.is_empty() {
                anyhow::bail!("Cannot resolve wallet address. Provide --from or ensure onchainos is logged in.");
            }
            addr
        }
    };

    // Build position data
    let mut position = json!({
        "wallet": address,
        "stETH": null,
        "wstETH": {},
        "rate": null,
        "totalPooledEther": null,
        "apr": null
    });

    // 1. stETH balance on Ethereum
    let steth_balance = get_erc20_balance(
        config::STETH_ADDRESS,
        &address,
        config::RPC_ETHEREUM,
    ).await?;
    position["stETH"] = json!({
        "wei": steth_balance.to_string(),
        "formatted": rpc::format_18dec(steth_balance),
        "chain": "Ethereum",
        "chainId": 1
    });

    // 2. wstETH per-chain balances
    let chains: &[(u64, &str, &str, &str)] = &[
        (1, "Ethereum", config::WSTETH_ETH_ADDRESS, config::RPC_ETHEREUM),
        (42161, "Arbitrum", "0x5979D7b546E38E414F7E9822514be443A4800529", config::RPC_ARBITRUM),
        (8453, "Base", "0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452", config::RPC_BASE),
        (10, "Optimism", "0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb", config::RPC_OPTIMISM),
    ];

    // If a specific chain_id is provided (not 0 or 1), only query that chain's wstETH
    let query_chains: Vec<_> = if chain_id == 0 {
        chains.iter().collect()
    } else {
        chains.iter().filter(|(cid, _, _, _)| *cid == chain_id).collect()
    };

    let mut wsteth_balances = serde_json::Map::new();
    for (cid, name, addr, rpc) in &query_chains {
        match get_erc20_balance(addr, &address, rpc).await {
            Ok(bal) => {
                wsteth_balances.insert(
                    name.to_string(),
                    json!({
                        "wei": bal.to_string(),
                        "formatted": rpc::format_18dec(bal),
                        "chain": name,
                        "chainId": cid,
                        "contract": addr
                    }),
                );
            }
            Err(e) => {
                wsteth_balances.insert(
                    name.to_string(),
                    json!({ "error": e.to_string(), "chainId": cid }),
                );
            }
        }
    }
    position["wstETH"] = serde_json::Value::Object(wsteth_balances);

    // 3. Exchange rate: stETH per wstETH (from Ethereum wstETH)
    match get_steth_per_token(config::WSTETH_ETH_ADDRESS, config::RPC_ETHEREUM).await {
        Ok(rate) => {
            position["rate"] = json!({
                "stEthPerWstEth_wei": rate.to_string(),
                "stEthPerWstEth": rpc::format_18dec(rate),
                "description": "1 wstETH = N stETH"
            });
        }
        Err(e) => {
            position["rate"] = json!({ "error": e.to_string() });
        }
    }

    // 4. Total pooled ether (protocol TVL)
    match get_total_pooled_ether(config::STETH_ADDRESS, config::RPC_ETHEREUM).await {
        Ok(tvl) => {
            position["totalPooledEther"] = json!({
                "wei": tvl.to_string(),
                "formatted": rpc::format_18dec(tvl),
                "description": "Total ETH staked in Lido protocol"
            });
        }
        Err(e) => {
            position["totalPooledEther"] = json!({ "error": e.to_string() });
        }
    }

    // 5. Current APR
    match crate::api::get_apr_sma().await {
        Ok(apr) => {
            position["apr"] = json!({
                "smaApr": apr,
                "description": "7-day SMA APR"
            });
        }
        Err(e) => {
            position["apr"] = json!({ "error": e.to_string() });
        }
    }

    println!("{}", json!({ "ok": true, "data": position }));
    Ok(())
}

async fn get_erc20_balance(token: &str, wallet: &str, rpc_url: &str) -> Result<u128> {
    // balanceOf(address) selector: 0x70a08231
    let wallet_clean = wallet.trim_start_matches("0x");
    let data = format!("0x70a08231{:0>64}", wallet_clean);
    let result = rpc::eth_call(token, &data, rpc_url).await?;
    Ok(rpc::decode_uint256(&result))
}

async fn get_steth_per_token(wsteth: &str, rpc_url: &str) -> Result<u128> {
    // stEthPerToken() selector: 0x035faf82
    let result = rpc::eth_call(wsteth, "0x035faf82", rpc_url).await?;
    Ok(rpc::decode_uint256(&result))
}

async fn get_total_pooled_ether(steth: &str, rpc_url: &str) -> Result<u128> {
    // getTotalPooledEther() selector: 0x37cfdaca
    let result = rpc::eth_call(steth, "0x37cfdaca", rpc_url).await?;
    Ok(rpc::decode_uint256(&result))
}
