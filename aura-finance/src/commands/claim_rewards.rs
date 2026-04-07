use anyhow::Result;
use clap::Args;
use crate::{config, onchainos, rpc};

#[derive(Args, Debug)]
pub struct ClaimRewardsArgs {
    /// Aura pool ID to claim rewards from
    #[arg(long)]
    pub pool_id: u64,

    /// Also claim extra rewards (default: true)
    #[arg(long, default_value = "true")]
    pub claim_extras: bool,

    /// Wallet address override
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(args: ClaimRewardsArgs, chain_id: u64, dry_run: bool) -> Result<()> {
    if dry_run {
        // getReward(address,bool) selector: 0x7050ccd9
        let wallet_padded = "0000000000000000000000000000000000000000000000000000000000000000";
        let claim_extras_hex = if args.claim_extras {
            "0000000000000000000000000000000000000000000000000000000000000001"
        } else {
            "0000000000000000000000000000000000000000000000000000000000000000"
        };
        let calldata = format!("0x7050ccd9{}{}", wallet_padded, claim_extras_hex);

        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "action": "claim-rewards",
            "pool_id": args.pool_id,
            "calldata": calldata,
            "note": "call goes to BaseRewardPool (crv_rewards) for pool. _claimExtras=true claims both BAL and AURA. ask user to confirm."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let wallet = match &args.from {
        Some(addr) => addr.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Fetch pool info to get BaseRewardPool address
    let (_lp_token, crv_rewards, _shutdown) =
        rpc::booster_pool_info(config::BOOSTER, args.pool_id).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch pool info for pid {}: {}", args.pool_id, e))?;

    // Check pending rewards
    let pending = rpc::base_reward_pool_earned(&crv_rewards, &wallet).await.unwrap_or(0);

    if pending == 0 {
        let output = serde_json::json!({
            "ok": true,
            "data": {
                "action": "claim-rewards",
                "pool_id": args.pool_id,
                "wallet": wallet,
                "pending_bal_rewards": "0",
                "note": "No pending BAL rewards to claim for this pool."
            }
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // getReward(address _account, bool _claimExtras) selector: 0x7050ccd9
    // _claimExtras=true claims both BAL and AURA from extra reward distributors
    let wallet_padded = rpc::pad_address(&wallet);
    let claim_extras_hex = if args.claim_extras {
        "0000000000000000000000000000000000000000000000000000000000000001"
    } else {
        "0000000000000000000000000000000000000000000000000000000000000000"
    };
    let calldata = format!("0x7050ccd9{}{}", wallet_padded, claim_extras_hex);

    eprintln!("Claiming BAL+AURA rewards from pool {} (ask user to confirm)...", args.pool_id);
    let result = onchainos::wallet_contract_call(
        chain_id,
        &crv_rewards,
        &calldata,
        Some(&wallet),
        None,
        false,
    ).await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "action": "claim-rewards",
            "pool_id": args.pool_id,
            "base_reward_pool": crv_rewards,
            "wallet": wallet,
            "pending_bal_rewards": rpc::format_amount(pending, 18),
            "claim_extras": args.claim_extras,
            "txHash": tx_hash,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
            "note": "BAL and AURA rewards claimed. Check your wallet for updated balances."
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
