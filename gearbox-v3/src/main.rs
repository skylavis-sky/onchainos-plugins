mod abi;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "gearbox-v3",
    about = "Gearbox V3 leveraged Credit Account management via OnchaionOS",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Chain ID (default: 42161 Arbitrum One)
    #[arg(long, global = true, default_value = "42161")]
    chain: u64,

    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,

    /// Simulate without broadcasting — shows calldata without submitting
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List Gearbox V3 Credit Managers and their debt limits
    GetPools {},

    /// Show Credit Account(s) for a wallet address
    GetAccount {},

    /// Open a leveraged Credit Account (approve + openCreditAccount with increaseDebt + addCollateral)
    OpenAccount {
        /// CreditFacadeV3 contract address
        #[arg(long)]
        facade: String,

        /// CreditManagerV3 contract address (approval target — NOT the facade)
        #[arg(long)]
        manager: String,

        /// Underlying token symbol (e.g. USDC, WETH, USDC.E)
        #[arg(long)]
        token: String,

        /// Token contract address (0x...)
        #[arg(long)]
        token_addr: String,

        /// Collateral amount to deposit (human-readable, e.g. 1000.0)
        #[arg(long)]
        collateral: f64,

        /// Amount to borrow (human-readable). Must be >= minDebt for the Credit Manager.
        /// For Trade USDC Tier 2: minimum 1000 USDC.
        #[arg(long)]
        borrow: f64,
    },

    /// Add collateral to an existing Credit Account (approve + multicall addCollateral)
    AddCollateral {
        /// CreditFacadeV3 contract address
        #[arg(long)]
        facade: String,

        /// CreditManagerV3 contract address (approval target)
        #[arg(long)]
        manager: String,

        /// Credit Account address (0x...)
        #[arg(long)]
        account: String,

        /// Token symbol (e.g. USDC)
        #[arg(long)]
        token: String,

        /// Token contract address (0x...)
        #[arg(long)]
        token_addr: String,

        /// Amount to add (human-readable)
        #[arg(long)]
        amount: f64,
    },

    /// Close a Credit Account (decreaseDebt(MAX) + withdrawCollateral(MAX))
    CloseAccount {
        /// CreditFacadeV3 contract address
        #[arg(long)]
        facade: String,

        /// Credit Account address to close
        #[arg(long)]
        account: String,

        /// Recipient address for withdrawn funds (defaults to wallet address)
        #[arg(long)]
        to: Option<String>,

        /// Underlying token address (e.g. USDC address)
        #[arg(long)]
        underlying: String,
    },

    /// Withdraw collateral from a Credit Account (multicall withdrawCollateral)
    Withdraw {
        /// CreditFacadeV3 contract address
        #[arg(long)]
        facade: String,

        /// Credit Account address
        #[arg(long)]
        account: String,

        /// Token symbol (e.g. USDC)
        #[arg(long)]
        token: String,

        /// Token contract address (0x...)
        #[arg(long)]
        token_addr: String,

        /// Amount to withdraw (omit to withdraw all)
        #[arg(long)]
        amount: Option<f64>,

        /// Recipient address (defaults to wallet address)
        #[arg(long)]
        to: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::GetPools {} => {
            commands::get_pools::run(cli.chain).await
        }

        Commands::GetAccount {} => {
            commands::get_account::run(cli.chain, cli.from.as_deref()).await
        }

        Commands::OpenAccount {
            facade,
            manager,
            token,
            token_addr,
            collateral,
            borrow,
        } => {
            commands::open_account::run(
                cli.chain,
                &facade,
                &manager,
                &token,
                &token_addr,
                collateral,
                borrow,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::AddCollateral {
            facade,
            manager,
            account,
            token,
            token_addr,
            amount,
        } => {
            commands::add_collateral::run(
                cli.chain,
                &facade,
                &manager,
                &account,
                &token,
                &token_addr,
                amount,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::CloseAccount {
            facade,
            account,
            to,
            underlying,
        } => {
            commands::close_account::run(
                cli.chain,
                &facade,
                &account,
                to.as_deref(),
                &underlying,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::Withdraw {
            facade,
            account,
            token,
            token_addr,
            amount,
            to,
        } => {
            commands::withdraw::run(
                cli.chain,
                &facade,
                &account,
                &token,
                &token_addr,
                amount,
                to.as_deref(),
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
    };

    match result {
        Ok(val) => {
            println!("{}", serde_json::to_string_pretty(&val).unwrap_or_default());
        }
        Err(err) => {
            let error_json = serde_json::json!({
                "ok": false,
                "error": err.to_string()
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&error_json).unwrap_or_default()
            );
            std::process::exit(1);
        }
    }
}
