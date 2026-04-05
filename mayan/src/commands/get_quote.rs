use anyhow::{anyhow, Context};
use reqwest::Client;

use crate::api::{get_quote, pick_best_route};
use crate::config::{chain_id_to_mayan_name, DEFAULT_SLIPPAGE_BPS};

pub struct GetQuoteArgs {
    pub from_chain: u64,
    pub to_chain: u64,
    pub from_token: String,
    pub to_token: String,
    pub amount: f64,
    pub slippage: Option<u32>,
}

pub async fn run(args: GetQuoteArgs) -> anyhow::Result<()> {
    let client = Client::new();

    let from_chain_name = chain_id_to_mayan_name(args.from_chain)
        .ok_or_else(|| anyhow!("Unsupported from-chain: {}", args.from_chain))?;
    let to_chain_name = chain_id_to_mayan_name(args.to_chain)
        .ok_or_else(|| anyhow!("Unsupported to-chain: {}", args.to_chain))?;

    let slippage_bps = args.slippage.unwrap_or(DEFAULT_SLIPPAGE_BPS);

    // Convert float amount to base units string.
    // The API expects integer base units; we accept floats and pass them as-is via a
    // best-effort conversion. For precise usage the caller should provide an integer
    // already scaled to the token's decimals, but for quoting purposes this works.
    let amount_in64 = float_to_base_units_str(args.amount, &args.from_token, from_chain_name);

    println!("Fetching quote:");
    println!("  {} ({}) -> {} ({})", args.from_token, from_chain_name, args.to_token, to_chain_name);
    println!("  Amount: {} (raw units: {})", args.amount, amount_in64);
    println!("  Slippage: {} bps\n", slippage_bps);

    let quotes = get_quote(
        &client,
        &amount_in64,
        &args.from_token,
        from_chain_name,
        &args.to_token,
        to_chain_name,
        slippage_bps,
        None,
        true, // full_list
    )
    .await
    .context("Failed to fetch quote")?;

    if quotes.is_empty() {
        println!("No routes available for this swap.");
        return Ok(());
    }

    println!("Available routes ({} total):", quotes.len());
    println!("{:-<70}", "");

    for (i, q) in quotes.iter().enumerate() {
        let route_type = q["type"].as_str().unwrap_or("UNKNOWN");
        let expected_out = q["expectedAmountOut"].as_str().unwrap_or("?");
        let min_received = q["minReceived"]
            .as_str()
            .or_else(|| q["minAmountOut"].as_str())
            .unwrap_or("?");
        let eta = q["etaSeconds"].as_u64().unwrap_or(0);
        let from_symbol = q["fromToken"]["symbol"].as_str().unwrap_or("?");
        let to_symbol = q["toToken"]["symbol"].as_str().unwrap_or("?");
        let swift_fee = q["swiftRelayerFee"].as_str().unwrap_or("0");
        let redeem_fee = q["redeemRelayerFee"].as_str().unwrap_or("0");
        let price = q["price"].as_f64().unwrap_or(0.0);

        let marker = if i == 0 { " [BEST]" } else { "" };
        println!("Route {}: {}{}", i + 1, route_type, marker);
        println!("  {} {} -> {} {}", args.amount, from_symbol, expected_out, to_symbol);
        println!("  Min received (after relayer fees): {}", min_received);
        println!("  Price: {:.6} {}/{}", price, to_symbol, from_symbol);
        println!("  ETA: ~{} seconds", eta);
        println!("  Relayer fees: swift={}, redeem={}", swift_fee, redeem_fee);
        println!();
    }

    if let Some(best) = pick_best_route(&quotes) {
        let route_type = best["type"].as_str().unwrap_or("UNKNOWN");
        let expected_out = best["expectedAmountOut"].as_str().unwrap_or("?");
        let to_symbol = best["toToken"]["symbol"].as_str().unwrap_or("?");
        let eta = best["etaSeconds"].as_u64().unwrap_or(0);
        println!("Recommended route: {} — receive ~{} {} in ~{}s", route_type, expected_out, to_symbol, eta);
    }

    Ok(())
}

/// Convert a float amount to a base-units string.
/// For well-known tokens we know the decimals; otherwise default to 6 for stablecoins
/// on Solana/EVM and 18 for ETH-like assets. The API also accepts decimal strings for
/// certain endpoints but requires base units for swap construction.
fn float_to_base_units_str(amount: f64, token: &str, chain: &str) -> String {
    let decimals: u32 = infer_decimals(token, chain);
    let multiplier = 10u64.pow(decimals) as f64;
    let base_units = (amount * multiplier).round() as u64;
    base_units.to_string()
}

pub fn infer_decimals(token: &str, chain: &str) -> u32 {
    // Native SOL
    if token == "11111111111111111111111111111111" || token == "So11111111111111111111111111111111111111112" {
        return 9;
    }
    // Native ETH / WETH
    if token.to_lowercase() == "0x0000000000000000000000000000000000000000"
        || token.to_lowercase() == "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"
        || token.to_lowercase() == "0x82af49447d8a07e3bd95bd0d56f35241523fbab1"
    // WETH on Arbitrum
    {
        return 18;
    }
    // USDC / USDT on most chains = 6
    if token.to_lowercase() == "epjfwdd5aufqssqem2qn1xzybapC8G4weggkzwyTDt1v".to_lowercase()
        || token.to_lowercase() == "es9vmfrzacermjfrf4h2fyd4kconky11mcce8benwnyb".to_lowercase()
        || token.to_lowercase().contains("usdc")
        || token.to_lowercase().contains("usdt")
    {
        return 6;
    }
    // Base USDC
    if token.to_lowercase() == "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" {
        return 6;
    }
    // WBTC
    if token.to_lowercase() == "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599" {
        return 8;
    }
    // EVM chains default to 18; Solana tokens commonly use 6 or 9
    if chain == "solana" { 6 } else { 18 }
}
