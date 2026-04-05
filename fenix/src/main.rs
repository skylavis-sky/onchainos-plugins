mod abi;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "fenix",
    about = "Fenix Finance plugin -- swap, add liquidity, get quotes, and query pools on Blast"
)]
struct Cli {
    /// Simulate without broadcasting on-chain transactions
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote from Fenix QuoterV2 (read-only)
    GetQuote {
        /// Input token symbol (WETH, USDB, FNX) or address
        #[arg(long)]
        token_in: String,

        /// Output token symbol or address
        #[arg(long)]
        token_out: String,

        /// Input amount in minimal units (e.g. 1000000000000000000 = 1 WETH)
        #[arg(long)]
        amount_in: u128,
    },

    /// Execute a swap via Fenix SwapRouter (approve + exactInputSingle)
    Swap {
        /// Input token symbol (WETH, USDB, FNX) or address
        #[arg(long)]
        token_in: String,

        /// Output token symbol or address
        #[arg(long)]
        token_out: String,

        /// Input amount in minimal units
        #[arg(long)]
        amount_in: u128,

        /// Slippage tolerance as a fraction (e.g. 0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,

        /// Transaction deadline offset in seconds from now (default: 300)
        #[arg(long, default_value = "300")]
        deadline_secs: u64,
    },

    /// List Fenix V3 pools sorted by TVL (GraphQL subgraph)
    GetPools {
        /// Maximum number of pools to display (default: 20)
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Add concentrated liquidity to a Fenix V3 pool (approve x2 + NFPM mint)
    AddLiquidity {
        /// First token symbol or address
        #[arg(long)]
        token0: String,

        /// Second token symbol or address
        #[arg(long)]
        token1: String,

        /// Desired amount of token0 in minimal units
        #[arg(long)]
        amount0: u128,

        /// Desired amount of token1 in minimal units
        #[arg(long)]
        amount1: u128,

        /// Lower tick of the price range
        #[arg(long, allow_hyphen_values = true)]
        tick_lower: i32,

        /// Upper tick of the price range
        #[arg(long, allow_hyphen_values = true)]
        tick_upper: i32,

        /// Transaction deadline offset in seconds from now (default: 300)
        #[arg(long, default_value = "300")]
        deadline_secs: u64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::GetQuote {
            token_in,
            token_out,
            amount_in,
        } => commands::get_quote::run(token_in, token_out, amount_in).await,

        Commands::Swap {
            token_in,
            token_out,
            amount_in,
            slippage,
            deadline_secs,
        } => {
            commands::swap::run(token_in, token_out, amount_in, slippage, deadline_secs, dry_run)
                .await
        }

        Commands::GetPools { limit } => commands::get_pools::run(limit).await,

        Commands::AddLiquidity {
            token0,
            token1,
            amount0,
            amount1,
            tick_lower,
            tick_upper,
            deadline_secs,
        } => {
            commands::add_liquidity::run(
                token0,
                token1,
                amount0,
                amount1,
                tick_lower,
                tick_upper,
                deadline_secs,
                dry_run,
            )
            .await
        }
    };

    if let Err(e) = result {
        eprintln!(
            "{}",
            serde_json::json!({ "ok": false, "error": e.to_string() })
        );
        std::process::exit(1);
    }
}
