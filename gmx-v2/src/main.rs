mod abi;
mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gmx-v2", about = "GMX V2 perpetuals and liquidity on Arbitrum/Avalanche")]
struct Cli {
    /// Target chain: "arbitrum" or "avalanche" (default: arbitrum) — can also be passed per subcommand
    #[arg(long, default_value = "arbitrum", global = true)]
    chain: String,

    /// Simulate without broadcasting on-chain transactions — can also be passed per subcommand
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List active GMX V2 markets with liquidity and rates
    ListMarkets(commands::list_markets::ListMarketsArgs),

    /// Get current oracle prices for GMX V2 tokens
    GetPrices(commands::get_prices::GetPricesArgs),

    /// Get open positions for a wallet address
    GetPositions(commands::get_positions::GetPositionsArgs),

    /// Get open orders for a wallet address
    GetOrders(commands::get_orders::GetOrdersArgs),

    /// Open a leveraged long or short position (market order)
    OpenPosition(commands::open_position::OpenPositionArgs),

    /// Close an open position (market decrease)
    ClosePosition(commands::close_position::ClosePositionArgs),

    /// Place a limit, stop-loss, or take-profit order
    PlaceOrder(commands::place_order::PlaceOrderArgs),

    /// Cancel a pending order by its key
    CancelOrder(commands::cancel_order::CancelOrderArgs),

    /// Deposit liquidity into a GM pool
    DepositLiquidity(commands::deposit_liquidity::DepositLiquidityArgs),

    /// Withdraw liquidity from a GM pool (burn GM tokens)
    WithdrawLiquidity(commands::withdraw_liquidity::WithdrawLiquidityArgs),

    /// Claim accrued funding fees from GMX V2 positions
    ClaimFundingFees(commands::claim_funding_fees::ClaimFundingFeesArgs),
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = run(cli).await;
    if let Err(e) = result {
        eprintln!("Error: {:#}", e);
        std::process::exit(1);
    }
}

async fn run(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        Commands::ListMarkets(args) => {
            commands::list_markets::run(&cli.chain, args).await
        }
        Commands::GetPrices(args) => {
            commands::get_prices::run(&cli.chain, args).await
        }
        Commands::GetPositions(args) => {
            commands::get_positions::run(&cli.chain, args).await
        }
        Commands::GetOrders(args) => {
            commands::get_orders::run(&cli.chain, args).await
        }
        Commands::OpenPosition(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::open_position::run(&chain, dry_run, args).await
        }
        Commands::ClosePosition(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::close_position::run(&chain, dry_run, args).await
        }
        Commands::PlaceOrder(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::place_order::run(&chain, dry_run, args).await
        }
        Commands::CancelOrder(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::cancel_order::run(&chain, dry_run, args).await
        }
        Commands::DepositLiquidity(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::deposit_liquidity::run(&chain, dry_run, args).await
        }
        Commands::WithdrawLiquidity(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::withdraw_liquidity::run(&chain, dry_run, args).await
        }
        Commands::ClaimFundingFees(args) => {
            let chain = args.chain.as_deref().unwrap_or(&cli.chain).to_string();
            let dry_run = args.dry_run || cli.dry_run;
            commands::claim_funding_fees::run(&chain, dry_run, args).await
        }
    }
}
