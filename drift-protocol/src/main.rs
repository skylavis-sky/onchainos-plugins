mod api;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "drift-protocol",
    version = "0.1.0",
    about = "Drift Protocol — Perpetual futures DEX and lending on Solana (read-only; writes paused pending protocol relaunch)"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check Solana wallet SOL and USDC/USDT balances
    GetBalance {
        /// Chain ID (default: 501 for Solana mainnet)
        #[arg(long, default_value = "501")]
        chain: u64,
    },

    /// Fetch L2 orderbook for a Drift perpetual market (currently 503 — protocol paused)
    GetMarkets {
        /// Market name, e.g. SOL-PERP, BTC-PERP, ETH-PERP
        #[arg(long, default_value = "SOL-PERP")]
        market: String,

        /// Number of orderbook levels to fetch
        #[arg(long, default_value = "10")]
        depth: u32,
    },

    /// Fetch funding rates for a Drift perpetual market (currently unavailable — protocol paused)
    GetFundingRates {
        /// Market name, e.g. SOL-PERP
        #[arg(long, default_value = "SOL-PERP")]
        market: String,
    },

    /// [PAUSED] Place an order on Drift — blocked pending protocol relaunch
    PlaceOrder {
        /// Market name (e.g. SOL-PERP)
        #[arg(long)]
        market: String,

        /// Order side: buy or sell
        #[arg(long)]
        side: String,

        /// Order size in base asset units
        #[arg(long)]
        size: f64,

        /// Limit price (omit for market order)
        #[arg(long)]
        price: Option<f64>,

        /// Simulate without broadcasting (no effect — command is fully paused)
        #[arg(long)]
        dry_run: bool,
    },

    /// [PAUSED] Deposit assets into Drift — blocked pending protocol relaunch
    Deposit {
        /// Token symbol (e.g. USDT, SOL)
        #[arg(long)]
        token: String,

        /// Amount to deposit
        #[arg(long)]
        amount: f64,

        /// Simulate without broadcasting (no effect — command is fully paused)
        #[arg(long)]
        dry_run: bool,
    },

    /// [PAUSED] Cancel an open order on Drift — blocked pending protocol relaunch
    CancelOrder {
        /// Order ID to cancel
        #[arg(long)]
        order_id: Option<String>,

        /// Simulate without broadcasting (no effect — command is fully paused)
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::GetBalance { chain: _ } => cmd_get_balance(),
        Commands::GetMarkets { market, depth } => cmd_get_markets(&market, depth),
        Commands::GetFundingRates { market } => cmd_get_funding_rates(&market),
        Commands::PlaceOrder { market: _, side: _, size: _, price: _, dry_run: _ } => {
            api::write_paused_error()
        }
        Commands::Deposit { token: _, amount: _, dry_run: _ } => {
            api::write_paused_error()
        }
        Commands::CancelOrder { order_id: _, dry_run: _ } => {
            api::write_paused_error()
        }
    };

    let output = serde_json::to_string_pretty(&result)
        .unwrap_or_else(|e| format!("{{\"ok\":false,\"error\":\"serialization failed: {}\"}}", e));

    if result["ok"].as_bool().unwrap_or(true) {
        println!("{}", output);
    } else {
        eprintln!("{}", output);
        std::process::exit(1);
    }
}

/// get-balance: call onchainos wallet balance --chain 501 and extract SOL + USDC/USDT
fn cmd_get_balance() -> serde_json::Value {
    match onchainos::resolve_wallet_solana() {
        Err(e) => serde_json::json!({
            "ok": false,
            "error": format!("Failed to query wallet balance: {}", e)
        }),
        Ok(balance_json) => {
            let wallet = onchainos::extract_wallet_address(&balance_json)
                .unwrap_or_else(|| "unknown".to_string());

            // Extract SOL balance
            let sol_amount = onchainos::find_token_asset(&balance_json, "sol")
                .and_then(|a| a["balance"].as_str().map(|s| s.to_string()))
                .or_else(|| {
                    onchainos::find_token_asset(&balance_json, "sol")
                        .and_then(|a| a["amount"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_else(|| "0".to_string());

            // Extract USDC balance (legacy Drift settlement token)
            let usdc_amount = onchainos::find_token_asset(&balance_json, "usdc")
                .and_then(|a| a["balance"].as_str().map(|s| s.to_string()))
                .or_else(|| {
                    onchainos::find_token_asset(&balance_json, "usdc")
                        .and_then(|a| a["amount"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_else(|| "0".to_string());

            // Extract USDT balance (new Drift settlement token post-relaunch)
            let usdt_amount = onchainos::find_token_asset(&balance_json, "usdt")
                .and_then(|a| a["balance"].as_str().map(|s| s.to_string()))
                .or_else(|| {
                    onchainos::find_token_asset(&balance_json, "usdt")
                        .and_then(|a| a["amount"].as_str().map(|s| s.to_string()))
                })
                .unwrap_or_else(|| "0".to_string());

            serde_json::json!({
                "ok": true,
                "wallet": wallet,
                "sol": sol_amount,
                "usdc": usdc_amount,
                "usdt": usdt_amount,
                "chain": "solana",
                "note": "Drift deposits are currently paused (protocol recovery mode). These are your on-chain Solana wallet balances."
            })
        }
    }
}

/// get-markets: fetch L2 orderbook from dlob.drift.trade
fn cmd_get_markets(market: &str, depth: u32) -> serde_json::Value {
    let result = api::get_l2_orderbook(market, depth);
    if result["ok"].as_bool().unwrap_or(false) {
        // Reshape the response for clarity
        let data = &result["data"];
        serde_json::json!({
            "ok": true,
            "market": market,
            "depth": depth,
            "bids": data["bids"],
            "asks": data["asks"],
            "slot": data["slot"]
        })
    } else {
        result
    }
}

/// get-funding-rates: fetch from data.api.drift.trade
fn cmd_get_funding_rates(market: &str) -> serde_json::Value {
    let result = api::get_funding_rates(market);
    if result["ok"].as_bool().unwrap_or(false) {
        let data = &result["data"];
        serde_json::json!({
            "ok": true,
            "market": market,
            "data": data
        })
    } else {
        result
    }
}
