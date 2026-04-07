// src/main.rs — Loopscale plugin CLI entry point
mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "loopscale",
    about = "Loopscale Solana lending plugin — lend, borrow, repay, and manage positions"
)]
struct Cli {
    /// Preview operation without broadcasting (no on-chain effect)
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List available Loopscale lending vaults with APY and TVL
    GetVaults {
        /// Filter by token: USDC, SOL, or mint address (default: all)
        #[arg(long)]
        token: Option<String>,
    },

    /// Show your active lend and borrow positions on Loopscale
    GetPosition {
        /// Wallet address (auto-resolved from onchainos if omitted)
        #[arg(long)]
        wallet: Option<String>,
    },

    /// Deposit tokens into a Loopscale lending vault to earn yield
    Lend {
        /// Token to lend: USDC or SOL (required)
        #[arg(long)]
        token: String,

        /// Amount to deposit in human-readable units (e.g. 10.0 for 10 USDC)
        #[arg(long)]
        amount: f64,

        /// Vault address (defaults to the largest vault for the given token)
        #[arg(long)]
        vault: Option<String>,

        /// Preview operation without broadcasting (no on-chain effect)
        #[arg(long)]
        dry_run: bool,
    },

    /// Withdraw tokens from a Loopscale lending vault
    Withdraw {
        /// Token to withdraw: USDC or SOL
        #[arg(long)]
        token: String,

        /// Amount to withdraw in human-readable units
        #[arg(long)]
        amount: Option<f64>,

        /// Withdraw entire deposit from the vault
        #[arg(long)]
        all: bool,

        /// Vault address (defaults to the largest vault for the given token)
        #[arg(long)]
        vault: Option<String>,

        /// Preview operation without broadcasting
        #[arg(long)]
        dry_run: bool,
    },

    /// Borrow tokens against collateral on Loopscale (two-step: create + borrow)
    Borrow {
        /// Token to borrow: USDC or SOL
        #[arg(long)]
        principal: String,

        /// Amount to borrow in human-readable units (e.g. 50.0 for 50 USDC)
        #[arg(long)]
        amount: f64,

        /// Collateral token: USDC, SOL, or mint address
        #[arg(long)]
        collateral: String,

        /// Collateral amount in human-readable units (e.g. 1.0 for 1 SOL)
        #[arg(long)]
        collateral_amount: f64,

        /// Loan duration value (default: 7)
        #[arg(long, default_value = "7")]
        duration: u64,

        /// Duration type: 0=days, 1=weeks, 2=months, 3=minutes, 4=years (default: 0)
        #[arg(long, default_value = "0")]
        duration_type: u8,

        /// Preview operation without broadcasting
        #[arg(long)]
        dry_run: bool,
    },

    /// Repay a Loopscale loan (may submit multiple transactions sequentially)
    Repay {
        /// Loan PDA address (from get-position or borrow output)
        #[arg(long)]
        loan: String,

        /// Amount to repay in human-readable units
        #[arg(long)]
        amount: Option<f64>,

        /// Repay entire outstanding principal and close the loan
        #[arg(long)]
        all: bool,

        /// Token being repaid: USDC or SOL (auto-detected from loan if omitted)
        #[arg(long)]
        token: Option<String>,

        /// Preview operation without broadcasting
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let global_dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::GetVaults { token } => {
            commands::get_vaults::run(token).await
        }
        Commands::GetPosition { wallet } => {
            commands::get_position::run(wallet).await
        }
        Commands::Lend { token, amount, vault, dry_run } => {
            commands::lend::run(token, amount, vault, global_dry_run || dry_run).await
        }
        Commands::Withdraw { token, amount, all, vault, dry_run } => {
            commands::withdraw::run(token, amount, vault, all, global_dry_run || dry_run).await
        }
        Commands::Borrow { principal, amount, collateral, collateral_amount, duration, duration_type, dry_run } => {
            commands::borrow::run(principal, amount, collateral, collateral_amount, duration, duration_type, global_dry_run || dry_run).await
        }
        Commands::Repay { loan, amount, all, token, dry_run } => {
            commands::repay::run(loan, amount, all, token, global_dry_run || dry_run).await
        }
    };

    if let Err(e) = result {
        eprintln!("{}", serde_json::json!({ "ok": false, "error": e.to_string() }));
        std::process::exit(1);
    }
}
