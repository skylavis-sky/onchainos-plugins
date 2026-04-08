mod abi;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "archimedes",
    about = "Archimedes Finance leveraged yield protocol on Ethereum",
    version = "0.1.0"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Wallet address (defaults to active onchainos wallet)
    #[arg(long, global = true)]
    from: Option<String>,
    /// Simulate without broadcasting any transaction
    #[arg(long, global = true, default_value = "false")]
    dry_run: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Open a leveraged OUSD position via Zapper.zapIn
    ///
    /// Approves the stablecoin, then calls Zapper.zapIn to mint a PositionToken NFT.
    OpenPosition {
        /// Amount of stablecoin to deposit (human-readable, e.g. 1000 for 1000 USDC)
        #[arg(long)]
        amount: f64,
        /// Input stablecoin: USDC, USDT, or DAI
        #[arg(long, default_value = "USDC")]
        token: String,
        /// Leverage cycles (1-10, where 10 = maximum leverage)
        #[arg(long, default_value = "5")]
        cycles: u64,
        /// Pay ARCH origination fee from wallet (false = fee taken from stablecoin)
        #[arg(long, default_value = "false")]
        use_arch: bool,
        /// Maximum slippage in basis points (50 = 0.5%)
        #[arg(long, default_value = "50")]
        max_slippage_bps: u16,
    },
    /// Close a leveraged position and redeem OUSD
    ///
    /// Sets approval for LeverageEngine if needed, then calls unwindLeveragedPosition.
    ClosePosition {
        /// PositionToken NFT ID to close
        #[arg(long)]
        token_id: u128,
        /// Minimum OUSD to receive (default: 95% of current position value)
        #[arg(long)]
        min_return: Option<f64>,
    },
    /// List all PositionToken NFTs and their details for a wallet
    GetPositions {
        /// Wallet address to query (defaults to active onchainos wallet)
        #[arg(long)]
        wallet: Option<String>,
    },
    /// Show current Archimedes protocol parameters
    ProtocolInfo,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::OpenPosition {
            amount,
            token,
            cycles,
            use_arch,
            max_slippage_bps,
        } => {
            commands::open_position::run(
                amount,
                &token,
                cycles,
                use_arch,
                max_slippage_bps,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
        Commands::ClosePosition {
            token_id,
            min_return,
        } => {
            commands::close_position::run(
                token_id,
                min_return,
                cli.from.as_deref(),
                cli.dry_run,
            )
            .await
        }
        Commands::GetPositions { wallet } => {
            commands::get_positions::run(wallet.as_deref()).await
        }
        Commands::ProtocolInfo => commands::protocol_info::run().await,
    };

    match result {
        Ok(val) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&val).unwrap_or_default()
            );
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
