// src/main.rs — Lido plugin CLI entry point
mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lido", about = "Lido liquid staking plugin for OnchainOS")]
struct Cli {
    /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 10=Optimism)
    #[arg(long, default_value = "1")]
    chain: u64,

    /// Simulate without broadcasting (preview calldata only)
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Stake ETH to get stETH (Ethereum only)
    Stake {
        /// Amount of ETH to stake in wei (e.g. 1000000000000000000 = 1 ETH)
        #[arg(long)]
        amount: u128,
        /// Wallet address (auto-resolved from onchainos if omitted)
        #[arg(long)]
        from: Option<String>,
    },
    /// Query stETH/wstETH position and current APR
    GetPosition {
        /// Wallet address to query (auto-resolved from onchainos if omitted)
        #[arg(long)]
        from: Option<String>,
    },
    /// Query current stETH staking APR
    GetApr,
    /// Wrap stETH to wstETH (Ethereum only)
    Wrap {
        /// Amount of stETH to wrap in wei
        #[arg(long)]
        amount: u128,
        /// Wallet address
        #[arg(long)]
        from: Option<String>,
    },
    /// Unwrap wstETH to stETH (Ethereum + L2s)
    Unwrap {
        /// Amount of wstETH to unwrap in wei
        #[arg(long)]
        amount: u128,
        /// Wallet address
        #[arg(long)]
        from: Option<String>,
    },
    /// Request withdrawal of stETH to ETH (Ethereum only)
    RequestWithdrawal {
        /// Amount of stETH to withdraw in wei (max 1000 stETH per request)
        #[arg(long)]
        amount: u128,
        /// Wallet address
        #[arg(long)]
        from: Option<String>,
    },
    /// Query withdrawal request status
    GetWithdrawalStatus {
        /// Withdrawal request IDs (comma-separated or repeated --request-ids)
        #[arg(long, value_delimiter = ',')]
        request_ids: Vec<u64>,
    },
    /// Claim ETH from finalized withdrawal requests (Ethereum only)
    ClaimWithdrawal {
        /// Withdrawal request IDs to claim (comma-separated)
        #[arg(long, value_delimiter = ',')]
        request_ids: Vec<u64>,
        /// Wallet address
        #[arg(long)]
        from: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    let dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::Stake { amount, from } => {
            commands::stake::run(amount, from, dry_run).await
        }
        Commands::GetPosition { from } => {
            commands::get_position::run(from, chain_id).await
        }
        Commands::GetApr => commands::get_apr::run().await,
        Commands::Wrap { amount, from } => {
            commands::wrap::run(amount, from, dry_run).await
        }
        Commands::Unwrap { amount, from } => {
            commands::unwrap::run(amount, from, chain_id, dry_run).await
        }
        Commands::RequestWithdrawal { amount, from } => {
            commands::request_withdrawal::run(amount, from, dry_run).await
        }
        Commands::GetWithdrawalStatus { request_ids } => {
            commands::get_withdrawal_status::run(request_ids).await
        }
        Commands::ClaimWithdrawal { request_ids, from } => {
            commands::claim_withdrawal::run(request_ids, from, dry_run).await
        }
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({ "ok": false, "error": e.to_string() }));
        std::process::exit(1);
    }
}
