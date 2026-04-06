use anyhow::Result;
use clap::Args;
use serde::Serialize;

use crate::abi::{NewTokenV6Params, encode_new_token_v6};
use crate::config::{PORTAL_ADDRESS, STANDARD_TOKEN_IMPL, TAX_TOKEN_V3_IMPL};
use crate::create2::{VanitySuffix, grind_salt, parse_impl_addr};
use crate::onchainos;

/// MigratorType: 0 = V1_MIGRATOR (standard), 1 = V2_MIGRATOR (tax tokens)
const MIGRATOR_TYPE_STANDARD: u8 = 0;
const MIGRATOR_TYPE_TAX: u8 = 1;

/// TokenVersion: 1 = TOKEN_V2_PERMIT (standard), 6 = TOKEN_TAXED_V3 (tax)
const TOKEN_VERSION_STANDARD: u8 = 1;
const TOKEN_VERSION_TAX_V3: u8 = 6;

const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";
/// dividendToken = address(0xfEEDFEEDfeEDFEedFEEdFEEDFeEdfEEdFeEdFEEd) = self (for tax tokens)
const DIVIDEND_TOKEN_SELF: &str = "0xfEEDFEEDfeEDFEedFEEdFEEDFeEdfEEdFeEdFEEd";

#[derive(Args, Debug)]
pub struct CreateTokenArgs {
    /// Token name (e.g. "Moon Hamster")
    #[arg(long)]
    pub name: String,

    /// Token symbol/ticker (e.g. "MHAMS")
    #[arg(long)]
    pub symbol: String,

    /// IPFS CID or metadata string for the token (upload separately via Flap API)
    #[arg(long, default_value = "")]
    pub meta: String,

    /// Buy tax rate in basis points (0 for standard token; >0 creates tax token V3)
    #[arg(long, default_value_t = 0)]
    pub buy_tax_bps: u16,

    /// Sell tax rate in basis points (0 for standard token)
    #[arg(long, default_value_t = 0)]
    pub sell_tax_bps: u16,

    /// Initial BNB buy amount in wei after token creation (0 = no initial buy)
    #[arg(long, default_value_t = 0)]
    pub initial_buy_wei: u128,

    /// Beneficiary address for tax proceeds (required if buy_tax_bps or sell_tax_bps > 0)
    #[arg(long)]
    pub beneficiary: Option<String>,

    /// Tax duration in seconds (0 = permanent; only used for tax tokens)
    #[arg(long, default_value_t = 0)]
    pub tax_duration: u64,

    /// Anti-farmer duration in seconds (0 = disabled)
    #[arg(long, default_value_t = 0)]
    pub anti_farmer_duration: u64,

    /// Skip CREATE2 salt grinding and use salt=0 (token address will not have vanity suffix)
    #[arg(long, default_value_t = false)]
    pub skip_salt_grind: bool,

    /// DEX id: 0=PancakeSwap V3, 1=PancakeSwap V2
    #[arg(long, default_value_t = 0)]
    pub dex_id: u8,
}

