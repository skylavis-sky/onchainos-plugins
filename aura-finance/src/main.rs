mod api;
mod commands;
mod config;
mod onchainos;
mod rpc;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "aura-finance", about = "Aura Finance plugin for onchainos - deposit BPT, claim BAL+AURA rewards, lock AURA as vlAURA")]
struct Cli {
    /// Chain ID (default: 1 Ethereum mainnet)
    #[arg(long, global = true, default_value = "1")]
    chain: u64,

    /// Simulate without broadcasting
    #[arg(long, global = true)]
    dry_run: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List Aura-supported Balancer pools with TVL and pool IDs
    GetPools(commands::get_pools::GetPoolsArgs),
    /// Query your Aura Finance positions (staked BPT, pending BAL/AURA rewards, vlAURA)
    GetPosition(commands::get_position::GetPositionArgs),
    /// Approve BPT and deposit into an Aura Booster pool (with _stake=true)
    Deposit(commands::deposit::DepositArgs),
    /// Withdraw staked BPT from an Aura BaseRewardPool
    Withdraw(commands::withdraw::WithdrawArgs),
    /// Claim pending BAL + AURA rewards from a BaseRewardPool
    ClaimRewards(commands::claim_rewards::ClaimRewardsArgs),
    /// Lock AURA as vlAURA (16-week irreversible lock) for voting and rewards
    LockAura(commands::lock_aura::LockAuraArgs),
    /// Process expired vlAURA locks to withdraw AURA
    UnlockAura(commands::unlock_aura::UnlockAuraArgs),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let chain_id = cli.chain;
    let dry_run = cli.dry_run;

    match cli.command {
        Commands::GetPools(args) => commands::get_pools::run(args, chain_id).await,
        Commands::GetPosition(args) => commands::get_position::run(args, chain_id).await,
        Commands::Deposit(args) => commands::deposit::run(args, chain_id, dry_run).await,
        Commands::Withdraw(args) => commands::withdraw::run(args, chain_id, dry_run).await,
        Commands::ClaimRewards(args) => commands::claim_rewards::run(args, chain_id, dry_run).await,
        Commands::LockAura(args) => commands::lock_aura::run(args, chain_id, dry_run).await,
        Commands::UnlockAura(args) => commands::unlock_aura::run(args, chain_id, dry_run).await,
    }
}
