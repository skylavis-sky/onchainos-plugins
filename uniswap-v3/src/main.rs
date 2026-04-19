mod calldata;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "uniswap-v3",
    about = "Swap tokens and manage concentrated liquidity positions on Uniswap V3"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a swap quote via QuoterV2 (read-only, no transaction)
    GetQuote {
        /// Input token address or symbol (e.g. USDC, WETH, or 0x...)
        #[arg(long)]
        token_in: String,
        /// Output token address or symbol
        #[arg(long)]
        token_out: String,
        /// Human-readable input amount (e.g. "100" or "1.5")
        #[arg(long)]
        amount: String,
        /// Chain ID (1=Ethereum, 10=Optimism, 137=Polygon, 8453=Base, 42161=Arbitrum)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// Override fee tier (100, 500, 3000, 10000). If omitted, auto-selects best.
        #[arg(long)]
        fee: Option<u32>,
    },

    /// Swap tokens via SwapRouter02.exactInputSingle
    Swap {
        /// Input token address or symbol
        #[arg(long)]
        token_in: String,
        /// Output token address or symbol
        #[arg(long)]
        token_out: String,
        /// Human-readable input amount (e.g. "100" or "1.5")
        #[arg(long)]
        amount: String,
        /// Slippage tolerance in basis points (default: 50 = 0.5%)
        #[arg(long, default_value = "50")]
        slippage_bps: u64,
        /// Chain ID (1=Ethereum, 10=Optimism, 137=Polygon, 8453=Base, 42161=Arbitrum)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// Override fee tier (100, 500, 3000, 10000). If omitted, auto-selects best.
        #[arg(long)]
        fee: Option<u32>,
        /// Preview calldata without submitting any transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// List Uniswap V3 pools for a token pair via UniswapV3Factory (read-only)
    GetPools {
        /// First token address or symbol
        #[arg(long)]
        token_a: String,
        /// Second token address or symbol
        #[arg(long)]
        token_b: String,
        /// Chain ID (1=Ethereum, 10=Optimism, 137=Polygon, 8453=Base, 42161=Arbitrum)
        #[arg(long, default_value = "1")]
        chain: u64,
    },

    /// View Uniswap V3 LP positions for a wallet (read-only)
    GetPositions {
        /// Wallet address to query (optional — uses connected wallet if omitted)
        #[arg(long)]
        owner: Option<String>,
        /// Query a specific position by NFT token ID
        #[arg(long)]
        token_id: Option<u128>,
        /// Chain ID (1=Ethereum, 10=Optimism, 137=Polygon, 8453=Base, 42161=Arbitrum)
        #[arg(long, default_value = "1")]
        chain: u64,
    },

    /// Add concentrated liquidity via NonfungiblePositionManager.mint
    AddLiquidity {
        /// First token address or symbol
        #[arg(long)]
        token_a: String,
        /// Second token address or symbol
        #[arg(long)]
        token_b: String,
        /// Fee tier (100=0.01%, 500=0.05%, 3000=0.3%, 10000=1.0%)
        #[arg(long, default_value = "3000")]
        fee: u32,
        /// Human-readable amount for token A
        #[arg(long)]
        amount_a: String,
        /// Human-readable amount for token B
        #[arg(long)]
        amount_b: String,
        /// Lower tick boundary (default: full range for fee tier)
        #[arg(long, allow_hyphen_values = true)]
        tick_lower: Option<i32>,
        /// Upper tick boundary (default: full range for fee tier)
        #[arg(long, allow_hyphen_values = true)]
        tick_upper: Option<i32>,
        /// Slippage tolerance in basis points (default: 50 = 0.5%)
        #[arg(long, default_value = "50")]
        slippage_bps: u64,
        /// Chain ID (1=Ethereum, 10=Optimism, 137=Polygon, 8453=Base, 42161=Arbitrum)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// Preview calldata without submitting any transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Remove liquidity from a V3 position (decreaseLiquidity + collect + optional burn)
    RemoveLiquidity {
        /// NFT position token ID
        #[arg(long)]
        token_id: u128,
        /// Percentage of liquidity to remove (0–100, default: 100)
        #[arg(long, default_value = "100")]
        liquidity_pct: f64,
        /// Chain ID (1=Ethereum, 10=Optimism, 137=Polygon, 8453=Base, 42161=Arbitrum)
        #[arg(long, default_value = "1")]
        chain: u64,
        /// Preview calldata without submitting any transactions
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::GetQuote {
            token_in,
            token_out,
            amount,
            chain,
            fee,
        } => {
            commands::get_quote::run(commands::get_quote::GetQuoteArgs {
                token_in,
                token_out,
                amount,
                chain,
                fee,
            })
            .await?;
        }

        Commands::Swap {
            token_in,
            token_out,
            amount,
            slippage_bps,
            chain,
            fee,
            dry_run,
        } => {
            commands::swap::run(commands::swap::SwapArgs {
                token_in,
                token_out,
                amount,
                slippage_bps,
                chain,
                fee,
                dry_run,
            })
            .await?;
        }

        Commands::GetPools {
            token_a,
            token_b,
            chain,
        } => {
            commands::get_pools::run(commands::get_pools::GetPoolsArgs {
                token_a,
                token_b,
                chain,
            })
            .await?;
        }

        Commands::GetPositions {
            owner,
            token_id,
            chain,
        } => {
            commands::get_positions::run(commands::get_positions::GetPositionsArgs {
                owner,
                token_id,
                chain,
            })
            .await?;
        }

        Commands::AddLiquidity {
            token_a,
            token_b,
            fee,
            amount_a,
            amount_b,
            tick_lower,
            tick_upper,
            slippage_bps,
            chain,
            dry_run,
        } => {
            commands::add_liquidity::run(commands::add_liquidity::AddLiquidityArgs {
                token_a,
                token_b,
                fee,
                amount_a,
                amount_b,
                tick_lower,
                tick_upper,
                slippage_bps,
                chain,
                dry_run,
            })
            .await?;
        }

        Commands::RemoveLiquidity {
            token_id,
            liquidity_pct,
            chain,
            dry_run,
        } => {
            commands::remove_liquidity::run(commands::remove_liquidity::RemoveLiquidityArgs {
                token_id,
                liquidity_pct,
                chain,
                dry_run,
            })
            .await?;
        }
    }

    Ok(())
}
