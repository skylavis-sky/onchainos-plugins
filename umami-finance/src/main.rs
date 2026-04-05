mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "umami-finance", about = "Umami Finance GM Vault plugin for onchainos")]
struct Cli {
    /// Chain ID (default: 42161 Arbitrum)
    #[arg(long, default_value = "42161")]
    chain: u64,

    /// Dry-run mode: build calldata but don't broadcast
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all Umami GM vaults with TVL and price per share
    ListVaults,

    /// Show detailed info for a specific vault
    VaultInfo {
        /// Vault name (e.g. gmUSDC-eth, gmWETH) or vault contract address
        #[arg(long)]
        vault: String,
    },

    /// Show your positions across all Umami vaults
    Positions {
        /// Wallet address (optional, resolved from onchainos if omitted)
        #[arg(long)]
        from: Option<String>,
    },

    /// Deposit assets into a Umami GM vault
    Deposit {
        /// Vault name or address (e.g. gmUSDC-eth, gmWETH)
        #[arg(long)]
        vault: String,
        /// Amount to deposit in human-readable units (e.g. 10.0 for 10 USDC)
        #[arg(long)]
        amount: f64,
        /// Sender wallet address (optional)
        #[arg(long)]
        from: Option<String>,
    },

    /// Redeem shares from a Umami GM vault
    Redeem {
        /// Vault name or address (e.g. gmUSDC-eth, gmWETH)
        #[arg(long)]
        vault: String,
        /// Number of shares to redeem (optional, defaults to all)
        #[arg(long)]
        shares: Option<f64>,
        /// Wallet address (optional)
        #[arg(long)]
        from: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::ListVaults => commands::list_vaults::execute(cli.chain).await,
        Commands::VaultInfo { vault } => commands::vault_info::execute(vault, cli.chain).await,
        Commands::Positions { from } => {
            commands::positions::execute(cli.chain, from.as_deref()).await
        }
        Commands::Deposit { vault, amount, from } => {
            commands::deposit::execute(vault, *amount, cli.chain, from.as_deref(), cli.dry_run).await
        }
        Commands::Redeem { vault, shares, from } => {
            commands::redeem::execute(vault, *shares, cli.chain, from.as_deref(), cli.dry_run).await
        }
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({"ok": false, "error": e.to_string()}));
        std::process::exit(1);
    }
}
