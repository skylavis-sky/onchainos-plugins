mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "marinade",
    about = "Marinade Finance liquid staking — stake SOL to receive mSOL on Solana"
)]
struct Cli {
    /// Simulate without broadcasting to chain
    #[arg(long)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Query mSOL/SOL exchange rate and staking APY
    Rates,

    /// Query your mSOL holdings and SOL-equivalent value
    Positions,

    /// Stake SOL to receive mSOL (SOL → mSOL via Jupiter)
    Stake {
        /// Amount of SOL to stake (e.g. "0.001")
        #[arg(long)]
        amount: String,

        /// Slippage tolerance in percent [default: 1.0]
        #[arg(long, default_value = "1.0")]
        slippage: f64,
    },

    /// Unstake mSOL to receive SOL (mSOL → SOL via Jupiter)
    Unstake {
        /// Amount of mSOL to unstake (e.g. "0.001")
        #[arg(long)]
        amount: String,

        /// Slippage tolerance in percent [default: 1.0]
        #[arg(long, default_value = "1.0")]
        slippage: f64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Rates => commands::rates::execute().await,
        Commands::Positions => commands::positions::execute().await,
        Commands::Stake { amount, slippage } => {
            commands::stake::execute(&amount, slippage, cli.dry_run).await
        }
        Commands::Unstake { amount, slippage } => {
            commands::unstake::execute(&amount, slippage, cli.dry_run).await
        }
    };

    if let Err(e) = result {
        let err = serde_json::json!({
            "ok": false,
            "error": e.to_string()
        });
        eprintln!("{}", serde_json::to_string_pretty(&err).unwrap());
        std::process::exit(1);
    }
}
