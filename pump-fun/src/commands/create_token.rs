use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::sync::Arc;

use pumpfun::{
    common::types::{Cluster, PriorityFee},
    utils::{create_token_metadata, CreateTokenMetadata},
    PumpFun,
};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    message::{v0, VersionedMessage},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    transaction::VersionedTransaction,
};

use crate::config::{
    DEFAULT_PRIORITY_FEE_UNIT_LIMIT, DEFAULT_PRIORITY_FEE_UNIT_PRICE, DEFAULT_RPC_URL,
    DEFAULT_SLIPPAGE_BPS, PUMPFUN_PROGRAM_ID,
};
use crate::onchainos;

#[derive(Args, Debug)]
pub struct CreateTokenArgs {
    /// Token name (e.g. "Moon Cat")
    #[arg(long)]
    pub name: String,

    /// Token symbol/ticker (e.g. "MCAT")
    #[arg(long)]
    pub symbol: String,

    /// Token description
    #[arg(long)]
    pub description: String,

    /// Path to image file or IPFS URI for token image
    #[arg(long)]
    pub image_path: String,

    /// Twitter/X URL (optional)
    #[arg(long)]
    pub twitter: Option<String>,

    /// Telegram URL (optional)
    #[arg(long)]
    pub telegram: Option<String>,

    /// Website URL (optional)
    #[arg(long)]
    pub website: Option<String>,

    /// SOL in lamports to buy immediately after create (0 = no initial buy)
    #[arg(long, default_value_t = 0)]
    pub initial_buy_sol: u64,

    /// Slippage for initial buy in basis points (default: 100 = 1%)
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
struct CreateTokenOutput {
    ok: bool,
    mint_address: String,
    name: String,
    symbol: String,
    initial_buy_sol_lamports: u64,
    tx_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
}

pub async fn execute(args: &CreateTokenArgs, dry_run: bool) -> Result<()> {
    // Generate fresh mint keypair at runtime — required by pump.fun program
    let mint_keypair = Keypair::new();
    let mint_pubkey = mint_keypair.pubkey();

    // dry_run guard: return early before resolving wallet
    if dry_run {
        let output = CreateTokenOutput {
            ok: true,
            mint_address: mint_pubkey.to_string(),
            name: args.name.clone(),
            symbol: args.symbol.clone(),
            initial_buy_sol_lamports: args.initial_buy_sol,
            tx_hash: String::new(),
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

    let metadata = CreateTokenMetadata {
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        description: args.description.clone(),
        file: args.image_path.clone(),
        twitter: args.twitter.clone(),
        telegram: args.telegram.clone(),
        website: args.website.clone(),
    };

    // Upload metadata to IPFS via pumpfun utils
    let ipfs = create_token_metadata(metadata)
        .await
        .map_err(|e| anyhow::anyhow!("upload_metadata failed: {e}"))?;

    // Build instructions
    let mut all_instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

    // Add create instruction
    let create_ix = pumpfun.get_create_instruction(&mint_keypair, ipfs);
    all_instructions.push(create_ix);

    // Optionally add initial buy instruction
    if args.initial_buy_sol > 0 {
        let buy_instructions = pumpfun
            .get_buy_instructions(
                mint_pubkey,
                args.initial_buy_sol,
                Some(true), // track_volume
                Some(args.slippage_bps),
            )
            .await
            .map_err(|e| anyhow::anyhow!("get_buy_instructions failed: {e}"))?;
        all_instructions.extend(buy_instructions);
    }

    // Fetch latest blockhash immediately before building tx — expires in ~60s
    let blockhash = pumpfun
        .rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get latest blockhash: {e}"))?;

    // Build VersionedTransaction — mint_keypair must co-sign (pump.fun requires it)
    // onchainos will provide the payer signature
    let message =
        v0::Message::try_compile(&payer_pubkey, &all_instructions, &[], blockhash)
            .map_err(|e| anyhow::anyhow!("Failed to compile message: {e}"))?;

    // Sign with mint keypair only; payer signing is handled by onchainos
    let tx = VersionedTransaction::try_new(VersionedMessage::V0(message), &[&mint_keypair])
        .map_err(|e| anyhow::anyhow!("Failed to sign transaction with mint keypair: {e}"))?;

    // Serialize to base64
    let serialized = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        bincode::serialize(&tx).map_err(|e| anyhow::anyhow!("bincode serialize failed: {e}"))?,
    );

    // Submit immediately — blockhash expires in ~60s
    let result =
        onchainos::wallet_contract_call_solana(PUMPFUN_PROGRAM_ID, &serialized, false).await?;

    let tx_hash = onchainos::extract_tx_hash(&result).to_string();

    let output = CreateTokenOutput {
        ok: true,
        mint_address: mint_pubkey.to_string(),
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        initial_buy_sol_lamports: args.initial_buy_sol,
        tx_hash,
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
