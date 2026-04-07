// swap-lst: swap between two validator LSTs via Sanctum Router.
//
// Flow:
//   1. Get quote (GET /v2/swap/quote)
//   2. Confirm with user (unless --dry-run)
//   3. Resolve wallet
//   4. POST /v1/swap → base64 tx
//   5. Convert base64 → base58
//   6. onchainos wallet contract-call --chain 501 --to <SPOOL_PROGRAM> --unsigned-tx <base58> --force

use anyhow::Result;
use clap::Args;
use serde_json::Value;

use crate::api;
use crate::config::{self, LST_DECIMALS, SPOOL_PROGRAM};
use crate::onchainos;

#[derive(Args)]
pub struct SwapLstArgs {
    /// Input LST symbol (e.g. jitoSOL) or mint address
    #[arg(long)]
    pub from: String,

    /// Output LST symbol (e.g. bSOL) or mint address
    #[arg(long)]
    pub to: String,

    /// Amount to swap in UI units (e.g. 0.1 for 0.1 jitoSOL)
    #[arg(long)]
    pub amount: f64,

    /// Slippage tolerance in percent (default: 0.5)
    #[arg(long, default_value_t = 0.5)]
    pub slippage: f64,

    /// Chain ID (must be 501 for Solana)
    #[arg(long, default_value_t = 501)]
    pub chain: u64,

    /// Preview without broadcasting
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

pub async fn run(args: SwapLstArgs) -> Result<Value> {
    if args.chain != config::SOLANA_CHAIN_ID {
        anyhow::bail!("sanctum-validator-lst only supports Solana (chain 501)");
    }
    if args.amount <= 0.0 {
        anyhow::bail!("Amount must be positive");
    }

    let input_mint = config::resolve_mint(&args.from).to_string();
    let output_mint = config::resolve_mint(&args.to).to_string();
    let amount_atomics = api::ui_to_atomics(args.amount, LST_DECIMALS);

    let client = reqwest::Client::new();

    // Step 1: Get quote
    let quote = api::get_swap_quote(&client, &input_mint, &output_mint, amount_atomics).await?;

    let out_amount: u64 = quote.out_amount.parse().unwrap_or(0);
    let min_out = api::apply_slippage(out_amount, args.slippage);

    let out_ui = api::atomics_to_ui(out_amount, LST_DECIMALS);
    let min_out_ui = api::atomics_to_ui(min_out, LST_DECIMALS);
    let swap_src = quote.swap_src.clone();

    let preview = serde_json::json!({
        "operation": "swap-lst",
        "from": args.from,
        "to": args.to,
        "input_mint": input_mint,
        "output_mint": output_mint,
        "in_amount_ui": format!("{:.9}", args.amount),
        "expected_out_ui": format!("{:.9}", out_ui),
        "min_out_ui": format!("{:.9}", min_out_ui),
        "slippage_pct": args.slippage,
        "swap_src": swap_src,
        "note": "Ask user to confirm before broadcasting."
    });

    if args.dry_run {
        return Ok(serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": preview
        }));
    }

    // Step 2: Resolve wallet
    let wallet = onchainos::resolve_wallet_solana()?;
    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve Solana wallet. Make sure onchainos is logged in.");
    }

    // Step 3: POST /v1/swap — get base64 transaction
    // Use swap_src from quote (do not hardcode "SPool")
    let tx_b64 = api::execute_swap(
        &client,
        &input_mint,
        &output_mint,
        amount_atomics,
        min_out,
        &wallet,
        &swap_src,
    )
    .await?;

    // Step 4: Submit via onchainos (base64→base58 conversion is handled in wallet_contract_call_solana)
    let result = onchainos::wallet_contract_call_solana(SPOOL_PROGRAM, &tx_b64, false).await?;

    let tx_hash = onchainos::extract_tx_hash(&result)?;

    Ok(serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "operation": "swap-lst",
            "from": args.from,
            "to": args.to,
            "in_amount_ui": format!("{:.9}", args.amount),
            "expected_out_ui": format!("{:.9}", out_ui),
            "min_out_ui": format!("{:.9}", min_out_ui),
            "wallet": wallet,
            "swap_src": swap_src,
            "solscan": format!("https://solscan.io/tx/{}", tx_hash)
        }
    }))
}
