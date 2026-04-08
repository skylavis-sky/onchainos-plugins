use clap::Args;
use crate::onchainos;
use crate::config::DEFAULT_SLIPPAGE_PCT;

#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Source token mint address (or 11111111111111111111111111111111 for native SOL)
    #[arg(long)]
    pub from_token: String,

    /// Destination token mint address
    #[arg(long)]
    pub to_token: String,

    /// Human-readable input amount (e.g. "1.5" for 1.5 SOL)
    #[arg(long)]
    pub amount: String,

    /// Slippage tolerance in percent (e.g. "0.5" for 0.5%). Defaults to auto-slippage.
    #[arg(long)]
    pub slippage: Option<f64>,

    /// Wallet address (Solana pubkey). If omitted, uses the currently logged-in wallet.
    #[arg(long)]
    pub wallet: Option<String>,
}

pub async fn execute(args: &SwapArgs, dry_run: bool) -> anyhow::Result<()> {
    // dry_run: show quote instead of executing swap
    if dry_run {
        let quote_result = onchainos::dex_quote_solana(
            &args.from_token,
            &args.to_token,
            &args.amount,
        )?;
        let output = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "message": "Dry run: showing quote only. No transaction submitted.",
            "from_token": args.from_token,
            "to_token": args.to_token,
            "amount": args.amount,
            "quote": quote_result,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Resolve wallet address AFTER dry_run guard
    let wallet = if let Some(w) = &args.wallet {
        w.clone()
    } else {
        onchainos::resolve_wallet_solana().map_err(|e| {
            anyhow::anyhow!("Cannot resolve wallet address. Pass --wallet <address> or log in via onchainos.\nError: {e}")
        })?
    };

    if wallet.is_empty() {
        anyhow::bail!("Wallet address is empty. Pass --wallet <address> or log in via onchainos.");
    }

    // Build slippage string
    let slippage_str = args
        .slippage
        .map(|s| s.to_string())
        .unwrap_or_else(|| DEFAULT_SLIPPAGE_PCT.to_string());

    // Execute swap via onchainos swap execute
    // NOTE: Solana does NOT need --force flag
    let result = onchainos::dex_swap_execute_solana(
        &args.from_token,
        &args.to_token,
        &args.amount,
        &wallet,
        Some(&slippage_str),
    )?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let ok = result["ok"].as_bool().unwrap_or(false);

    let output = serde_json::json!({
        "ok": ok,
        "from_token": args.from_token,
        "to_token": args.to_token,
        "amount": args.amount,
        "wallet": wallet,
        "tx_hash": tx_hash,
        "explorer_url": if tx_hash != "pending" && !tx_hash.is_empty() {
            format!("https://solscan.io/tx/{}", tx_hash)
        } else {
            String::new()
        },
        "raw_result": result,
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
