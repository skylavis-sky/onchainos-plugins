// Maple Finance Plugin — CLI entry point
// Chains: Ethereum mainnet (1)
// DApp: https://maple.finance

use clap::{Parser, Subcommand};

mod commands;
mod config;
mod onchainos;
mod rpc;

#[derive(Parser)]
#[command(name = "maple", about = "Maple Finance lending protocol integration")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all Maple Finance syrup pools with TVL
    Pools {
        /// Ethereum chain ID (default: 1)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// RPC endpoint override
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Show your Maple Finance lending positions
    Positions {
        /// Wallet address (optional, resolves from onchainos if not provided)
        #[arg(long)]
        from: Option<String>,
        /// Ethereum chain ID (default: 1)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// RPC endpoint override
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Show pool exchange rates and TVL
    Rates {
        /// Ethereum chain ID (default: 1)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// RPC endpoint override
        #[arg(long)]
        rpc: Option<String>,
    },
    /// Deposit USDC or USDT into a Maple syrup pool
    Deposit {
        /// Pool name: syrupUSDC, syrupUSDT, usdc, usdt
        #[arg(long)]
        pool: String,
        /// Amount to deposit (human-readable, e.g. 0.01)
        #[arg(long)]
        amount: f64,
        /// Wallet address (optional, resolves from onchainos if not provided)
        #[arg(long)]
        from: Option<String>,
        /// Ethereum chain ID (default: 1)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// RPC endpoint override
        #[arg(long)]
        rpc: Option<String>,
        /// Simulate transaction without broadcasting
        #[arg(long)]
        dry_run: bool,
    },
    /// Request withdrawal (requestRedeem) from a Maple syrup pool
    Withdraw {
        /// Pool name: syrupUSDC, syrupUSDT, usdc, usdt
        #[arg(long)]
        pool: String,
        /// Number of shares to redeem (omit to redeem all shares)
        #[arg(long)]
        shares: Option<f64>,
        /// Wallet address (optional, resolves from onchainos if not provided)
        #[arg(long)]
        from: Option<String>,
        /// Ethereum chain ID (default: 1)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// RPC endpoint override
        #[arg(long)]
        rpc: Option<String>,
        /// Simulate transaction without broadcasting
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = run(cli).await;
    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({ "ok": false, "error": e.to_string() }));
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::Pools { chain: _, rpc } => {
            let rpc_url = rpc.as_deref().unwrap_or(config::RPC_URL);
            commands::pools::run(rpc_url).await
        }
        Commands::Positions { from, chain: _, rpc } => {
            let rpc_url = rpc.as_deref().unwrap_or(config::RPC_URL);
            commands::positions::run(rpc_url, from).await
        }
        Commands::Rates { chain: _, rpc } => {
            let rpc_url = rpc.as_deref().unwrap_or(config::RPC_URL);
            commands::rates::run(rpc_url).await
        }
        Commands::Deposit {
            pool,
            amount,
            from,
            chain: _,
            rpc,
            dry_run,
        } => {
            let rpc_url = rpc.as_deref().unwrap_or(config::RPC_URL);
            commands::deposit::run(&pool, amount, rpc_url, from, dry_run).await
        }
        Commands::Withdraw {
            pool,
            shares,
            from,
            chain: _,
            rpc,
            dry_run,
        } => {
            let rpc_url = rpc.as_deref().unwrap_or(config::RPC_URL);
            commands::withdraw::run(&pool, shares, rpc_url, from, dry_run).await
        }
    }
}
