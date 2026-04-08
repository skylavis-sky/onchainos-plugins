use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

use pumpfun::{
    common::types::{Cluster, PriorityFee},
    PumpFun,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    transaction::VersionedTransaction,
};

use crate::config::{
    DEFAULT_PRIORITY_FEE_UNIT_LIMIT, DEFAULT_PRIORITY_FEE_UNIT_PRICE, DEFAULT_RPC_URL,
    DEFAULT_SLIPPAGE_BPS, PUMPFUN_PROGRAM_ID,
};
use crate::onchainos;

#[derive(Args, Debug)]
pub struct BuyArgs {
    /// Token mint address (base58)
    #[arg(long)]
    pub mint: String,

    /// SOL amount in lamports (e.g. 100000000 = 0.1 SOL)
    #[arg(long)]
    pub sol_amount: u64,

    /// Slippage tolerance in basis points (default: 100 = 1%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// Compute unit limit for priority fee (default: 200000)
    #[arg(long, default_value_t = DEFAULT_PRIORITY_FEE_UNIT_LIMIT)]
    pub priority_fee_unit_limit: u32,

    /// Micro-lamports per compute unit (default: 1000)
    #[arg(long, default_value_t = DEFAULT_PRIORITY_FEE_UNIT_PRICE)]
    pub priority_fee_unit_price: u64,

    /// Solana RPC URL (overrides HELIUS_RPC_URL env var and default)
    #[arg(long)]
    pub rpc_url: Option<String>,
}

#[derive(Serialize, Debug)]
struct BuyOutput {
    ok: bool,
    mint: String,
    sol_amount_lamports: u64,
    slippage_bps: u64,
    tx_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    graduated_warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
}

pub async fn execute(args: &BuyArgs, dry_run: bool) -> Result<()> {
    // dry_run guard: return early before resolving wallet
    if dry_run {
        let output = BuyOutput {
            ok: true,
            mint: args.mint.clone(),
            sol_amount_lamports: args.sol_amount,
            slippage_bps: args.slippage_bps,
            tx_hash: String::new(),
            graduated_warning: None,
            dry_run: Some(true),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let rpc_url = args
        .rpc_url
        .clone()
        .or_else(|| std::env::var("HELIUS_RPC_URL").ok())
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    let mint: Pubkey = args
        .mint
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid mint address '{}': {}", args.mint, e))?;

    // Resolve wallet address after dry_run guard
    let payer_pubkey_str = onchainos::resolve_wallet_solana()?;
    let payer_pubkey: Pubkey = payer_pubkey_str
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid payer pubkey from onchainos: {e}"))?;

    let placeholder_keypair = Arc::new(Keypair::new());
    let commitment = CommitmentConfig::confirmed();

    let priority_fee = PriorityFee {
        unit_limit: Some(args.priority_fee_unit_limit),
        unit_price: Some(args.priority_fee_unit_price),
    };

    let ws_url = derive_ws_url(&rpc_url);
    let cluster = Cluster::new(rpc_url, ws_url, commitment, priority_fee);
    let pumpfun = PumpFun::new(placeholder_keypair, cluster);

    // Check bonding curve status before proceeding
    let curve = pumpfun
        .get_bonding_curve_account(&mint)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch bonding curve: {e}"))?;

    if curve.complete {
        // Graduated token — redirect to DEX swap
        let output = BuyOutput {
            ok: false,
            mint: args.mint.clone(),
            sol_amount_lamports: args.sol_amount,
            slippage_bps: args.slippage_bps,
            tx_hash: String::new(),
            graduated_warning: Some(
                "Token has graduated from bonding curve. Use: onchainos dex swap execute --chain 501"
                    .to_string(),
            ),
            dry_run: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build buy instructions via pumpfun crate
    let mut all_instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

    let buy_instructions = pumpfun
        .get_buy_instructions(
            mint,
            args.sol_amount,
            Some(true), // track_volume
            Some(args.slippage_bps),
        )
        .await
        .map_err(|e| anyhow::anyhow!("get_buy_instructions failed: {e}"))?;
    all_instructions.extend(buy_instructions);

    // Fetch latest blockhash immediately before building tx — expires in ~60s
    let blockhash = pumpfun
        .rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get latest blockhash: {e}"))?;

    // Build unsigned VersionedTransaction — onchainos will provide the payer signature
    let message = v0::Message::try_compile(&payer_pubkey, &all_instructions, &[], blockhash)
        .map_err(|e| anyhow::anyhow!("Failed to compile message: {e}"))?;
    let tx = VersionedTransaction {
        signatures: vec![solana_sdk::signature::Signature::default()],
        message: VersionedMessage::V0(message),
    };

    // Serialize to base64
    let serialized = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        bincode::serialize(&tx).map_err(|e| anyhow::anyhow!("bincode serialize failed: {e}"))?,
    );

    // Submit immediately — blockhash expires in ~60s
    let result =
        onchainos::wallet_contract_call_solana(PUMPFUN_PROGRAM_ID, &serialized, false).await?;

    // Propagate onchainos error if broadcast failed
    let ok = result["ok"].as_bool().unwrap_or(false);
    if !ok {
        let err = result["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos broadcast failed: {err}\nFull response: {result}");
    }

    let tx_hash = onchainos::extract_tx_hash(&result).to_string();

    let output = BuyOutput {
        ok,
        mint: args.mint.clone(),
        sol_amount_lamports: args.sol_amount,
        slippage_bps: args.slippage_bps,
        tx_hash,
        graduated_warning: None,
        dry_run: None,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Derive a WebSocket URL from an HTTP RPC URL.
fn derive_ws_url(http_url: &str) -> String {
    http_url
        .replace("https://", "wss://")
        .replace("http://", "ws://")
}
