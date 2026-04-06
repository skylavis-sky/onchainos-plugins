mod abi;
#[allow(dead_code)]
mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};
use config::{CHAIN_ETHEREUM, ETH_VAULT_STETH};

#[derive(Parser)]
#[command(
    name = "cian",
    version = "0.1.0",
    about = "CIAN Yield Layer -- multi-chain ERC4626 yield vaults for ETH/BTC LST assets"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all public CIAN vaults on a chain with APY and TVL
    ListVaults {
        /// Chain ID: 1 (Ethereum, default), 42161 (Arbitrum), 56 (BSC)
        #[arg(long, default_value_t = CHAIN_ETHEREUM)]
        chain: u64,
    },

    /// Query your position in a CIAN vault (shares, asset value, earnings)
    GetPositions {
        /// Chain ID: 1 (Ethereum, default), 42161 (Arbitrum), 56 (BSC), 5000 (Mantle)
        #[arg(long, default_value_t = CHAIN_ETHEREUM)]
        chain: u64,

        /// Vault proxy address (e.g. 0xB13aa2d0345b0439b064f26B82D8dCf3f508775d)
        #[arg(long, default_value = ETH_VAULT_STETH)]
        vault: String,

        /// Wallet address to query (defaults to onchainos active wallet)
        #[arg(long, default_value = "")]
        wallet: String,
    },

    /// Deposit tokens into a CIAN vault (approve + optionalDeposit)
    Deposit {
        /// Chain ID: 1 (Ethereum, default), 42161 (Arbitrum), 56 (BSC), 5000 (Mantle)
        #[arg(long, default_value_t = CHAIN_ETHEREUM)]
        chain: u64,

        /// Vault proxy address
        #[arg(long, default_value = ETH_VAULT_STETH)]
        vault: String,

        /// Underlying token address to deposit (e.g. WETH, stETH, pumpBTC address)
        #[arg(long)]
        token: String,

        /// Amount to deposit in human-readable form (e.g. 1.0)
        #[arg(long)]
        amount: f64,

        /// Token decimals (default: 18)
        #[arg(long, default_value_t = 18u32)]
        decimals: u32,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Request withdrawal from a CIAN vault (queued, non-instant)
    RequestWithdraw {
        /// Chain ID: 1 (Ethereum, default), 42161 (Arbitrum), 56 (BSC), 5000 (Mantle)
        #[arg(long, default_value_t = CHAIN_ETHEREUM)]
        chain: u64,

        /// Vault proxy address
        #[arg(long, default_value = ETH_VAULT_STETH)]
        vault: String,

        /// Number of yl-token shares to redeem in human-readable form (e.g. 0.5)
        #[arg(long)]
        shares: f64,

        /// Token address to receive on redemption (ETH-class vaults only, e.g. WETH address)
        /// Leave empty for BTC-class vaults (pumpBTC)
        #[arg(long, default_value = "")]
        token: String,

        /// Share token decimals (default: 18)
        #[arg(long, default_value_t = 18u32)]
        decimals: u32,

        /// Simulate without broadcasting transactions
        #[arg(long)]
        dry_run: bool,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::ListVaults { chain } => commands::list_vaults::run(chain).await,

        Commands::GetPositions { chain, vault, wallet } => {
            // Resolve wallet from onchainos if not provided
            let wallet_addr = if wallet.is_empty() {
                match onchainos::resolve_wallet(chain) {
                    Ok(addr) => addr,
                    Err(e) => {
                        eprintln!("Error resolving wallet: {:#}", e);
                        std::process::exit(1);
                    }
                }
            } else {
                wallet
            };
            commands::get_positions::run(chain, &vault, &wallet_addr).await
        }

        Commands::Deposit {
            chain,
            vault,
            token,
            amount,
            decimals,
            dry_run,
        } => commands::deposit::run(chain, &vault, &token, amount, decimals, dry_run).await,

        Commands::RequestWithdraw {
            chain,
            vault,
            shares,
            token,
            decimals,
            dry_run,
        } => {
            let token_opt = if token.is_empty() { None } else { Some(token.as_str()) };
            commands::request_withdraw::run(chain, &vault, shares, token_opt, decimals, dry_run)
                .await
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}
