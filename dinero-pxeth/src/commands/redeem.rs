use crate::config;
use crate::onchainos;
use clap::Args;

#[derive(Args)]
pub struct RedeemArgs {
    /// Amount of apxETH to redeem (e.g. 0.00005)
    #[arg(long)]
    pub amount: f64,

    /// Chain ID (only Ethereum mainnet supported)
    #[arg(long, default_value = "1")]
    pub chain: u64,

    /// Simulate without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

/// Redeem apxETH to receive pxETH via ERC-4626 redeem(shares, receiver, owner).
pub async fn run(args: RedeemArgs) -> anyhow::Result<()> {
    let shares_wei = (args.amount * 1e18) as u128;

    // Dry-run guard must be before resolve_wallet
    if args.dry_run {
        // redeem(uint256,address,address) — use zero address placeholder for dry-run
        let shares_hex = format!("{:064x}", shares_wei);
        let zero_addr = "0000000000000000000000000000000000000000000000000000000000000000";
        let calldata = format!(
            "0x{}{}{}{}",
            config::SEL_REDEEM,
            shares_hex,
            zero_addr,
            zero_addr
        );
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "calldata": calldata,
                "amount_apxeth": args.amount,
                "amount_wei": shares_wei.to_string(),
                "contract": config::APXETH_VAULT
            })
        );
        return Ok(());
    }

    let wallet = onchainos::resolve_wallet(args.chain)?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Ensure onchainos is logged in.");
    }

    if shares_wei == 0 {
        anyhow::bail!("Amount too small (rounds to 0 wei)");
    }

    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);
    let shares_hex = format!("{:064x}", shares_wei);

    // redeem(uint256 shares, address receiver, address owner)
    let calldata = format!(
        "0x{}{}{}{}",
        config::SEL_REDEEM,
        shares_hex,
        wallet_padded, // receiver
        wallet_padded  // owner
    );

    let result = onchainos::wallet_contract_call(
        args.chain,
        config::APXETH_VAULT,
        &calldata,
        Some(&wallet),
        None,
        false,
    )
    .await?;

    let tx_hash = onchainos::extract_tx_hash(&result);

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "txHash": tx_hash,
                "action": "redeem apxETH → pxETH",
                "amount_apxeth": args.amount,
                "amount_wei": shares_wei.to_string(),
                "from": wallet,
                "contract": config::APXETH_VAULT,
                "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
            }
        })
    );
    Ok(())
}
