use anyhow::Result;
use crate::config::*;
use crate::onchainos::build_client;

pub async fn run() -> Result<()> {
    let client = build_client()?;

    // Fetch SolvBTC (Arbitrum) and xSolvBTC (Ethereum) prices in a single request
    let coin_keys = format!(
        "{},{},{}",
        DEFI_LLAMA_SOLVBTC_ARB, DEFI_LLAMA_SOLVBTC_ETH, DEFI_LLAMA_XSOLVBTC_ETH
    );
    let price_url = format!("https://coins.llama.fi/prices/current/{}", coin_keys);
    let price_resp = client
        .get(&price_url)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    let coins = &price_resp["coins"];

    let solvbtc_arb_price = coins[DEFI_LLAMA_SOLVBTC_ARB]["price"].as_f64().unwrap_or(0.0);
    let solvbtc_eth_price = coins[DEFI_LLAMA_SOLVBTC_ETH]["price"].as_f64().unwrap_or(0.0);
    let xsolvbtc_price = coins[DEFI_LLAMA_XSOLVBTC_ETH]["price"].as_f64().unwrap_or(0.0);

    // Use Arbitrum SolvBTC price as canonical; fall back to Ethereum if missing
    let solvbtc_price = if solvbtc_arb_price > 0.0 {
        solvbtc_arb_price
    } else {
        solvbtc_eth_price
    };

    // Fetch protocol TVL
    let tvl_url = format!("https://api.llama.fi/tvl/{}", DEFI_LLAMA_PROTOCOL_SLUG);
    let tvl: f64 = client
        .get(&tvl_url)
        .send()
        .await?
        .json::<f64>()
        .await
        .unwrap_or(0.0);

    println!("=== Solv Protocol NAV / Price ===");
    println!();

    if solvbtc_price > 0.0 {
        println!("SolvBTC price:  ${:.2} (Arbitrum)", solvbtc_price);
    } else {
        println!("SolvBTC price:  unavailable");
    }

    if solvbtc_eth_price > 0.0 {
        println!("SolvBTC price:  ${:.2} (Ethereum)", solvbtc_eth_price);
    }

    if xsolvbtc_price > 0.0 {
        let nav = if solvbtc_price > 0.0 {
            xsolvbtc_price / solvbtc_price
        } else {
            0.0
        };
        println!("xSolvBTC price: ${:.2} (Ethereum)", xsolvbtc_price);
        if nav > 0.0 {
            let yield_pct = (nav - 1.0) * 100.0;
            println!(
                "xSolvBTC NAV:   {:.4} BTC per xSolvBTC  (+{:.2}% over SolvBTC)",
                nav, yield_pct
            );
        }
    } else {
        println!("xSolvBTC price: unavailable");
    }

    println!();
    if tvl > 0.0 {
        println!("Solv Protocol TVL: ${:.1}M", tvl / 1_000_000.0);
    } else {
        println!("Solv Protocol TVL: unavailable");
    }

    println!();
    println!("Yield note: xSolvBTC earns variable yield via Babylon staking,");
    println!("            GMX LP, and other strategies (strategy-dependent).");

    Ok(())
}
