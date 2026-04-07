use anyhow::Result;
use clap::Args;
use crate::{config, onchainos, rpc};

#[derive(Args, Debug)]
pub struct GetPositionArgs {
    /// Aura pool ID to check staked BPT position for
    #[arg(long)]
    pub pool_id: Option<u64>,

    /// Wallet address to query (defaults to onchainos logged-in wallet)
    #[arg(long)]
    pub address: Option<String>,
}

pub async fn run(args: GetPositionArgs, chain_id: u64) -> Result<()> {
    let wallet = match args.address {
        Some(addr) => addr,
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Liquid AURA and BAL balances
    let aura_liquid = rpc::erc20_balance_of(config::AURA_TOKEN, &wallet).await.unwrap_or(0);
    let bal_liquid = rpc::erc20_balance_of(config::BAL_TOKEN, &wallet).await.unwrap_or(0);

    // vlAURA locked balance
    let vlaura_balance = rpc::erc20_balance_of(config::AURA_LOCKER, &wallet).await.unwrap_or(0);

    // Per-pool position (if pool_id specified)
    let pool_position = if let Some(pid) = args.pool_id {
        match rpc::booster_pool_info(config::BOOSTER, pid).await {
            Ok((lp_token, crv_rewards, shutdown)) => {
                let staked = rpc::erc20_balance_of(&crv_rewards, &wallet).await.unwrap_or(0);
                let earned = rpc::base_reward_pool_earned(&crv_rewards, &wallet).await.unwrap_or(0);
                Some(serde_json::json!({
                    "pool_id": pid,
                    "lp_token": lp_token,
                    "base_reward_pool": crv_rewards,
                    "staked_bpt": rpc::format_amount(staked, 18),
                    "pending_bal_rewards": rpc::format_amount(earned, 18),
                    "shutdown": shutdown
                }))
            }
            Err(e) => {
                Some(serde_json::json!({
                    "pool_id": pid,
                    "error": format!("Failed to fetch pool info: {}", e)
                }))
            }
        }
    } else {
        None
    };

    let mut data = serde_json::json!({
        "wallet": wallet,
        "chain": "ethereum",
        "chain_id": chain_id,
        "vlAURA_locked": {
            "contract": config::AURA_LOCKER,
            "balance": rpc::format_amount(vlaura_balance, 18),
            "note": "16-week lock period. Use unlock-aura to process expired locks."
        },
        "liquid_balances": {
            "AURA": rpc::format_amount(aura_liquid, 18),
            "BAL": rpc::format_amount(bal_liquid, 18)
        }
    });

    if let Some(pos) = pool_position {
        data["pool_position"] = pos;
    } else {
        data["note"] = serde_json::Value::String(
            "Pass --pool-id <pid> to check staked BPT and pending rewards for a specific pool. Use get-pools to list available pool IDs.".to_string()
        );
    }

    let output = serde_json::json!({ "ok": true, "data": data });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
