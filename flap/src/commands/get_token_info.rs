use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::abi::{
    decode_address, decode_uint256_as_u128, decode_uint256_as_u16, decode_uint256_as_u64,
    decode_uint256_as_u8, encode_get_token_v8_safe,
};
use crate::config::{DEFAULT_RPC_URL, GRADUATION_SUPPLY_THRESHOLD, PORTAL_ADDRESS};
use crate::onchainos;

#[derive(Args, Debug)]
pub struct GetTokenInfoArgs {
    /// Token contract address (0x-prefixed)
    #[arg(long)]
    pub token: String,

    /// BSC RPC URL (default: public BSC node)
    #[arg(long)]
    pub rpc_url: Option<String>,
}

/// Decoded TokenStateV8Safe from the Portal contract.
/// Fields decoded from ABI-encoded return data.
#[derive(Serialize, Debug)]
struct TokenStateOutput {
    ok: bool,
    token: String,
    status: String,
    status_code: u8,
    price_wei_per_token: String,
    circulating_supply: String,
    reserve_bnb_wei: String,
    reserve_bnb: f64,
    buy_tax_rate_bps: u16,
    sell_tax_rate_bps: u16,
    buy_tax_pct: f64,
    sell_tax_pct: f64,
    bonding_progress_pct: f64,
    dex_pool: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    bscscan_url: String,
}

/// TokenStatus enum values from Flap:
/// 0=Invalid, 1=Tradable, 4=DEX, 5=Staged
fn status_name(code: u8) -> &'static str {
    match code {
        1 => "Tradable",
        4 => "DEX (graduated)",
        5 => "Staged",
        _ => "Invalid",
    }
}

pub async fn execute(args: &GetTokenInfoArgs) -> Result<()> {
    let rpc_url = args
        .rpc_url
        .clone()
        .unwrap_or_else(|| DEFAULT_RPC_URL.to_string());

    // Validate token address
    let token = normalize_address(&args.token)?;

    // Build calldata for getTokenV8Safe(address)
    let calldata = encode_get_token_v8_safe(&token)?;
    let calldata_hex = format!("0x{}", hex::encode(&calldata));

    // eth_call
    let raw_result = onchainos::eth_call(&rpc_url, PORTAL_ADDRESS, &calldata_hex).await?;

    // Parse the returned ABI data
    // getTokenV8Safe returns a TokenStateV8Safe struct.
    // We decode the relevant fields by position.
    // The return is ABI-encoded. For a struct return value:
    // offset(32) + struct_data
    // We decode field by field from the struct data starting at offset 32.
    let raw = raw_result.trim_start_matches("0x");
    if raw.len() < 64 {
        anyhow::bail!("getTokenV8Safe returned too little data: {}", raw_result);
    }

    let bytes = hex::decode(raw)
        .map_err(|e| anyhow::anyhow!("Failed to hex-decode eth_call result: {e}"))?;

    // getTokenV8Safe returns TokenStateV8Safe directly (no outer ABI offset wrapper).
    // The struct starts at byte 0 of the return data.
    //
    // Observed on-chain layout (verified against live BSC data):
    //   word[ 0]: status (uint8)             — 0=Invalid, 1=Tradable, 4=DEX, 5=Staged
    //   word[ 1]: price (uint256)             — price in wei per token
    //   word[ 2]: circulatingSupply (uint256) — tokens in circulation (token wei units)
    //   word[ 3]: reserve (uint256)           — BNB reserve in gwei (divide by 1e9 for wei)
    //   word[ 4]: unknown field               — constant=6 (likely dexThresh or tokenVersion)
    //   word[ 5]: migrationThreshold (uint256)— BNB migration threshold in wei
    //   word[ 6]: r (uint256)                 — bonding curve param
    //   word[ 7]: h (uint256)                 — bonding curve param
    //   word[ 8]: k (uint256)                 — bonding curve param (max supply)
    //   word[ 9]: dexPool (address)           — zero if not graduated
    //   word[10]: unknown
    //   word[11]: unknown
    //   word[12]: buyTaxRate (uint16)         — basis points
    //   word[13]: sellTaxRate (uint16)        — basis points
    //   word[14]: unknown
    //   word[15]: unknown (per-token value)
    let struct_start = 0usize;

    let word = |n: usize| -> &[u8] {
        let start = struct_start + n * 32;
        let end = start + 32;
        if end <= bytes.len() {
            &bytes[start..end]
        } else {
            &[]
        }
    };

    let status_code = decode_uint256_as_u8(word(0));
    let price_wei = decode_uint256_as_u128(word(1));
    let circulating_supply = decode_uint256_as_u128(word(2));
    // reserve is stored in gwei; convert to wei by multiplying by 1e9
    let reserve_gwei = decode_uint256_as_u128(word(3));
    let reserve_wei = reserve_gwei.saturating_mul(1_000_000_000);
    let buy_tax_bps = decode_uint256_as_u16(word(12));
    let sell_tax_bps = decode_uint256_as_u16(word(13));
    let dex_pool = decode_address(word(9));

    let reserve_bnb = reserve_wei as f64 / 1e18;
    // Bonding progress: circulating supply / graduation threshold (800M tokens in wei units)
    let bonding_progress = if GRADUATION_SUPPLY_THRESHOLD > 0 {
        (circulating_supply as f64 / GRADUATION_SUPPLY_THRESHOLD as f64 * 100.0).min(100.0)
    } else {
        0.0
    };

    let status_name_str = status_name(status_code);

    let mut warning: Option<String> = None;
    if status_code == 4 {
        warning = Some(format!(
            "BONDING CURVE COMPLETE: This token has graduated to DEX. Trade via DEX pool: {}",
            dex_pool
        ));
    } else if sell_tax_bps > 500 {
        warning = Some(format!(
            "WARNING: This token has a {:.1}% sell tax ({} bps). You may receive little or no BNB when selling. Proceed with extreme caution.",
            sell_tax_bps as f64 / 100.0,
            sell_tax_bps
        ));
    }

    let output = TokenStateOutput {
        ok: true,
        token: token.clone(),
        status: status_name_str.to_string(),
        status_code,
        price_wei_per_token: price_wei.to_string(),
        circulating_supply: circulating_supply.to_string(),
        reserve_bnb_wei: reserve_wei.to_string(),
        reserve_bnb,
        buy_tax_rate_bps: buy_tax_bps,
        sell_tax_rate_bps: sell_tax_bps,
        buy_tax_pct: buy_tax_bps as f64 / 100.0,
        sell_tax_pct: sell_tax_bps as f64 / 100.0,
        bonding_progress_pct: bonding_progress,
        dex_pool,
        warning,
        bscscan_url: format!("https://bscscan.com/address/{}", token),
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Normalize an Ethereum address: lowercase, 0x-prefixed.
fn normalize_address(addr: &str) -> anyhow::Result<String> {
    let clean = addr.trim_start_matches("0x");
    if clean.len() != 40 {
        anyhow::bail!("Invalid Ethereum address: '{}'", addr);
    }
    hex::decode(clean).map_err(|e| anyhow::anyhow!("Invalid hex in address: {e}"))?;
    Ok(format!("0x{}", clean.to_lowercase()))
}
