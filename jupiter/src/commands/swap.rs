/// swap: Execute a token swap on Jupiter via onchainos.
///
/// Flow:
///   1. dry_run guard — return early before any wallet resolution
///   2. Resolve Solana wallet address via onchainos
///   3. GET https://api.jup.ag/swap/v2/order → quote + base64 unsigned tx
///   4. Convert base64 -> bytes -> base58
///   5. onchainos wallet contract-call --unsigned-tx <base58> --to JUP6... --chain 501 --force
///
/// NOTE: Solana blockhash expires in ~60s; broadcast immediately after receiving the tx.
use anyhow::Result;
use clap::Args;

use crate::api;
use crate::config::{from_raw_amount, resolve_mint, to_raw_amount, DEFAULT_SLIPPAGE_BPS};
use crate::onchainos;

#[derive(Args, Debug)]
pub struct SwapArgs {
    /// Input token symbol (SOL, USDC, USDT) or raw mint address
    #[arg(long)]
    pub input_mint: String,

    /// Output token symbol (SOL, USDC, USDT) or raw mint address
    #[arg(long)]
    pub output_mint: String,

    /// Input amount in UI units (e.g. 0.1 for 0.1 SOL)
    #[arg(long)]
    pub amount: f64,

    /// Slippage tolerance in basis points (default: 50 = 0.5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u32,

    /// Simulate without broadcasting on-chain (no onchainos call)
    #[arg(long)]
    pub dry_run: bool,
}

pub async fn execute(args: &SwapArgs) -> Result<()> {
    // dry_run guard — must come BEFORE resolve_wallet_solana()
    if args.dry_run {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "inputMint": resolve_mint(&args.input_mint),
                "outputMint": resolve_mint(&args.output_mint),
                "amount": args.amount,
                "slippageBps": args.slippage_bps,
                "note": "dry_run=true: tx not built or broadcast"
            }))?
        );
        return Ok(());
    }

    let input_mint = resolve_mint(&args.input_mint).to_string();
    let output_mint = resolve_mint(&args.output_mint).to_string();
    let raw_amount = to_raw_amount(args.amount, &input_mint);

    // Resolve wallet address from onchainos
    let wallet = onchainos::resolve_wallet_solana()?;

    // GET /swap/v2/order — returns quote AND unsigned tx in one call
    let resp = api::get_order(
        &input_mint,
        &output_mint,
        raw_amount,
        args.slippage_bps,
        Some(&wallet),
    )
    .await?;

    // Extract transaction (base64 encoded)
    let tx_base64 = resp["transaction"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No 'transaction' field in Jupiter API response: {}", resp))?;

    // Extract output amount for display
    let out_amount_raw = resp["outAmount"]
        .as_str()
        .and_then(|s| s.parse::<u64>().ok())
        .or_else(|| resp["outAmount"].as_u64())
        .unwrap_or(0);
    let out_amount_ui = from_raw_amount(out_amount_raw, &output_mint);

    let price_impact = resp["priceImpactPct"]
        .as_str()
        .map(|s| s.to_string())
        .unwrap_or_else(|| resp["priceImpactPct"].to_string());

    // Broadcast via onchainos — converts base64 -> base58 internally
    let broadcast = onchainos::wallet_contract_call_solana(tx_base64, false)?;
    let tx_hash = onchainos::extract_tx_hash(&broadcast);

    let output = serde_json::json!({
        "ok": true,
        "txHash": tx_hash,
        "input": format!("{} {}", args.amount, args.input_mint.to_uppercase()),
        "output_estimate": format!("{:.6} {}", out_amount_ui, args.output_mint.to_uppercase()),
        "price_impact": format!("{}%", price_impact),
        "slippage_bps": args.slippage_bps,
        "wallet": wallet
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
