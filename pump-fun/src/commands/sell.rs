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
pub struct SellArgs {
    /// Token mint address (base58)
    #[arg(long)]
    pub mint: String,

    /// Token units to sell; omit to sell all tokens
    #[arg(long)]
    pub token_amount: Option<u64>,

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
struct SellOutput {
    ok: bool,
    mint: String,
    token_amount: Option<u64>,
    slippage_bps: u64,
    sell_all: bool,
    tx_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    graduated_warning: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
}

pub async fn execute(args: &SellArgs, dry_run: bool) -> Result<()> {
    // dry_run guard: return early before resolving wallet
    if dry_run {
        let output = SellOutput {
            ok: true,
            mint: args.mint.clone(),
            token_amount: args.token_amount,
            slippage_bps: args.slippage_bps,
            sell_all: args.token_amount.is_none(),
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

    // Check bonding curve status
    let curve = pumpfun
        .get_bonding_curve_account(&mint)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to fetch bonding curve: {e}"))?;

    if curve.complete {
        let output = SellOutput {
            ok: false,
            mint: args.mint.clone(),
            token_amount: args.token_amount,
            slippage_bps: args.slippage_bps,
            sell_all: args.token_amount.is_none(),
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

    // Build sell instructions via pumpfun crate
    // token_amount = None means sell all
    let mut all_instructions = PumpFun::get_priority_fee_instructions(&priority_fee);

    let sell_instructions = pumpfun
        .get_sell_instructions(mint, args.token_amount, Some(args.slippage_bps))
        .await
        .map_err(|e| anyhow::anyhow!("get_sell_instructions failed: {e}"))?;
    all_instructions.extend(sell_instructions);

    // Fetch latest blockhash immediately before building tx — expires in ~60s
    let blockhash = pumpfun
        .rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to get latest blockhash: {e}"))?;

    // Build unsigned VersionedTransaction
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

    let tx_hash = onchainos::extract_tx_hash(&result).to_string();

    let output = SellOutput {
        ok: true,
        mint: args.mint.clone(),
        token_amount: args.token_amount,
        slippage_bps: args.slippage_bps,
        sell_all: args.token_amount.is_none(),
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
