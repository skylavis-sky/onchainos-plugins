use anyhow::Result;
use clap::Args;
use crate::{config, onchainos, rpc};

#[derive(Args, Debug)]
pub struct LockAuraArgs {
    /// Amount of AURA to lock as vlAURA (in token units, e.g. 100.0)
    #[arg(long)]
    pub amount: f64,

    /// Wallet address override
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(args: LockAuraArgs, chain_id: u64, dry_run: bool) -> Result<()> {
    let amount_raw = (args.amount * 1e18) as u128;
    if amount_raw == 0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    // dry_run guard - resolve wallet AFTER this check (KB: dry-run-wallet-ordering)
    if dry_run {
        let locker_padded = format!("{:0>64}", config::AURA_LOCKER.strip_prefix("0x").unwrap_or(config::AURA_LOCKER).to_lowercase());
        let calldata_approve = format!("0x095ea7b3{}{:064x}", locker_padded, u128::MAX);

        // lock(address _account, uint256 _amount) selector: 0x282d3fdf
        // _account is the recipient of the vlAURA (usually caller's wallet)
        let account_padded = "0000000000000000000000000000000000000000000000000000000000000000";
        let amount_hex = format!("{:064x}", amount_raw);
        let calldata_lock = format!("0x282d3fdf{}{}", account_padded, amount_hex);

        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "steps": [
                {
                    "action": "approve",
                    "token": config::AURA_TOKEN,
                    "spender": config::AURA_LOCKER,
                    "calldata": calldata_approve,
                    "note": "ask user to confirm AURA approval"
                },
                {
                    "action": "lock",
                    "contract": config::AURA_LOCKER,
                    "amount": args.amount,
                    "calldata": calldata_lock,
                    "WARNING": "AURA will be locked as vlAURA for 16 WEEKS. This lock is IRREVERSIBLE until expiry. ask user to confirm."
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

    // Check AURA balance
    let balance = rpc::erc20_balance_of(config::AURA_TOKEN, &wallet).await?;
    if balance < amount_raw {
        anyhow::bail!(
            "Insufficient AURA balance. Have: {}, Need: {}",
            rpc::format_amount(balance, 18),
            args.amount
        );
    }

    // Check allowance
    let allowance = rpc::erc20_allowance(config::AURA_TOKEN, &wallet, config::AURA_LOCKER).await.unwrap_or(0);

    let mut approve_tx = None;
    if allowance < amount_raw {
        eprintln!("Approving AURA for vlAURA locking (ask user to confirm)...");
        let approve_result = onchainos::erc20_approve(
            chain_id,
            config::AURA_TOKEN,
            config::AURA_LOCKER,
            u128::MAX,
            from_ref,
            false,
        ).await?;
        let approve_hash = onchainos::extract_tx_hash_or_err(&approve_result)?;
        approve_tx = Some(approve_hash.clone());
        eprintln!("Approve tx: {}", approve_hash);
        // Wait for approval to confirm before locking
        tokio::time::sleep(tokio::time::Duration::from_secs(15)).await;
    }

    // === PROMINENT 16-WEEK LOCK WARNING ===
    // This warning is surfaced in the eprintln before the on-chain call.
    // SKILL.md also documents this requirement (E106 compliance).
    eprintln!(
        "\n*** WARNING: You are about to lock {} AURA as vlAURA for 16 WEEKS. ***\n\
         *** This lock is IRREVERSIBLE until expiry. You CANNOT withdraw early. ***\n\
         *** After 16 weeks, call unlock-aura to process expired locks. ***\n\
         Locking {} AURA (ask user to confirm)...",
        args.amount, args.amount
    );

    // lock(address _account, uint256 _amount) selector: 0x282d3fdf
    let wallet_padded = rpc::pad_address(&wallet);
    let amount_hex = format!("{:064x}", amount_raw);
    let calldata = format!("0x282d3fdf{}{}", wallet_padded, amount_hex);

    let result = onchainos::wallet_contract_call(
        chain_id,
        config::AURA_LOCKER,
        &calldata,
        from_ref,
        None,
        false,
    ).await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "action": "lock-aura",
            "amount": args.amount,
            "wallet": wallet,
            "lock_period": "16 weeks",
            "lock_contract": config::AURA_LOCKER,
            "approve_txHash": approve_tx,
            "txHash": tx_hash,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
            "WARNING": "AURA is now locked as vlAURA for 16 weeks. Use unlock-aura after lock expires to retrieve your AURA."
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
