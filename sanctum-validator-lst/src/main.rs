mod api;
mod config;
mod instructions;
mod onchainos;
mod rpc;
mod commands;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "sanctum-validator-lst",
    about = "Stake SOL into validator LSTs and swap between LSTs via Sanctum Router on Solana"
)]
struct Cli {
    /// Preview operation without broadcasting (can be placed before or after subcommand)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List tracked validator LSTs with APY, TVL, and SOL value
    ListLsts(commands::list_lsts::ListLstsArgs),
    /// Get a quote to swap between two LSTs via Sanctum Router
    GetQuote(commands::get_quote::GetQuoteArgs),
    /// Swap between two validator LSTs via Sanctum Router
    SwapLst(commands::swap_lst::SwapLstArgs),
    /// Stake SOL into a specific validator LST pool (SPL DepositSol)
    Stake(commands::stake::StakeArgs),
    /// Show your validator LST holdings and SOL equivalent value
    GetPosition(commands::get_position::GetPositionArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let global_dry_run = cli.dry_run;
    let result = match cli.command {
        Commands::ListLsts(args) => commands::list_lsts::run(args).await,
        Commands::GetQuote(args) => commands::get_quote::run(args).await,
        Commands::SwapLst(mut args) => { args.dry_run = global_dry_run || args.dry_run; commands::swap_lst::run(args).await },
        Commands::Stake(mut args) => { args.dry_run = global_dry_run || args.dry_run; commands::stake::run(args).await },
        Commands::GetPosition(args) => commands::get_position::run(args).await,
    };
    match result {
        Ok(val) => println!("{}", serde_json::to_string_pretty(&val).unwrap()),
        Err(e) => {
            let err = serde_json::json!({"ok": false, "error": e.to_string()});
            println!("{}", serde_json::to_string_pretty(&err).unwrap());
            std::process::exit(1);
        }
    }
}
