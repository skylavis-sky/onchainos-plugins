mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "jupiter",
    about = "Jupiter DEX aggregator plugin — swap SPL tokens at best price on Solana",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote: expected output, price impact, and route plan (no on-chain action)
    GetQuote(commands::get_quote::GetQuoteArgs),

    /// Execute a token swap on Jupiter via onchainos (on-chain write)
    Swap(commands::swap::SwapArgs),

    /// Get real-time USD price for a token via Jupiter Price API
    GetPrice(commands::get_price::GetPriceArgs),

    /// Search for SPL tokens by symbol, name, or list verified tokens
    GetTokens(commands::get_tokens::GetTokensArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GetQuote(args) => commands::get_quote::execute(&args).await?,
        Commands::Swap(args) => commands::swap::execute(&args).await?,
        Commands::GetPrice(args) => commands::get_price::execute(&args).await?,
        Commands::GetTokens(args) => commands::get_tokens::execute(&args).await?,
    }

    Ok(())
}
