mod abi;
mod api;
mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};
use commands::{bridge, get_limits, get_quote, get_routes, get_status};

#[derive(Parser)]
#[command(
    name = "across",
    version = "0.1.0",
    about = "Across Protocol cross-chain bridge plugin for onchainos"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a cross-chain bridge quote (fees, output amount, estimated fill time)
    GetQuote {
        /// Source chain token address
        #[arg(long)]
        input_token: String,

        /// Destination chain token address
        #[arg(long)]
        output_token: String,

        /// Origin chain ID (e.g. 1 = Ethereum)
        #[arg(long)]
        origin_chain_id: u64,

        /// Destination chain ID (e.g. 10 = Optimism)
        #[arg(long)]
        destination_chain_id: u64,

        /// Transfer amount in token base units (e.g. 100000000 for 100 USDC)
        #[arg(long)]
        amount: String,

        /// Optional depositor address for more accurate quote
        #[arg(long)]
        depositor: Option<String>,

        /// Optional recipient address on destination chain
        #[arg(long)]
        recipient: Option<String>,
    },

    /// List available cross-chain routes
    GetRoutes {
        /// Filter by origin chain ID
        #[arg(long)]
        origin_chain_id: Option<u64>,

        /// Filter by destination chain ID
        #[arg(long)]
        destination_chain_id: Option<u64>,

        /// Filter by origin token address
        #[arg(long)]
        origin_token: Option<String>,

        /// Filter by destination token address
        #[arg(long)]
        destination_token: Option<String>,
    },

    /// Get transfer limits (min/max) for a specific route
    GetLimits {
        /// Source chain token address
        #[arg(long)]
        input_token: String,

        /// Destination chain token address
        #[arg(long)]
        output_token: String,

        /// Origin chain ID
        #[arg(long)]
        origin_chain_id: u64,

        /// Destination chain ID
        #[arg(long)]
        destination_chain_id: u64,
    },

    /// Bridge tokens cross-chain via Across Protocol
    Bridge {
        /// Source chain token address
        #[arg(long)]
        input_token: String,

        /// Destination chain token address
        #[arg(long)]
        output_token: String,

        /// Origin chain ID
        #[arg(long)]
        origin_chain_id: u64,

        /// Destination chain ID
        #[arg(long)]
        destination_chain_id: u64,

        /// Transfer amount in token base units
        #[arg(long)]
        amount: String,

        /// Optional recipient address on destination chain (defaults to wallet address)
        #[arg(long)]
        recipient: Option<String>,

        /// Dry run: simulate without submitting on-chain transactions
        #[arg(long, default_value = "false")]
        dry_run: bool,
    },

    /// Get the status of a bridge deposit
    GetStatus {
        /// Source chain transaction hash (from bridge command)
        #[arg(long)]
        tx_hash: Option<String>,

        /// Deposit ID (alternative lookup)
        #[arg(long)]
        deposit_id: Option<u64>,

        /// Origin chain ID (required when using --deposit-id)
        #[arg(long)]
        origin_chain_id: Option<u64>,

        /// Relay data hash (alternative lookup)
        #[arg(long)]
        relay_data_hash: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GetQuote {
            input_token,
            output_token,
            origin_chain_id,
            destination_chain_id,
            amount,
            depositor,
            recipient,
        } => {
            get_quote::run(get_quote::GetQuoteArgs {
                input_token,
                output_token,
                origin_chain_id,
                destination_chain_id,
                amount,
                depositor,
                recipient,
            })
            .await?;
        }

        Commands::GetRoutes {
            origin_chain_id,
            destination_chain_id,
            origin_token,
            destination_token,
        } => {
            get_routes::run(get_routes::GetRoutesArgs {
                origin_chain_id,
                destination_chain_id,
                origin_token,
                destination_token,
            })
            .await?;
        }

        Commands::GetLimits {
            input_token,
            output_token,
            origin_chain_id,
            destination_chain_id,
        } => {
            get_limits::run(get_limits::GetLimitsArgs {
                input_token,
                output_token,
                origin_chain_id,
                destination_chain_id,
            })
            .await?;
        }

        Commands::Bridge {
            input_token,
            output_token,
            origin_chain_id,
            destination_chain_id,
            amount,
            recipient,
            dry_run,
        } => {
            bridge::run(bridge::BridgeArgs {
                input_token,
                output_token,
                origin_chain_id,
                destination_chain_id,
                amount,
                recipient,
                dry_run,
            })
            .await?;
        }

        Commands::GetStatus {
            tx_hash,
            deposit_id,
            origin_chain_id,
            relay_data_hash,
        } => {
            get_status::run(get_status::GetStatusArgs {
                deposit_txn_ref: tx_hash,
                deposit_id,
                origin_chain_id,
                relay_data_hash,
            })
            .await?;
        }
    }

    Ok(())
}
