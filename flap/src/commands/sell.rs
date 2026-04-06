use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::abi::{
    decode_uint256_as_u128, decode_uint256_as_u16, decode_uint256_as_u8, encode_approve,
    encode_quote_exact_input, encode_swap_exact_input,
};
use crate::config::{
    DEFAULT_RPC_URL, DEFAULT_SLIPPAGE_BPS, PORTAL_ADDRESS, SELL_TAX_WARNING_THRESHOLD_BPS,
};
use crate::onchainos;

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

#[derive(Args, Debug)]
pub struct SellArgs {
    /// Token contract address to sell (0x-prefixed)
    #[arg(long)]
    pub token: String,

    /// Token amount to sell (in token units, not wei for ERC-20 with 18 decimals)
    #[arg(long)]
    pub token_amount: u128,

    /// Slippage tolerance in basis points (default: 500 = 5%)
    #[arg(long, default_value_t = DEFAULT_SLIPPAGE_BPS)]
    pub slippage_bps: u64,

    /// BSC RPC URL (default: public BSC node)
    #[arg(long)]
    pub rpc_url: Option<String>,

    /// Skip sell-tax warning and proceed anyway
    #[arg(long, default_value_t = false)]
    pub force: bool,
}

#[derive(Serialize, Debug)]
struct SellOutput {
    ok: bool,
    token: String,
    token_amount: String,
    min_bnb_out_wei: String,
    expected_bnb_out_wei: String,
    expected_bnb_out: f64,
    slippage_bps: u64,
    approve_tx_hash: String,
    sell_tx_hash: String,
    bscscan_sell_tx_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

pub async fn execute(args: &SellArgs, dry_run: bool) -> Result<()> {
    let rpc_url = args
        .rpc_url
        .clone()
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    let token = normalize_address(&args.token)?;

    // Fetch token state: check status and sell tax
    let (status_code, sell_tax_bps) = get_token_status_and_tax(&rpc_url, &token).await?;

    if status_code == 4 {
        let output = SellOutput {
            ok: false,
            token: token.clone(),
            token_amount: args.token_amount.to_string(),
            min_bnb_out_wei: "0".to_string(),
            expected_bnb_out_wei: "0".to_string(),
            expected_bnb_out: 0.0,
            slippage_bps: args.slippage_bps,
            approve_tx_hash: String::new(),
            sell_tx_hash: String::new(),
            bscscan_sell_tx_url: String::new(),
            dry_run: None,
            warning: Some(
                "BONDING CURVE COMPLETE: This token has graduated to DEX. Use onchainos dex swap to trade."
                    .to_string(),
            ),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Build sell-tax warning
    let sell_warning = if sell_tax_bps > SELL_TAX_WARNING_THRESHOLD_BPS {
        Some(format!(
            "WARNING: This token has a {:.1}% sell tax ({} bps). You may receive significantly less BNB. Vault risk is elevated. Proceed with extreme caution.",
            sell_tax_bps as f64 / 100.0,
            sell_tax_bps
        ))
    } else if sell_tax_bps > 0 {
        Some(format!(
            "Note: This token has a {:.1}% sell tax.",
            sell_tax_bps as f64 / 100.0
        ))
    } else {
        None
    };

    // Get quote for sell
    let quote_calldata =
        encode_quote_exact_input(&token, ZERO_ADDRESS, args.token_amount)?;
    let quote_hex = format!("0x{}", hex::encode(&quote_calldata));

    let expected_bnb_out = match onchainos::eth_call(&rpc_url, PORTAL_ADDRESS, &quote_hex).await {
        Ok(raw) => {
            let clean = raw.trim_start_matches("0x");
            if clean.len() >= 64 {
                let bytes = hex::decode(&clean[..64]).unwrap_or_default();
                decode_uint256_as_u128(&bytes)
            } else {
                0u128
            }
        }
        Err(_) => 0u128,
    };

    // Apply slippage
    let min_bnb_out = if expected_bnb_out > 0 {
        expected_bnb_out
            .saturating_mul(10000 - args.slippage_bps as u128)
            / 10000
    } else {
        0u128
    };

    if dry_run {
        let output = SellOutput {
            ok: true,
            token: token.clone(),
            token_amount: args.token_amount.to_string(),
            min_bnb_out_wei: min_bnb_out.to_string(),
            expected_bnb_out_wei: expected_bnb_out.to_string(),
            expected_bnb_out: expected_bnb_out as f64 / 1e18,
            slippage_bps: args.slippage_bps,
            approve_tx_hash: String::new(),
            sell_tx_hash: String::new(),
            bscscan_sell_tx_url: String::new(),
            dry_run: Some(true),
            warning: sell_warning,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Step 1: ERC-20 approve token to Portal
    // Approve at least token_amount (use max u128 for convenience)
    let approve_calldata = encode_approve(PORTAL_ADDRESS, args.token_amount)?;
    let approve_input_data = format!("0x{}", hex::encode(&approve_calldata));

    let approve_result =
        onchainos::wallet_contract_call_evm(&token, &approve_input_data, 0, false).await?;

    let approve_ok = approve_result["ok"].as_bool().unwrap_or(false);
    if !approve_ok {
        let err = approve_result["error"]
            .as_str()
            .unwrap_or("unknown onchainos error");
        anyhow::bail!("approve tx failed: {err}\nFull response: {approve_result}");
    }

    let approve_tx_hash = onchainos::extract_tx_hash(&approve_result);

    // Wait a few seconds for approve to be included
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // Step 2: swapExactInput sell
    // sell: inputToken=token, outputToken=address(0), inputAmount=token_amount, permitData=empty
    let sell_calldata = encode_swap_exact_input(
        &token,
        ZERO_ADDRESS,
        args.token_amount,
        min_bnb_out,
        &[],
    )?;
    let sell_input_data = format!("0x{}", hex::encode(&sell_calldata));

    let sell_result =
        onchainos::wallet_contract_call_evm(PORTAL_ADDRESS, &sell_input_data, 0, false).await?;

    let sell_ok = sell_result["ok"].as_bool().unwrap_or(false);
    if !sell_ok {
        let err = sell_result["error"]
            .as_str()
            .unwrap_or("unknown onchainos error");
        anyhow::bail!("sell tx failed: {err}\nFull response: {sell_result}");
    }

    let sell_tx_hash = onchainos::extract_tx_hash(&sell_result);
    let bscscan_sell_tx_url = if sell_tx_hash.is_empty() || sell_tx_hash == "pending" {
        String::new()
    } else {
        format!("https://bscscan.com/tx/{}", sell_tx_hash)
    };

    let output = SellOutput {
        ok: sell_ok,
        token,
        token_amount: args.token_amount.to_string(),
        min_bnb_out_wei: min_bnb_out.to_string(),
        expected_bnb_out_wei: expected_bnb_out.to_string(),
        expected_bnb_out: expected_bnb_out as f64 / 1e18,
        slippage_bps: args.slippage_bps,
        approve_tx_hash,
        sell_tx_hash,
        bscscan_sell_tx_url,
        dry_run: None,
        warning: sell_warning,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Fetch token status and sell tax rate from Portal.
async fn get_token_status_and_tax(rpc_url: &str, token: &str) -> anyhow::Result<(u8, u16)> {
    let calldata = crate::abi::encode_get_token_v8_safe(token)?;
    let calldata_hex = format!("0x{}", hex::encode(&calldata));
    let raw = onchainos::eth_call(rpc_url, PORTAL_ADDRESS, &calldata_hex).await?;
    let clean = raw.trim_start_matches("0x");

    // Need at least 14 words (0..13) for status and sellTaxRate
    if clean.len() < 14 * 64 {
        return Ok((0, 0));
    }

    let bytes = hex::decode(clean)?;
    // TokenStateV8Safe starts at byte 0 (no outer offset wrapper).
    // word[0]  = status (uint8)
    // word[13] = sellTaxRate (uint16) — verified on-chain
    if bytes.len() < 14 * 32 {
        return Ok((0, 0));
    }

    let status_code = decode_uint256_as_u8(&bytes[0..32]);
    let sell_tax_bps = decode_uint256_as_u16(&bytes[13 * 32..14 * 32]);

    Ok((status_code, sell_tax_bps))
}

fn normalize_address(addr: &str) -> anyhow::Result<String> {
    let clean = addr.trim_start_matches("0x");
    if clean.len() != 40 {
        anyhow::bail!("Invalid Ethereum address: '{}'", addr);
    }
    hex::decode(clean).map_err(|e| anyhow::anyhow!("Invalid hex in address: {e}"))?;
    Ok(format!("0x{}", clean.to_lowercase()))
}
