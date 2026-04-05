use clap::{Parser, Subcommand};

mod api;
mod commands;
mod config;
mod onchainos;

#[derive(Parser)]
#[command(
    name = "mayan",
    about = "Mayan cross-chain swap plugin — Swift/MCTP routes between Solana, Ethereum, Arbitrum, Base, Optimism, Polygon",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a cross-chain swap quote (all available routes)
    GetQuote {
        /// Source chain ID (e.g. 501=Solana, 1=Ethereum, 42161=Arbitrum, 8453=Base)
        #[arg(long)]
        from_chain: u64,
        /// Destination chain ID
        #[arg(long)]
        to_chain: u64,
        /// Source token address (use 11111111111111111111111111111111 for native SOL,
        /// 0x0000000000000000000000000000000000000000 for native ETH)
        #[arg(long)]
        from_token: String,
        /// Destination token address
        #[arg(long)]
        to_token: String,
        /// Amount to swap (human-readable float, e.g. 100.0 for 100 USDC)
        #[arg(long)]
        amount: f64,
        /// Slippage in basis points (default: 100 = 1%)
        #[arg(long)]
        slippage: Option<u32>,
    },

    /// Execute a cross-chain swap via Mayan
    Swap {
        /// Source chain ID
        #[arg(long)]
        from_chain: u64,
        /// Destination chain ID
        #[arg(long)]
        to_chain: u64,
        /// Source token address
        #[arg(long)]
        from_token: String,
        /// Destination token address
        #[arg(long)]
        to_token: String,
        /// Amount to swap (human-readable float)
        #[arg(long)]
        amount: f64,
        /// Slippage in basis points (default: 100 = 1%)
        #[arg(long)]
        slippage: Option<u32>,
        /// Dry run — build calldata but do not broadcast
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Check the status of a cross-chain swap by transaction hash
    GetStatus {
        /// Source chain transaction hash (EVM tx hash or Solana signature)
        #[arg(long)]
        tx_hash: String,
        /// Source chain ID (optional, for context)
        #[arg(long)]
        chain: Option<u64>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::GetQuote {
            from_chain,
            to_chain,
            from_token,
            to_token,
            amount,
            slippage,
        } => {
            commands::get_quote::run(commands::get_quote::GetQuoteArgs {
                from_chain,
                to_chain,
                from_token,
                to_token,
                amount,
                slippage,
            })
            .await
        }

        Commands::Swap {
            from_chain,
            to_chain,
            from_token,
            to_token,
            amount,
            slippage,
            dry_run,
        } => {
            commands::swap::run(commands::swap::SwapArgs {
                from_chain,
                to_chain,
                from_token,
                to_token,
                amount,
                slippage,
                dry_run,
            })
            .await
        }

        Commands::GetStatus { tx_hash, chain } => {
            commands::get_status::run(commands::get_status::GetStatusArgs {
                tx_hash,
                chain,
            })
            .await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
