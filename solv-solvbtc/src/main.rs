mod abi;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};
use config::CHAIN_ARBITRUM;

#[derive(Parser)]
#[command(
    name = "solv-solvbtc",
    version = "0.1.0",
    about = "Solv Protocol SolvBTC - mint yield-bearing BTC on Arbitrum and Ethereum"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit WBTC to receive SolvBTC (Arbitrum or Ethereum)
    Mint {
        /// WBTC amount in human-readable form (e.g. 0.001)
        #[arg(long)]
        amount: f64,

        /// Chain ID: 42161 (Arbitrum, default) or 1 (Ethereum)
        #[arg(long, default_value_t = CHAIN_ARBITRUM)]
        chain: u64,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Submit a SolvBTC redemption request (non-instant, returns SFT ticket)
    Redeem {
        /// SolvBTC amount in human-readable form (e.g. 0.001)
        #[arg(long)]
        amount: f64,

        /// Chain ID: 42161 (Arbitrum, default) or 1 (Ethereum)
        #[arg(long, default_value_t = CHAIN_ARBITRUM)]
        chain: u64,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Cancel a pending SolvBTC redemption request
    CancelRedeem {
        /// Redemption contract address (from the SFT ticket)
        #[arg(long)]
        redemption_addr: String,

        /// Redemption token ID
        #[arg(long)]
        redemption_id: u128,

        /// Chain ID: 42161 (Arbitrum, default) or 1 (Ethereum)
        #[arg(long, default_value_t = CHAIN_ARBITRUM)]
        chain: u64,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Wrap SolvBTC into yield-bearing xSolvBTC (Ethereum mainnet only)
    Wrap {
        /// SolvBTC amount in human-readable form (e.g. 0.05)
        #[arg(long)]
        amount: f64,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Unwrap xSolvBTC back to SolvBTC (Ethereum mainnet only, 0.05% fee)
    Unwrap {
        /// xSolvBTC amount in human-readable form (e.g. 0.05)
        #[arg(long)]
        amount: f64,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Fetch current SolvBTC and xSolvBTC price / NAV from DeFiLlama
    GetNav,

    /// Query SolvBTC and xSolvBTC token balances for your wallet
    GetBalance {
        /// Chain ID: 42161 (Arbitrum, default) or 1 (Ethereum)
        #[arg(long, default_value_t = CHAIN_ARBITRUM)]
        chain: u64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Mint {
            amount,
            chain,
            dry_run,
        } => commands::mint::run(amount, chain, dry_run).await,

        Commands::Redeem {
            amount,
            chain,
            dry_run,
        } => commands::redeem::run(amount, chain, dry_run).await,

        Commands::CancelRedeem {
            redemption_addr,
            redemption_id,
            chain,
            dry_run,
        } => commands::redeem::cancel(&redemption_addr, redemption_id, chain, dry_run).await,

        Commands::Wrap { amount, dry_run } => {
            commands::wrap::run(amount, dry_run).await
        }

        Commands::Unwrap { amount, dry_run } => {
            commands::unwrap::run(amount, dry_run).await
        }

        Commands::GetNav => commands::get_nav::run().await,

        Commands::GetBalance { chain } => commands::get_balance::run(chain).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