#[derive(Serialize, Debug)]
struct CreateTokenOutput {
    ok: bool,
    name: String,
    symbol: String,
    token_version: u8,
    is_tax_token: bool,
    buy_tax_bps: u16,
    sell_tax_bps: u16,
    predicted_token_address: String,
    salt_hex: String,
    salt_iterations: u64,
    initial_buy_wei: String,
    tx_hash: String,
    bscscan_tx_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    dry_run: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

pub async fn execute(args: &CreateTokenArgs, dry_run: bool) -> Result<()> {
    let is_tax_token = args.buy_tax_bps > 0 || args.sell_tax_bps > 0;

    // Validate tax token config
    if is_tax_token && args.beneficiary.is_none() {
        anyhow::bail!(
            "Tax tokens require --beneficiary address. Provide the address that receives tax proceeds."
        );
    }

    let beneficiary = if is_tax_token {
        normalize_address(args.beneficiary.as_deref().unwrap_or(ZERO_ADDRESS))?
    } else {
        ZERO_ADDRESS.to_string()
    };

    let token_version = if is_tax_token {
        TOKEN_VERSION_TAX_V3
    } else {
        TOKEN_VERSION_STANDARD
    };

    let migrator_type = if is_tax_token {
        MIGRATOR_TYPE_TAX
    } else {
        MIGRATOR_TYPE_STANDARD
    };

    // mktBps must sum with deflation/dividend/lp to 10000
    // For simplest valid config: mktBps=10000, others=0
    let mkt_bps: u16 = 10000;
    let deflation_bps: u16 = 0;
    let dividend_bps: u16 = 0;
    let lp_bps: u16 = 0;

    // Choose the correct token implementation address for CREATE2 grinding
    let impl_addr_str = if is_tax_token {
        TAX_TOKEN_V3_IMPL
    } else {
        STANDARD_TOKEN_IMPL
    };

    let vanity_suffix = if is_tax_token {
        VanitySuffix::Tax
    } else {
        VanitySuffix::Standard
    };

    // Grind for vanity salt
    let (salt, predicted_address, iterations) = if args.skip_salt_grind {
        ([0u8; 32], format!("0x{}", "0".repeat(40)), 0u64)
    } else {
        let impl_bytes = parse_impl_addr(impl_addr_str)?;
        eprintln!(
            "Grinding CREATE2 salt for vanity suffix '{}' (expected ~65,536 iterations)...",
            vanity_suffix.suffix_str()
        );
        let start = std::time::Instant::now();
        // Run in a blocking thread since this is CPU-intensive
        let (s, addr) = tokio::task::spawn_blocking(move || grind_salt(&impl_bytes, vanity_suffix))
            .await?;
        let elapsed = start.elapsed();
        // Count iterations by reading the counter from salt
        let iter_count = u64::from_le_bytes(s[0..8].try_into().unwrap_or([0u8; 8]));
        eprintln!(
            "Salt found after {} iterations in {:.2}s. Predicted token address: {}",
            iter_count,
            elapsed.as_secs_f64(),
            addr
        );
        (s, addr, iter_count)
    };

    let salt_hex = format!("0x{}", hex::encode(&salt));

    let dividend_token = if is_tax_token {
        DIVIDEND_TOKEN_SELF.to_string()
    } else {
        ZERO_ADDRESS.to_string()
    };

    // Build NewTokenV6Params
    let params = NewTokenV6Params {
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        meta: args.meta.clone(),
        dex_thresh: 0,
        salt,
        migrator_type,
        quote_token: ZERO_ADDRESS.to_string(),
        quote_amt: args.initial_buy_wei,
        beneficiary,
        permit_data: vec![],
        extension_id: [0u8; 32],
        extension_data: vec![],
        dex_id: args.dex_id,
        lp_fee_profile: 0,
        buy_tax_rate: args.buy_tax_bps,
        sell_tax_rate: args.sell_tax_bps,
        tax_duration: args.tax_duration,
        anti_farmer_duration: args.anti_farmer_duration,
        mkt_bps,
        deflation_bps,
        dividend_bps,
        lp_bps,
        minimum_share_balance: 0,
        dividend_token,
        commission_receiver: ZERO_ADDRESS.to_string(),
        token_version,
    };

    let calldata = encode_new_token_v6(&params)?;
    let input_data = format!("0x{}", hex::encode(&calldata));

    // For payable: msg.value = quoteAmt (BNB initial buy)
    // Tax token with ERC-20 quote: msg.value = quoteAmt + 1 gwei (but we use BNB = native so just quoteAmt)
    let value_wei = args.initial_buy_wei;

    let warning = if args.skip_salt_grind {
        Some(
            "Salt grinding was skipped (--skip-salt-grind). The token address will NOT have the vanity suffix (8888/7777). This may affect discoverability.".to_string()
        )
    } else {
        None
    };

    if dry_run {
        let output = CreateTokenOutput {
            ok: true,
            name: args.name.clone(),
            symbol: args.symbol.clone(),
            token_version,
            is_tax_token,
            buy_tax_bps: args.buy_tax_bps,
            sell_tax_bps: args.sell_tax_bps,
            predicted_token_address: predicted_address,
            salt_hex,
            salt_iterations: iterations,
            initial_buy_wei: value_wei.to_string(),
            tx_hash: String::new(),
            bscscan_tx_url: String::new(),
            dry_run: Some(true),
            warning,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Execute via onchainos
    let result = onchainos::wallet_contract_call_evm(
        PORTAL_ADDRESS,
        &input_data,
        value_wei,
        false,
    )
    .await?;

    let ok = result["ok"].as_bool().unwrap_or(false);
    if !ok {
        let err = result["error"].as_str().unwrap_or("unknown onchainos error");
        anyhow::bail!("create-token tx failed: {err}\nFull response: {result}");
    }

    let tx_hash = onchainos::extract_tx_hash(&result);
    let bscscan_tx_url = if tx_hash.is_empty() || tx_hash == "pending" {
        String::new()
    } else {
        format!("https://bscscan.com/tx/{}", tx_hash)
    };

    let output = CreateTokenOutput {
        ok,
        name: args.name.clone(),
        symbol: args.symbol.clone(),
        token_version,
        is_tax_token,
        buy_tax_bps: args.buy_tax_bps,
        sell_tax_bps: args.sell_tax_bps,
        predicted_token_address: predicted_address,
        salt_hex,
        salt_iterations: iterations,
        initial_buy_wei: value_wei.to_string(),
        tx_hash,
        bscscan_tx_url,
        dry_run: None,
        warning,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn normalize_address(addr: &str) -> anyhow::Result<String> {
    let clean = addr.trim_start_matches("0x");
    if clean.len() != 40 {
        anyhow::bail!("Invalid Ethereum address: '{}'", addr);
    }
    hex::decode(clean).map_err(|e| anyhow::anyhow!("Invalid hex in address: {e}"))?;
    Ok(format!("0x{}", clean.to_lowercase()))
}
