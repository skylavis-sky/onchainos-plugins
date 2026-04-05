use crate::config;
use crate::onchainos;
use clap::Args;

#[derive(Args)]
pub struct DepositArgs {
    /// Amount of ETH to deposit (e.g. 0.00005)
    #[arg(long)]
    pub amount: f64,

    /// Auto-compound to apxETH directly (false = receive pxETH)
    #[arg(long, default_value = "false")]
    pub compound: bool,

    /// Chain ID (only Ethereum mainnet supported)
    #[arg(long, default_value = "1")]
    pub chain: u64,

    /// Simulate without broadcasting
    #[arg(long)]
    pub dry_run: bool,
}

/// Deposit ETH to receive pxETH via PirexEth.deposit(address receiver, bool shouldCompound).
/// ⚠️ PirexEth contract is currently PAUSED — this operation will revert on-chain.
pub async fn run(args: DepositArgs) -> anyhow::Result<()> {
    let amt_wei = (args.amount * 1e18) as u128;

    // Dry-run guard must be before resolve_wallet
    if args.dry_run {
        // deposit(address,bool) — receiver + shouldCompound
        let compound_val: u128 = if args.compound { 1 } else { 0 };
        // Use zero address placeholder for dry-run receiver
        let zero_addr = "0000000000000000000000000000000000000000000000000000000000000000";
        let compound_hex = format!("{:064x}", compound_val);
        let calldata = format!("0x{}{}{}", config::SEL_PIREX_DEPOSIT, zero_addr, compound_hex);
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "dry_run": true,
                "warning": "PirexEth contract is currently PAUSED. On-chain execution will revert.",
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                },
                "calldata": calldata,
                "amount_eth": args.amount,
                "amount_wei": amt_wei.to_string(),
                "should_compound": args.compound,
                "contract": config::PIREXETH
            })
        );
        return Ok(());
    }

    // Check if PirexEth is paused before attempting on-chain call
    let paused_calldata = format!("0x{}", config::SEL_PAUSED);
    let paused = onchainos::eth_call(args.chain, config::PIREXETH, &paused_calldata)
        .map(|r| onchainos::decode_bool(&r))
        .unwrap_or(false);

    if paused {
        println!(
            "{}",
            serde_json::json!({
                "ok": false,
                "error": "PirexEth protocol is currently PAUSED. ETH deposits are not available. The apxETH vault (stake/redeem pxETH) remains operational.",
                "protocol_status": "paused",
                "contract": config::PIREXETH,
                "suggestion": "Use 'dinero-pxeth rates' to check current apxETH yield, or 'dinero-pxeth stake' if you already hold pxETH."
            })
        );
        return Ok(());
    }

    let wallet = onchainos::resolve_wallet(args.chain)?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Ensure onchainos is logged in.");
    }

    if amt_wei == 0 {
        anyhow::bail!("Amount too small (rounds to 0 wei)");
    }

    // deposit(address receiver, bool shouldCompound)
    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);
    let compound_val: u128 = if args.compound { 1 } else { 0 };
    let compound_hex = format!("{:064x}", compound_val);
    let calldata = format!("0x{}{}{}", config::SEL_PIREX_DEPOSIT, wallet_padded, compound_hex);

    let result = onchainos::wallet_contract_call(
        args.chain,
        config::PIREXETH,
        &calldata,
        Some(&wallet),
        Some(amt_wei),
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
                "action": "deposit ETH → pxETH",
                "amount_eth": args.amount,
                "amount_wei": amt_wei.to_string(),
                "should_compound": args.compound,
                "from": wallet,
                "contract": config::PIREXETH,
                "explorer": format!("https://etherscan.io/tx/{}", tx_hash)
            }
        })
    );
    Ok(())
}
