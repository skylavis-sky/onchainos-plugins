use anyhow::Result;
use clap::Args;
use crate::{config, onchainos, rpc};

#[derive(Args, Debug)]
pub struct WithdrawArgs {
    /// Aura pool ID (pid) to withdraw from
    #[arg(long)]
    pub pool_id: u64,

    /// Amount of BPT to withdraw (in token units, e.g. 1.5)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address override
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(args: WithdrawArgs, chain_id: u64, dry_run: bool) -> Result<()> {
    let amount_raw = (args.amount * 1e18) as u128;
    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    if dry_run {
        // withdrawAndUnwrap(uint256 amount, bool claim) selector: 0xc32e7202
        // claim=false (handle rewards separately)
        let amount_hex = format!("{:064x}", amount_raw);
        let claim_hex = "0000000000000000000000000000000000000000000000000000000000000000";
        let calldata = format!("0xc32e7202{}{}", amount_hex, claim_hex);

        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "action": "withdrawAndUnwrap",
            "pool_id": args.pool_id,
            "amount": args.amount,
            "calldata": calldata,
            "note": "call goes to BaseRewardPool (crv_rewards) address for pool, fetched from Booster.poolInfo(pid). ask user to confirm."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let wallet = match &args.from {
        Some(addr) => addr.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // Fetch pool info to get BaseRewardPool address
    let (_lp_token, crv_rewards, shutdown) =
        rpc::booster_pool_info(config::BOOSTER, args.pool_id).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch pool info for pid {}: {}", args.pool_id, e))?;

    if shutdown {
        // Still allow withdrawal from shutdown pools
        eprintln!("Note: Pool {} is shut down. Proceeding with withdrawal.", args.pool_id);
    }

    // Check staked balance
    let staked = rpc::erc20_balance_of(&crv_rewards, &wallet).await?;
    if staked < amount_raw {
        anyhow::bail!(
            "Insufficient staked BPT in pool {}. Staked: {}, Requested: {}",
            args.pool_id,
            rpc::format_amount(staked, 18),
            args.amount
        );
    }

    // withdrawAndUnwrap(uint256 amount, bool claim=false) selector: 0xc32e7202
    let amount_hex = format!("{:064x}", amount_raw);
    let claim_hex = "0000000000000000000000000000000000000000000000000000000000000000";
    let calldata = format!("0xc32e7202{}{}", amount_hex, claim_hex);

    eprintln!("Withdrawing BPT from Aura pool {} BaseRewardPool (ask user to confirm)...", args.pool_id);
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
            "action": "withdraw",
            "pool_id": args.pool_id,
            "base_reward_pool": crv_rewards,
            "amount": args.amount,
            "wallet": wallet,
            "txHash": tx_hash,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
            "note": "Rewards NOT claimed atomically. Use claim-rewards to claim pending BAL+AURA."
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
