mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "dinero-pxeth",
    about = "Dinero pxETH liquid staking — deposit ETH to pxETH, stake pxETH to apxETH"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deposit ETH to receive pxETH (via PirexEth — currently paused)
    Deposit(commands::deposit::DepositArgs),
    /// Stake pxETH to receive yield-bearing apxETH (ERC-4626 deposit)
    Stake(commands::stake::StakeArgs),
    /// Redeem apxETH back to pxETH (ERC-4626 redeem)
    Redeem(commands::redeem::RedeemArgs),
    /// Get current apxETH APR and exchange rate
    Rates,
    /// Query pxETH and apxETH positions for a wallet
    Positions(commands::positions::PositionsArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Deposit(args) => commands::deposit::run(args).await,
        Commands::Stake(args) => commands::stake::run(args).await,
        Commands::Redeem(args) => commands::redeem::run(args).await,
        Commands::Rates => commands::rates::run().await,
        Commands::Positions(args) => commands::positions::run(args).await,
    }
}
