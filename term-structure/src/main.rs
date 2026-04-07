mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "term-structure",
    about = "Lend and borrow at fixed rates on TermMax V2 (Term Structure) — fixed-rate AMM on Arbitrum, Ethereum, BNB",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Chain ID (default: 42161 Arbitrum One — primary TermMax chain)
    #[arg(long, global = true, default_value = "42161")]
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
    /// List active TermMax markets with current lend/borrow APR
    GetMarkets {
        /// Filter by underlying token symbol (e.g. USDC, WETH)
        #[arg(long)]
        underlying: Option<String>,
    },

    /// View your lend (FT) and borrow (GT) positions across markets
    GetPosition {},

    /// Lend tokens at a fixed rate (buy FT bond tokens)
    Lend {
        /// Market contract address
        #[arg(long)]
        market: String,
        /// Human-readable amount to lend (e.g. 1000.0)
        #[arg(long)]
        amount: f64,
        /// Token symbol to lend (e.g. USDC, WETH)
        #[arg(long, default_value = "USDC")]
        token: String,
    },

    /// Borrow tokens by posting collateral (receive GT NFT with loanId)
    Borrow {
        /// Market contract address
        #[arg(long)]
        market: String,
        /// Human-readable collateral amount to post (e.g. 1.0)
        #[arg(long)]
        collateral_amount: f64,
        /// Collateral token symbol (e.g. WETH, WBTC, wstETH, ARB)
        #[arg(long)]
        collateral_token: String,
        /// Human-readable amount to borrow (e.g. 500.0)
        #[arg(long)]
        borrow_amount: f64,
    },

    /// Repay a borrow position by loan ID (GT NFT)
    Repay {
        /// GT NFT loan ID (from get-position output)
        #[arg(long)]
        loan_id: u64,
        /// Market contract address
        #[arg(long)]
        market: String,
        /// Maximum repayment amount (underlying token, human-readable)
        #[arg(long, default_value = "10000.0")]
        max_amount: f64,
        /// Underlying token symbol (e.g. USDC)
        #[arg(long, default_value = "USDC")]
        token: String,
    },

    /// Redeem FT tokens after market maturity for underlying + interest
    Redeem {
        /// Market contract address
        #[arg(long)]
        market: String,
        /// Human-readable amount of FT to redeem (omit to use --all)
        #[arg(long)]
        amount: Option<f64>,
        /// Redeem all FT tokens in wallet
        #[arg(long, default_value = "false")]
        all: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::GetMarkets { underlying } => {
            commands::get_markets::run(cli.chain, underlying.as_deref()).await
        }

        Commands::GetPosition {} => {
            commands::get_position::run(cli.chain, cli.from.as_deref()).await
        }

        Commands::Lend {
            market,
            amount,
            token,
        } => {
            commands::lend::run(
                cli.chain,
                &market,
                amount,
                &token,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::Borrow {
            market,
            collateral_amount,
            collateral_token,
            borrow_amount,
        } => {
            commands::borrow::run(
                cli.chain,
                &market,
                collateral_amount,
                &collateral_token,
                borrow_amount,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::Repay {
            loan_id,
            market,
            max_amount,
            token,
        } => {
            commands::repay::run(
                cli.chain,
                &market,
                loan_id,
                max_amount,
                &token,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::Redeem { market, amount, all } => {
            commands::redeem::run(
                cli.chain,
                &market,
                amount,
                all,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
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
