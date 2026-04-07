mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "vertex-edge",
    about = "Vertex Edge perpetual DEX - query markets, positions, and deposit collateral on Arbitrum",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Chain ID (default: 42161 Arbitrum)
    #[arg(long, global = true, default_value = "42161")]
    chain: u64,

    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,

    /// Simulate without broadcasting on-chain transactions
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all Vertex Edge markets with oracle prices and open interest
    GetMarkets {},

    /// View perp positions, spot balances, and margin health for a subaccount
    GetPositions {
        /// Wallet address to query (defaults to active onchainos wallet)
        #[arg(long)]
        address: Option<String>,
    },

    /// Query orderbook depth for a market
    GetOrderbook {
        /// Market symbol (e.g. BTC-PERP, ETH-PERP) -- use get-markets to list valid symbols
        #[arg(long)]
        market: Option<String>,

        /// Product ID (numeric, alternative to --market)
        #[arg(long)]
        product_id: Option<u32>,

        /// Number of levels to return (default: 10)
        #[arg(long)]
        depth: Option<u32>,
    },

    /// Query current mark prices and index prices for perp markets
    GetPrices {
        /// Comma-separated product IDs to query (e.g. 2,4,6 for BTC/ETH/ARB perps)
        /// Defaults to top 10 perp markets
        #[arg(long, value_delimiter = ',')]
        product_ids: Option<Vec<u32>>,
    },

    /// Deposit USDC collateral into Vertex Edge (on-chain tx: approve + depositCollateral)
    /// NOTE: You will be asked to confirm two transactions. Place/cancel orders require
    /// EIP-712 signing via the Vertex web UI (not supported in v0.1 of this plugin).
    Deposit {
        /// Amount of USDC to deposit (human-readable, e.g. 100.0 for 100 USDC)
        #[arg(long)]
        amount: f64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::GetMarkets {} => commands::get_markets::run(cli.chain).await,

        Commands::GetPositions { address } => {
            commands::get_positions::run(
                cli.chain,
                address.as_deref().or(cli.from.as_deref()),
            )
            .await
        }

        Commands::GetOrderbook {
            market,
            product_id,
            depth,
        } => {
            commands::get_orderbook::run(
                cli.chain,
                market.as_deref(),
                product_id,
                depth,
            )
            .await
        }

        Commands::GetPrices { product_ids } => {
            commands::get_prices::run(cli.chain, product_ids).await
        }

        Commands::Deposit { amount } => {
            commands::deposit::run(cli.chain, amount, cli.from.as_deref(), cli.dry_run).await
        }
    };

    match result {
        Ok(val) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&val).unwrap_or_default()
            );
        }
        Err(err) => {
            let error_json = serde_json::json!({
                "ok": false,
                "error": err.to_string()
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&error_json).unwrap_or_default()
            );
            std::process::exit(1);
        }
    }
}
