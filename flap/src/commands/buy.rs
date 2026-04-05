use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::abi::{decode_uint256_as_u8, encode_quote_exact_input, encode_swap_exact_input};
use crate::config::{DEFAULT_RPC_URL, DEFAULT_SLIPPAGE_BPS, PORTAL_ADDRESS};
use crate::onchainos;

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

#[derive(Args, Debug)]
pub struct BuyArgs {
    /// Token contract address to buy (0x-prefixed)
    #[arg(long)]
    pub token: String,

    /// BNB amount to spend in wei (e.g. 100000000000000000 = 0.1 BNB)
    #[arg(long)]
    pub bnb_amount: u128,

    /// Slippage tolerance in basis points (default: 500 = 5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// BSC RPC URL (default: public BSC node)
    #[arg(long)]
    pub rpc_url: Option<String>,
}

#[derive(Serialize, Debug)]
struct BuyOutput {
    ok: bool,
    token: String,
    bnb_amount_wei: String,
    min_tokens_out: String,
    expected_tokens_out: String,
    slippage_bps: u64,
    tx_hash: String,
    bscscan_tx_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

pub async fn execute(args: &BuyArgs, dry_run: bool) -> Result<()> {
    let rpc_url = args
        .rpc_url
        .clone()
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    let token = normalize_address(&args.token)?;

    // Check token status first
    let status_code = get_token_status(&rpc_url, &token).await?;
    if status_code == 4 {
        let output = BuyOutput {
            ok: false,
            token: token.clone(),
            bnb_amount_wei: args.bnb_amount.to_string(),
            min_tokens_out: "0".to_string(),
            expected_tokens_out: "0".to_string(),
            slippage_bps: args.slippage_bps,
            tx_hash: String::new(),
            bscscan_tx_url: String::new(),
            dry_run: None,
            warning: Some(
                "BONDING CURVE COMPLETE: This token has graduated to DEX. Use onchainos dex swap to trade."
                    .to_string(),
            ),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Get quote
    let quote_calldata = encode_quote_exact_input(ZERO_ADDRESS, &token, args.bnb_amount)?;
    let quote_hex = format!("0x{}", hex::encode(&quote_calldata));

    let expected_tokens_out = match onchainos::eth_call(&rpc_url, PORTAL_ADDRESS, &quote_hex).await {
        Ok(raw) => {
            let clean = raw.trim_start_matches("0x");
            if clean.len() >= 64 {
                let bytes = hex::decode(&clean[..64]).unwrap_or_default();
                crate::abi::decode_uint256_as_u128(&bytes)
            } else {
                0u128
            }
        }
        Err(_) => 0u128,
    };

    // Apply slippage: minOut = expectedOut * (10000 - slippage_bps) / 10000
    let min_tokens_out = if expected_tokens_out > 0 {
        expected_tokens_out
            .saturating_mul(10000 - args.slippage_bps as u128)
            / 10000
    } else {
        0u128
    };

    if dry_run {
        let output = BuyOutput {
            ok: true,
            token: token.clone(),
            bnb_amount_wei: args.bnb_amount.to_string(),
            min_tokens_out: min_tokens_out.to_string(),
            expected_tokens_out: expected_tokens_out.to_string(),
            slippage_bps: args.slippage_bps,
            tx_hash: String::new(),
            bscscan_tx_url: String::new(),
            dry_run: Some(true),
            warning: None,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build swapExactInput calldata
    // BNB buy: inputToken=address(0), outputToken=token, inputAmount=bnb_amount, permitData=empty
    let calldata = encode_swap_exact_input(
        ZERO_ADDRESS,
        &token,
        args.bnb_amount,
        min_tokens_out,
        &[],
    )?;
    let input_data = format!("0x{}", hex::encode(&calldata));

    // Execute via onchainos — must pass --amt = bnb_amount (payable)
    let result = onchainos::wallet_contract_call_evm(
        PORTAL_ADDRESS,
        &input_data,
        args.bnb_amount,
        false,
    )
    .await?;

    let ok = result["ok"].as_bool().unwrap_or(false);
    if !ok {
        let err = result["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("onchainos broadcast failed: {err}\nFull response: {result}");
    }

    let tx_hash = onchainos::extract_tx_hash(&result);
    let bscscan_tx_url = if tx_hash.is_empty() || tx_hash == "pending" {
        String::new()
    } else {
        format!("https://bscscan.com/tx/{}", tx_hash)
    };

    let output = BuyOutput {
        ok,
        token,
        bnb_amount_wei: args.bnb_amount.to_string(),
        min_tokens_out: min_tokens_out.to_string(),
        expected_tokens_out: expected_tokens_out.to_string(),
        slippage_bps: args.slippage_bps,
        tx_hash,
        bscscan_tx_url,
        dry_run: None,
        warning: None,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Quick status check — returns the TokenStatus code (0=Invalid, 1=Tradable, 4=DEX, 5=Staged).
async fn get_token_status(rpc_url: &str, token: &str) -> anyhow::Result<u8> {
    let calldata = crate::abi::encode_get_token_v8_safe(token)?;
    let calldata_hex = format!("0x{}", hex::encode(&calldata));
    let raw = onchainos::eth_call(rpc_url, PORTAL_ADDRESS, &calldata_hex).await?;
    let clean = raw.trim_start_matches("0x");
    // TokenStateV8Safe is returned directly (no outer ABI offset wrapper).
    // status is the first field at byte 0.
    if clean.len() < 64 {
        return Ok(0);
    }
    let bytes = hex::decode(clean)?;
    if bytes.len() < 32 {
        return Ok(0);
    }
    Ok(decode_uint256_as_u8(&bytes[0..32]))
}

fn normalize_address(addr: &str) -> anyhow::Result<String> {
    let clean = addr.trim_start_matches("0x");
    if clean.len() != 40 {
        anyhow::bail!("Invalid Ethereum address: '{}'", addr);
    }
    hex::decode(clean).map_err(|e| anyhow::anyhow!("Invalid hex in address: {e}"))?;
    Ok(format!("0x{}", clean.to_lowercase()))
}
