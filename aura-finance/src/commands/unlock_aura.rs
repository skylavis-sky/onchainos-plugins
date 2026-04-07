use anyhow::Result;
use clap::Args;
use alloy_sol_types::{sol, SolCall};
use crate::{config, onchainos};

sol! {
    function processExpiredLocks(bool _relock) external;
}

#[derive(Args, Debug)]
pub struct UnlockAuraArgs {
    /// Re-lock the AURA after unlocking instead of withdrawing
    #[arg(long, default_value = "false")]
    pub relock: bool,

    /// Wallet address override
    #[arg(long)]
    pub from: Option<String>,
}

pub async fn run(args: UnlockAuraArgs, chain_id: u64, dry_run: bool) -> Result<()> {
    if dry_run {
        let call = processExpiredLocksCall { _relock: args.relock };
        let calldata = format!("0x{}", hex::encode(call.abi_encode()));
        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": { "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000" },
            "contract": config::AURA_LOCKER,
            "calldata": calldata,
            "relock": args.relock,
            "note": "processExpiredLocks will revert if there are no expired vlAURA locks. ask user to confirm."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let wallet = match &args.from {
        Some(addr) => addr.clone(),
        None => onchainos::resolve_wallet(chain_id)?,
    };

    // processExpiredLocks(bool _relock) selector: 0x312ff839
    // _relock=false = withdraw AURA; _relock=true = extend lock
    let call = processExpiredLocksCall { _relock: args.relock };
    let calldata = format!("0x{}", hex::encode(call.abi_encode()));

    eprintln!("Processing expired vlAURA locks (ask user to confirm)...");
    let result = onchainos::wallet_contract_call(
        chain_id,
        config::AURA_LOCKER,
        &calldata,
        Some(&wallet),
        None,
        false,
    ).await?;

    let tx_hash = onchainos::extract_tx_hash_or_err(&result)?;

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "action": "unlock-aura",
            "wallet": wallet,
            "relock": args.relock,
            "contract": config::AURA_LOCKER,
            "txHash": tx_hash,
            "explorer": format!("https://etherscan.io/tx/{}", tx_hash),
            "note": if args.relock {
                "Expired vlAURA locks have been re-locked for another 16 weeks."
            } else {
                "Expired vlAURA locks processed. AURA tokens returned to your wallet."
            }
        }
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
