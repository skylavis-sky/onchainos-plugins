use anyhow::Result;
use clap::Args;
use crate::{config, onchainos, rpc};

#[derive(Args, Debug)]
pub struct DepositArgs {
    /// Aura pool ID (pid) to deposit into
    #[arg(long)]
    pub pool_id: u64,

    /// Amount of BPT to deposit (in token units, e.g. 1.5)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address override
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(args: DepositArgs, chain_id: u64, dry_run: bool) -> Result<()> {
    let amount_raw = (args.amount * 1e18) as u128;
    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // dry_run guard - resolve wallet AFTER this check (KB: dry-run-wallet-ordering)
    if dry_run {
        let booster_padded = format!("{:0>64}", config::BOOSTER.strip_prefix("0x").unwrap_or(config::BOOSTER).to_lowercase());
        let calldata_approve = format!("0x095ea7b3{}{:064x}", booster_padded, u128::MAX);

        // deposit(uint256 _pid, uint256 _amount, bool _stake) selector: 0x43a0d066
        // _stake = true (0x01) to start reward accrual immediately
        let pid_hex = format!("{:064x}", args.pool_id);
        let amount_hex = format!("{:064x}", amount_raw);
        let stake_hex = "0000000000000000000000000000000000000000000000000000000000000001";
        let calldata_deposit = format!("0x43a0d066{}{}{}", pid_hex, amount_hex, stake_hex);

        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "steps": [
                {
                    "action": "approve",
                    "token": "<bpt_token_for_pool>",
                    "spender": config::BOOSTER,
                    "calldata": calldata_approve,
                    "note": "ask user to confirm BPT approval"
                },
                {
                    "action": "deposit",
                    "contract": config::BOOSTER,
                    "pool_id": args.pool_id,
                    "amount": args.amount,
                    "calldata": calldata_deposit,
                    "note": "_stake=true enables immediate reward accrual. ask user to confirm deposit."
                }
            ]
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let wallet = match &args.from {
        Some(addr) => addr.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };
    let from_ref: Option<&str> = Some(&wallet);

    // Fetch pool info to get BPT (lptoken) address
    let (lp_token, _crv_rewards, shutdown) =
        rpc::booster_pool_info(config::BOOSTER, args.pool_id).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch pool info for pid {}: {}", args.pool_id, e))?;

    if shutdown {
        anyhow::bail!("Pool {} is shut down and no longer accepting deposits.", args.pool_id);
    }

    // Check BPT balance
    let balance = rpc::erc20_balance_of(&lp_token, &wallet).await?;
    if balance == 0 {
        anyhow::bail!(
            "No BPT (Balancer Pool Tokens) found for pool {} (lptoken: {}). \
            You must first add liquidity on Balancer to receive BPT before depositing into Aura.",
            args.pool_id, lp_token
        );
    }
    if balance < amount_raw {
        anyhow::bail!(
            "Insufficient BPT balance. Have: {}, Need: {}",
            rpc::format_amount(balance, 18),
            args.amount
        );
    }

    // Check allowance
    let allowance = rpc::erc20_allowance(&lp_token, &wallet, config::BOOSTER).await.unwrap_or(0);

    let mut approve_tx = None;
    if allowance < amount_raw {
        eprintln!("Approving BPT for Aura Booster (ask user to confirm)...");
        let approve_result = onchainos::erc20_approve(
            chain_id,
            &lp_token,
            config::BOOSTER,
            u128::MAX,
            from_ref,
            false,
        ).await?;
        let approve_hash = onchainos::extract_tx_hash_or_err(&approve_result)?;
        approve_tx = Some(approve_hash.clone());
        eprintln!("Approve tx: {}", approve_hash);
        // Wait for approval to confirm before deposit (KB: symbiotic deposit-two-tx-delay)
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }

    // deposit(uint256 _pid, uint256 _amount, bool _stake=true)
    // selector: 0x43a0d066
    let pid_hex = format!("{:064x}", args.pool_id);
    let amount_hex = format!("{:064x}", amount_raw);
    let stake_hex = "0000000000000000000000000000000000000000000000000000000000000001";
    let calldata = format!("0x43a0d066{}{}{}", pid_hex, amount_hex, stake_hex);

    eprintln!("Depositing BPT into Aura pool {} (ask user to confirm)...", args.pool_id);
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::BOOSTER,
        &calldata,
        from_ref,
        None,
        false,
    ).await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "action": "deposit",
            "pool_id": args.pool_id,
            "lp_token": lp_token,
            "amount": args.amount,
            "wallet": wallet,
            "stake": true,
            "approve_txHash": approve_tx,
            "txHash": tx_hash,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
            "note": "BPT staked immediately into BaseRewardPool (_stake=true). Rewards will start accruing."
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
