mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "ion-protocol",
    about = "Ion Protocol CDP lending plugin -- supply wstETH/WETH to earn yield, or borrow against LRT collateral (rsETH, rswETH, ezETH, weETH). Ethereum Mainnet only.",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Chain ID (must be 1 for Ethereum Mainnet)
    #[arg(long, global = true, default_value = "1")]
    chain: u64,

    /// Wallet address (auto-resolved from onchainos if omitted)
    #[arg(long, global = true)]
    from: Option<String>,

    /// Simulate without broadcasting transactions
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// List all 4 Ion Protocol pools with borrow APY and TVL
    GetPools {},

    /// Show collateral deposited and debt for a wallet address
    GetPosition {},

    /// Supply wstETH or WETH to earn interest (lender side)
    Lend {
        /// Pool name or collateral symbol (e.g. "rsETH", "rsETH/wstETH", "weETH")
        #[arg(long)]
        pool: String,

        /// Amount of lend token to supply in WAD units (18 decimals, e.g. 10000000000000000 = 0.01 wstETH)
        #[arg(long)]
        amount: u128,
    },

    /// Withdraw previously lent wstETH or WETH from IonPool
    WithdrawLend {
        /// Pool name or collateral symbol
        #[arg(long)]
        pool: String,

        /// Amount to withdraw in WAD units (18 decimals)
        #[arg(long)]
        amount: u128,
    },

    /// Deposit LRT collateral without borrowing (steps 1-3 of borrow flow)
    DepositCollateral {
        /// Pool name or collateral symbol (e.g. "rsETH", "weETH")
        #[arg(long)]
        pool: String,

        /// Collateral amount in WAD units (18 decimals)
        #[arg(long)]
        amount: u128,
    },

    /// Full 4-step borrow: approve collateral -> GemJoin.join -> depositCollateral -> borrow
    Borrow {
        /// Pool name or collateral symbol (e.g. "rsETH")
        #[arg(long)]
        pool: String,

        /// Collateral amount to deposit in WAD units (18 decimals)
        #[arg(long)]
        collateral_amount: u128,

        /// Loan token amount to borrow in WAD units (18 decimals, will be normalized internally)
        #[arg(long)]
        borrow_amount: u128,
    },

    /// Repay borrowed debt (with optional collateral withdrawal)
    Repay {
        /// Pool name or collateral symbol
        #[arg(long)]
        pool: String,

        /// Amount of lend token to repay in WAD units (omit if using --all)
        #[arg(long)]
        amount: Option<u128>,

        /// Repay the full outstanding debt (reads normalizedDebt from chain, adds 0.1% buffer)
        #[arg(long, default_value = "false")]
        all: bool,

        /// Also withdraw collateral from vault after repay
        #[arg(long, default_value = "false")]
        withdraw_collateral: bool,

        /// Collateral amount to withdraw in WAD units (required if --withdraw-collateral)
        #[arg(long)]
        collateral_amount: Option<u128>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::GetPools {} => commands::get_pools::run().await,

        Commands::GetPosition {} => {
            commands::get_position::run(cli.chain, cli.from.as_deref()).await
        }

        Commands::Lend { pool, amount } => {
            commands::lend::run(cli.chain, &pool, amount, cli.from.as_deref(), cli.dry_run).await
        }

        Commands::WithdrawLend { pool, amount } => {
            commands::withdraw_lend::run(cli.chain, &pool, amount, cli.from.as_deref(), cli.dry_run)
                .await
        }

        Commands::DepositCollateral { pool, amount } => {
            commands::deposit_collateral::run(
                cli.chain,
                &pool,
                amount,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::Borrow {
            pool,
            collateral_amount,
            borrow_amount,
        } => {
            commands::borrow::run(
                cli.chain,
                &pool,
                collateral_amount,
                borrow_amount,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }

        Commands::Repay {
            pool,
            amount,
            all,
            withdraw_collateral,
            collateral_amount,
        } => {
            commands::repay::run(
                cli.chain,
                &pool,
                amount,
                all,
                withdraw_collateral,
                collateral_amount,
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
