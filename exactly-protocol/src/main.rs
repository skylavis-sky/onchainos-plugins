mod commands;
mod config;
mod onchainos;
mod previewer;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "exactly-protocol",
    about = "Fixed-rate and floating-rate lending on Exactly Protocol (Optimism, Ethereum)",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Chain ID: 10 = Optimism (default), 1 = Ethereum Mainnet
    #[arg(long, global = true, default_value = "10")]
    chain: u64,

    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,

    /// Simulate without broadcasting transactions
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all Exactly Protocol markets with rates and liquidity
    GetMarkets {},

    /// Show your current positions (deposits and borrows) across all markets
    GetPosition {},

    /// Deposit/lend assets into a market (floating or fixed-rate)
    Deposit {
        /// Market symbol: WETH, USDC, OP, wstETH, WBTC
        #[arg(long)]
        market: String,

        /// Human-readable amount (e.g. 1000.0 for 1000 USDC)
        #[arg(long)]
        amount: f64,

        /// Maturity timestamp for fixed-rate deposit (Unix seconds), or omit for floating-rate
        #[arg(long)]
        maturity: Option<u64>,
    },

    /// Borrow assets from a market (floating or fixed-rate)
    Borrow {
        /// Market symbol: WETH, USDC, OP, wstETH, WBTC
        #[arg(long)]
        market: String,

        /// Human-readable amount to borrow
        #[arg(long)]
        amount: f64,

        /// Maturity timestamp for fixed-rate borrow (Unix seconds), or omit for floating-rate
        #[arg(long)]
        maturity: Option<u64>,
    },

    /// Repay a borrow position (floating or fixed-rate)
    Repay {
        /// Market symbol: WETH, USDC, OP, wstETH, WBTC
        #[arg(long)]
        market: String,

        /// Human-readable amount to repay (use positionAssets from get-position for fixed)
        #[arg(long)]
        amount: f64,

        /// Maturity timestamp for fixed-rate repay (Unix seconds), or omit for floating-rate (refund)
        #[arg(long)]
        maturity: Option<u64>,

        /// Borrow shares for floating-rate repay (from get-position floatingBorrowShares)
        #[arg(long)]
        borrow_shares: Option<u128>,
    },

    /// Withdraw deposited assets from a market (floating or fixed-rate)
    Withdraw {
        /// Market symbol: WETH, USDC, OP, wstETH, WBTC
        #[arg(long)]
        market: String,

        /// Human-readable amount to withdraw (omit if using --all)
        #[arg(long)]
        amount: Option<f64>,

        /// Maturity timestamp for fixed-rate withdrawal (Unix seconds), or omit for floating-rate
        #[arg(long)]
        maturity: Option<u64>,

        /// Withdraw entire floating-rate position
        #[arg(long, default_value = "false")]
        all: bool,
    },

    /// Enable an asset as collateral (Auditor.enterMarket)
    EnterMarket {
        /// Market symbol or address to enable as collateral
        #[arg(long)]
        market: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::GetMarkets {} => {
            commands::get_markets::run(cli.chain).await
        }
        Commands::GetPosition {} => {
            commands::get_position::run(cli.chain, cli.from.as_deref()).await
        }
        Commands::Deposit { market, amount, maturity } => {
            commands::deposit::run(
                cli.chain,
                &market,
                amount,
                maturity,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
        Commands::Borrow { market, amount, maturity } => {
            commands::borrow::run(
                cli.chain,
                &market,
                amount,
                maturity,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
        Commands::Repay { market, amount, maturity, borrow_shares } => {
            commands::repay::run(
                cli.chain,
                &market,
                amount,
                maturity,
                borrow_shares,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
        Commands::Withdraw { market, amount, maturity, all } => {
            commands::withdraw::run(
                cli.chain,
                &market,
                amount,
                maturity,
                all,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
        Commands::EnterMarket { market } => {
            commands::enter_market::run(
                cli.chain,
                &market,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
    };

    match result {
        Ok(val) => {
            println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
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
