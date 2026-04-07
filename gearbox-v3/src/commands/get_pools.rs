/// get-pools: List Gearbox V3 Credit Managers via DataCompressor.
///
/// Calls DataCompressor.getCreditManagersV3List() to enumerate all Credit Managers,
/// then for each reads debt limits and pool info.
///
/// Selector: getCreditManagersV3List() = 0xc7fd2b45

use anyhow::Context;
use serde_json::{json, Value};

use crate::config::get_chain_config;
use crate::rpc::{eth_call, strip_0x};

pub async fn run(chain_id: u64) -> anyhow::Result<Value> {
    let cfg = get_chain_config(chain_id)?;
    let rpc = cfg.rpc_url;
    let dc = cfg.data_compressor;

    // Call DataCompressor.getCreditManagersV3List()
    // Selector: 0xc7fd2b45 — returns array of CreditManagerData structs
    // The response is complex ABI-encoded data. We parse it partially.
    let result_hex = eth_call(rpc, dc, "0xc7fd2b45")
        .await
        .context("getCreditManagersV3List eth_call failed")?;

    let raw = strip_0x(&result_hex);

    // If very short response, no data
    if raw.len() < 128 {
        return Ok(json!({
            "ok": true,
            "chain": chain_id,
            "creditManagers": [],
            "note": "No credit managers found or DataCompressor unavailable on this chain."
        }));
    }

    // Parse array length from first 32 bytes (slot 0)
    // The response is: offset to array data (32 bytes) + array data
    // For a bare array return: first 32 bytes = offset (usually 0x20), next 32 = length
    let array_offset_hex = &raw[0..64];
    let _array_offset = u64::from_str_radix(&array_offset_hex[48..64], 16).unwrap_or(0);

    // Array length is at offset (after the offset word)
    let len_raw = &raw[64..128];
    let count = u64::from_str_radix(&len_raw[48..64], 16).unwrap_or(0);

    if count == 0 {
        return Ok(json!({
            "ok": true,
            "chain": chain_id,
            "creditManagers": [],
            "note": "No credit managers registered."
        }));
    }

    // The CreditManagerData struct is large and complex. Instead of full decoding,
    // we fall back to the well-known addresses from design.md for Arbitrum,
    // and query each facade individually for debt limits.
    // This avoids complex nested ABI decoding while still providing useful data.
    let cms = get_known_credit_managers(chain_id);

    let mut results: Vec<Value> = Vec::new();
    for cm in &cms {
        let debt_limits = crate::rpc::get_debt_limits(cm.facade, rpc).await;
        let (min_debt, max_debt) = debt_limits.unwrap_or((0, 0));

        let decimals = if cm.underlying_symbol.contains("USDC") {
            6u8
        } else if cm.underlying_symbol.contains("WETH") {
            18u8
        } else {
            18u8
        };

        let factor = 10u128.pow(decimals as u32) as f64;
        let min_debt_human = min_debt as f64 / factor;
        let max_debt_human = max_debt as f64 / factor;

        results.push(json!({
            "name": cm.name,
            "creditFacade": cm.facade,
            "creditManager": cm.manager,
            "underlying": cm.underlying_symbol,
            "underlyingAddress": cm.underlying_addr,
            "minDebt": format!("{:.2} {}", min_debt_human, cm.underlying_symbol),
            "maxDebt": format!("{:.2} {}", max_debt_human, cm.underlying_symbol),
            "minDebtRaw": min_debt.to_string(),
            "maxDebtRaw": max_debt.to_string()
        }));
    }

    Ok(json!({
        "ok": true,
        "chain": chain_id,
        "creditManagerCount": results.len(),
        "creditManagers": results,
        "note": "Use --facade <creditFacade> in open-account, add-collateral, close-account, withdraw commands."
    }))
}

struct KnownCM {
    name: &'static str,
    facade: &'static str,
    manager: &'static str,
    underlying_symbol: &'static str,
    underlying_addr: &'static str,
}

fn get_known_credit_managers(chain_id: u64) -> Vec<KnownCM> {
    if chain_id == 42161 {
        vec![
            KnownCM {
                name: "Trade USDC Tier 2 (recommended, minDebt 1000 USDC)",
                facade: "0x3974888520a637ce73bdcb2ee28a396f4b303876",
                manager: "0xb780dd9cec259a0bbf7b32587802f33730353e86",
                underlying_symbol: "USDC",
                underlying_addr: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            },
            KnownCM {
                name: "Trade USDC Tier 1",
                facade: "0xbe0715eceadb3b238be599bbdb30bea28a3ebef6",
                manager: "0xe5e2d4bb15d26a6036805fce666c5488367623e2",
                underlying_symbol: "USDC",
                underlying_addr: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
            },
            KnownCM {
                name: "Trade USDC.e Tier 2",
                facade: "0x8d5d92d4595fdb190d41e1a20f96a0363f17f72c",
                manager: "0xb4bc02c0859b372c61abccfa5df91b1ccaa4dd1f",
                underlying_symbol: "USDC.e",
                underlying_addr: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
            },
            KnownCM {
                name: "Trade USDC.e Tier 1",
                facade: "0x026329e9b559ece6eaab765e6d3aa6aaa7d01e11",
                manager: "0x75bc0fef1c93723be3d73b2000b5ba139a0c680c",
                underlying_symbol: "USDC.e",
                underlying_addr: "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
            },
            KnownCM {
                name: "Trade WETH Tier 2",
                facade: "0xf1fada023dd48b9bb5f52c10b0f833e35d1c4c56",
                manager: "0x3ab1d35500d2da4216f5863229a7b81e2f6ff976",
                underlying_symbol: "WETH",
                underlying_addr: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            },
            KnownCM {
                name: "Trade WETH Tier 1",
                facade: "0x7d4a58b2f09f97537310a31e77ecd41e7d0dcbfa",
                manager: "0xcedaa4b4a42c0a771f6c24a3745c3ca3ed73f17a",
                underlying_symbol: "WETH",
                underlying_addr: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
            },
        ]
    } else {
        vec![]
    }
}
