mod commands;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "spectra",
    about = "Spectra Finance plugin — yield tokenization: deposit/redeem PT+YT, claim yield, swap PT via Curve"
)]
struct Cli {
    /// Chain ID (default: 8453 Base — primary Spectra deployment)
    #[arg(long, default_value = "8453", global = true)]
    chain: u64,

    /// Simulate without broadcasting any transaction
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List active Spectra PT pools with maturity, APY, and TVL
    GetPools {
        /// Only show active (non-expired) pools
        #[arg(long)]
        active_only: bool,

        /// Max number of pools to return
        #[arg(long, default_value = "20")]
        limit: usize,
    },

    /// Get PT, YT, and yield positions for a wallet address
    GetPosition {
        /// Wallet address (defaults to logged-in wallet)
        #[arg(long)]
        user: Option<String>,
    },

    /// Deposit underlying asset (or IBT) into a PrincipalToken to receive PT + YT
    Deposit {
        /// PrincipalToken contract address
        #[arg(long)]
        pt: String,

        /// Amount to deposit in wei (underlying or IBT units)
        #[arg(long)]
        amount: String,

        /// Deposit IBT directly instead of underlying (skips internal IBT wrapping)
        #[arg(long)]
        use_ibt: bool,

        /// Recipient of PT and YT (defaults to sender)
        #[arg(long)]
        receiver: Option<String>,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance (0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },

    /// Redeem PT for underlying. Post-expiry: PT only. Pre-expiry: PT+YT pair (withdraw).
    Redeem {
        /// PrincipalToken contract address
        #[arg(long)]
        pt: String,

        /// Amount of PT shares to redeem in wei
        #[arg(long)]
        shares: String,

        /// Recipient of underlying (defaults to sender)
        #[arg(long)]
        receiver: Option<String>,

        /// Owner of PT shares (defaults to sender)
        #[arg(long)]
        owner: Option<String>,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance (0.005 = 0.5%)
        #[arg(long, default_value = "0.005")]
        slippage: f64,
    },

    /// Claim accrued yield from YT holdings via the PT contract
    ClaimYield {
        /// PrincipalToken contract address
        #[arg(long)]
        pt: String,

        /// Claim yield as IBT instead of underlying
        #[arg(long)]
        in_ibt: bool,

        /// Recipient of claimed yield (defaults to sender)
        #[arg(long)]
        receiver: Option<String>,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,
    },

    /// Swap PT <-> IBT via Curve pool using Router execute dispatcher
    Swap {
        /// PrincipalToken address
        #[arg(long)]
        pt: String,

        /// Amount to sell in wei
        #[arg(long)]
        amount_in: String,

        /// Sell PT for IBT (use --sell-pt); omit to buy PT (sell IBT)
        #[arg(long)]
        sell_pt: bool,

        /// Minimum amount to receive in wei (0 = auto-compute from slippage)
        #[arg(long, default_value = "0")]
        min_out: String,

        /// Curve pool address (auto-resolved from known pools if omitted)
        #[arg(long)]
        curve_pool: Option<String>,

        /// Sender wallet address (defaults to logged-in wallet)
        #[arg(long)]
        from: Option<String>,

        /// Slippage tolerance (0.01 = 1%)
        #[arg(long, default_value = "0.01")]
        slippage: f64,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let chain = cli.chain;
    let dry_run = cli.dry_run;

    let result = match cli.command {
        Commands::GetPools { active_only, limit } => {
            commands::get_pools::run(chain, active_only, limit).await
        }

        Commands::GetPosition { user } => {
            commands::get_position::run(user.as_deref(), chain).await
        }

        Commands::Deposit {
            pt,
            amount,
            use_ibt,
            receiver,
            from,
            slippage,
        } => {
            commands::deposit::run(
                chain,
                &pt,
                &amount,
                use_ibt,
                receiver.as_deref(),
                from.as_deref(),
                slippage,
                dry_run,
            )
            .await
        }

        Commands::Redeem {
            pt,
            shares,
            receiver,
            owner,
            from,
            slippage,
        } => {
            commands::redeem::run(
                chain,
                &pt,
                &shares,
                receiver.as_deref(),
                owner.as_deref(),
                from.as_deref(),
                slippage,
                dry_run,
            )
            .await
        }

        Commands::ClaimYield {
            pt,
            in_ibt,
            receiver,
            from,
        } => {
            commands::claim_yield::run(
                chain,
                &pt,
                receiver.as_deref(),
                from.as_deref(),
                in_ibt,
                dry_run,
            )
            .await
        }

        Commands::Swap {
            pt,
            amount_in,
            sell_pt,
            min_out,
            curve_pool,
            from,
            slippage,
        } => {
            commands::swap::run(
                chain,
                &pt,
                &amount_in,
                &min_out,
                sell_pt,
                curve_pool.as_deref(),
                from.as_deref(),
                slippage,
                dry_run,
            )
            .await
        }
    };

    match result {
        Ok(value) => {
            println!("{}", serde_json::to_string_pretty(&value).unwrap_or_default());
        }
        Err(e) => {
            let error_output = serde_json::json!({
                "ok": false,
                "error": e.to_string()
            });
            eprintln!("{}", serde_json::to_string_pretty(&error_output).unwrap_or_default());
            std::process::exit(1);
        }
    }
}
