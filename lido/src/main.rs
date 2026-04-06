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
    /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 10=Optimism) — can also be passed per subcommand
    #[arg(long, default_value = "1", global = true)]
    chain: u64,

    /// Simulate without broadcasting (preview calldata only) — can also be passed per subcommand
    #[arg(long, global = true)]
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
        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,
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
        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,
    },
    /// Unwrap wstETH to stETH (Ethereum + L2s)
    Unwrap {
        /// Amount of wstETH to unwrap in wei
        #[arg(long)]
        amount: u128,
        /// Wallet address
        #[arg(long)]
        from: Option<String>,
        /// Chain ID (overrides global --chain)
        #[arg(long)]
        chain: Option<u64>,
        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,
    },
    /// Request withdrawal of stETH to ETH (Ethereum only)
    RequestWithdrawal {
        /// Amount of stETH to withdraw in wei (max 1000 stETH per request)
        #[arg(long)]
        amount: u128,
        /// Wallet address
        #[arg(long)]
        from: Option<String>,
        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,
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
        /// Simulate without broadcasting (overrides global --dry-run)
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let global_chain = cli.chain;
    let global_dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::Stake { amount, from, dry_run } => {
            let dry_run = dry_run || global_dry_run;
            commands::stake::run(amount, from, dry_run).await
        }
        Commands::GetPosition { from } => {
            commands::get_position::run(from, global_chain).await
        }
        Commands::GetApr => commands::get_apr::run().await,
        Commands::Wrap { amount, from, dry_run } => {
            let dry_run = dry_run || global_dry_run;
            commands::wrap::run(amount, from, dry_run).await
        }
        Commands::Unwrap { amount, from, chain, dry_run } => {
            let chain_id = chain.unwrap_or(global_chain);
            let dry_run = dry_run || global_dry_run;
            commands::unwrap::run(amount, from, chain_id, dry_run).await
        }
        Commands::RequestWithdrawal { amount, from, dry_run } => {
            let dry_run = dry_run || global_dry_run;
            commands::request_withdrawal::run(amount, from, dry_run).await
        }
        Commands::GetWithdrawalStatus { request_ids } => {
            commands::get_withdrawal_status::run(request_ids).await
        }
        Commands::ClaimWithdrawal { request_ids, from, dry_run } => {
            let dry_run = dry_run || global_dry_run;
            commands::claim_withdrawal::run(request_ids, from, dry_run).await
        }
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({ "ok": false, "error": e.to_string() }));
        std::process::exit(1);
    }
}
