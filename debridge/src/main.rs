mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "debridge",
    version = "0.1.0",
    about = "deBridge DLN cross-chain bridge plugin — quote and execute cross-chain swaps via the Decentralized Liquidity Network"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch a cross-chain swap quote (no transaction built)
    GetQuote {
        /// Source chain onchainos ID (e.g. 42161 for Arbitrum, 501 for Solana)
        #[arg(long)]
        src_chain_id: u64,
        /// Destination chain onchainos ID (e.g. 8453 for Base, 501 for Solana)
        #[arg(long)]
        dst_chain_id: u64,
        /// Source token address (EVM: 0x...; Solana: base58 mint; native ETH: 0x000...000)
        #[arg(long)]
        src_token: String,
        /// Destination token address
        #[arg(long)]
        dst_token: String,
        /// Input amount in token base units (e.g. 1000000 for 1 USDC)
        #[arg(long)]
        amount: String,
    },

    /// Execute a cross-chain bridge (quote → approve if needed → submit order)
    Bridge {
        /// Source chain onchainos ID (e.g. 42161 for Arbitrum, 501 for Solana)
        #[arg(long)]
        src_chain_id: u64,
        /// Destination chain onchainos ID (e.g. 8453 for Base, 501 for Solana)
        #[arg(long)]
        dst_chain_id: u64,
        /// Source token address
        #[arg(long)]
        src_token: String,
        /// Destination token address
        #[arg(long)]
        dst_token: String,
        /// Input amount in token base units
        #[arg(long)]
        amount: String,
        /// Override destination recipient address (default: auto-resolved wallet)
        #[arg(long)]
        recipient: Option<String>,
        /// Preview mode — show calldata without broadcasting
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },

    /// Query the status of a deBridge DLN order by order ID
    GetStatus {
        /// Order ID returned by bridge (0x hex string)
        #[arg(long)]
        order_id: String,
    },

    /// List all chains supported by deBridge DLN
    GetChains,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GetQuote {
            src_chain_id,
            dst_chain_id,
            src_token,
            dst_token,
            amount,
        } => {
            commands::get_quote::run(commands::get_quote::GetQuoteArgs {
                src_chain_id,
                dst_chain_id,
                src_token,
                dst_token,
                amount,
            })
            .await?;
        }

        Commands::Bridge {
            src_chain_id,
            dst_chain_id,
            src_token,
            dst_token,
            amount,
            recipient,
            dry_run,
        } => {
            commands::bridge::run(commands::bridge::BridgeArgs {
                src_chain_id,
                dst_chain_id,
                src_token,
                dst_token,
                amount,
                recipient,
                dry_run,
            })
            .await?;
        }

        Commands::GetStatus { order_id } => {
            commands::get_status::run(commands::get_status::GetStatusArgs { order_id }).await?;
        }

        Commands::GetChains => {
            commands::get_chains::run().await?;
        }
    }

    Ok(())
}
