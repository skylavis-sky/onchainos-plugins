mod abi;
mod commands;
mod config;
mod create2;
mod onchainos;

use clap::{Parser, Subcommand};

use commands::{
    buy::BuyArgs, create_token::CreateTokenArgs, get_token_info::GetTokenInfoArgs, sell::SellArgs,
};

#[derive(Parser, Debug)]
#[command(
    name = "flap",
    about = "Plugin for Flap — create and trade tokens on BSC bonding curves (flap.sh)",
    version = "0.1.0"
)]
struct Cli {
    /// Simulate without broadcasting (no on-chain transaction sent)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Query bonding curve state, price, and status for a Flap token
    GetTokenInfo(GetTokenInfoArgs),

    /// Create a new standard or tax token on Flap bonding curve
    CreateToken(CreateTokenArgs),

    /// Buy tokens from a Flap bonding curve with BNB
    Buy(BuyArgs),

    /// Sell tokens back to a Flap bonding curve for BNB (requires ERC-20 approve)
    Sell(SellArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match &cli.command {
        Commands::GetTokenInfo(args) => commands::get_token_info::execute(args).await,
        Commands::CreateToken(args) => commands::create_token::execute(args, cli.dry_run).await,
        Commands::Buy(args) => commands::buy::execute(args, cli.dry_run).await,
        Commands::Sell(args) => commands::sell::execute(args, cli.dry_run).await,
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({"ok": false, "error": e.to_string()}));
        std::process::exit(1);
    }
}
