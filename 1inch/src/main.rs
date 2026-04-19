mod api;
mod config;
mod onchainos;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "1inch",
    about = "Swap tokens at the best rates across 200+ DEXs via the 1inch aggregation protocol"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get best swap quote (read-only, no transaction)
    GetQuote {
        /// Source token symbol or 0x address (e.g. ETH, USDC)
        #[arg(long)]
        src: String,
        /// Destination token symbol or 0x address (e.g. USDC, WETH)
        #[arg(long)]
        dst: String,
        /// Input amount in human-readable units (e.g. 0.001 for 0.001 ETH)
        #[arg(long)]
        amount: String,
        /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 56=BSC, 137=Polygon)
        #[arg(long, default_value = "8453")]
        chain: u64,
    },

    /// Swap tokens via 1inch aggregation protocol
    Swap {
        /// Source token symbol or 0x address (e.g. ETH, USDC)
        #[arg(long)]
        src: String,
        /// Destination token symbol or 0x address (e.g. USDC, WETH)
        #[arg(long)]
        dst: String,
        /// Input amount in human-readable units (e.g. 0.001 for 0.001 ETH)
        #[arg(long)]
        amount: String,
        /// Slippage tolerance in basis points (default: 50 = 0.5%)
        #[arg(long, default_value = "50")]
        slippage_bps: u64,
        /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 56=BSC, 137=Polygon)
        #[arg(long, default_value = "8453")]
        chain: u64,
        /// Preview calldata without submitting any transactions
        #[arg(long)]
        dry_run: bool,
    },

    /// Check current ERC-20 allowance for the 1inch router (read-only)
    GetAllowance {
        /// Token symbol or 0x address to check allowance for
        #[arg(long)]
        token: String,
        /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 56=BSC, 137=Polygon)
        #[arg(long, default_value = "8453")]
        chain: u64,
    },

    /// Approve ERC-20 token for use by the 1inch router
    Approve {
        /// Token symbol or 0x address to approve
        #[arg(long)]
        token: String,
        /// Approval amount in human-readable units (omit for unlimited / uint256 max)
        #[arg(long)]
        amount: Option<String>,
        /// Chain ID (1=Ethereum, 42161=Arbitrum, 8453=Base, 56=BSC, 137=Polygon)
        #[arg(long, default_value = "8453")]
        chain: u64,
        /// Preview calldata without submitting any transactions
        #[arg(long)]
        dry_run: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let client = api::build_client();

    match cli.command {
        Commands::GetQuote { src, dst, amount, chain } => {
            cmd_get_quote(&client, &src, &dst, &amount, chain)?;
        }

        Commands::Swap { src, dst, amount, slippage_bps, chain, dry_run } => {
            cmd_swap(&client, &src, &dst, &amount, slippage_bps, chain, dry_run)?;
        }

        Commands::GetAllowance { token, chain } => {
            cmd_get_allowance(&client, &token, chain)?;
        }

        Commands::Approve { token, amount, chain, dry_run } => {
            cmd_approve(&client, &token, amount.as_deref(), chain, dry_run)?;
        }
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// get-quote
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_get_quote(
    client: &reqwest::blocking::Client,
    src_sym: &str,
    dst_sym: &str,
    amount_human: &str,
    chain_id: u64,
) -> anyhow::Result<()> {
    config::validate_chain(chain_id)?;

    let (src_addr, src_dec) = config::resolve_token(src_sym, chain_id)?;
    let (dst_addr, _dst_dec) = config::resolve_token(dst_sym, chain_id)?;
    let amount_raw = config::to_minimal_units(amount_human, src_dec)?;

    eprintln!("  Querying 1inch quote: {} {} -> {} on chain {}...",
        amount_human, src_sym.to_uppercase(), dst_sym.to_uppercase(), chain_id);

    let resp = api::get_quote(client, chain_id, &src_addr, &dst_addr, &amount_raw)?;

    let dst_amount_raw = resp["dstAmount"].as_str().unwrap_or("0");
    let dst_dec_api = resp["dstToken"]["decimals"].as_u64().unwrap_or(18) as u8;
    let src_symbol = resp["srcToken"]["symbol"]
        .as_str()
        .unwrap_or(src_sym);
    let dst_symbol = resp["dstToken"]["symbol"]
        .as_str()
        .unwrap_or(dst_sym);
    let dst_amount_human = config::from_minimal_units(dst_amount_raw, dst_dec_api);
    let chain_name = config::get_chain_name(chain_id);

    let output = serde_json::json!({
        "src": src_symbol,
        "dst": dst_symbol,
        "src_amount": amount_human,
        "dst_amount": dst_amount_human,
        "chain": chain_name,
        "protocols": resp["protocols"]
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    eprintln!("\n  For {} {}, you will receive approximately {} {} on 1inch ({}).",
        amount_human, src_symbol, dst_amount_human, dst_symbol, chain_name);
    eprintln!("  No transaction submitted (read-only).");

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// swap
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_swap(
    client: &reqwest::blocking::Client,
    src_sym: &str,
    dst_sym: &str,
    amount_human: &str,
    slippage_bps: u64,
    chain_id: u64,
    dry_run: bool,
) -> anyhow::Result<()> {
    // ── dry_run guard BEFORE resolve_wallet ──
    if dry_run {
        eprintln!("  [dry-run] Dry-run mode active. No wallet resolution or transactions will occur.");
    }

    config::validate_chain(chain_id)?;

    let (src_addr, src_dec) = config::resolve_token(src_sym, chain_id)?;
    let (dst_addr, _) = config::resolve_token(dst_sym, chain_id)?;
    let amount_raw = config::to_minimal_units(amount_human, src_dec)?;

    // slippage conversion: bps → percent (50 bps → 0.5%)
    let slippage_percent = slippage_bps as f64 / 100.0;

    // Resolve wallet (skipped for dry-run — will use a placeholder)
    let wallet = if dry_run {
        "0x0000000000000000000000000000000000000001".to_string()
    } else {
        onchainos::resolve_wallet(chain_id)?
    };

    // ── Get quote for display ──
    eprintln!("  Fetching quote for {} {} -> {} on chain {}...",
        amount_human, src_sym.to_uppercase(), dst_sym.to_uppercase(), chain_id);

    let quote = api::get_quote(client, chain_id, &src_addr, &dst_addr, &amount_raw)?;
    let dst_dec_api = quote["dstToken"]["decimals"].as_u64().unwrap_or(18) as u8;
    let dst_symbol = quote["dstToken"]["symbol"].as_str().unwrap_or(dst_sym).to_string();
    let src_symbol = quote["srcToken"]["symbol"].as_str().unwrap_or(src_sym).to_string();
    let expected_out_raw = quote["dstAmount"].as_str().unwrap_or("0");
    let expected_out_human = config::from_minimal_units(expected_out_raw, dst_dec_api);

    eprintln!("  Expected output: ~{} {}", expected_out_human, dst_symbol);
    eprintln!("  Slippage tolerance: {}% ({} bps)", slippage_percent, slippage_bps);

    // ── Approve if ERC-20 source ──
    let src_is_native = config::is_native_token(&src_addr);
    if !src_is_native && !dry_run {
        eprintln!("  Checking {} allowance for 1inch router...", src_symbol);
        let allowance_resp = api::get_allowance(client, chain_id, &src_addr, &wallet)?;
        let allowance_str = allowance_resp["allowance"].as_str().unwrap_or("0");
        let allowance: u128 = allowance_str.parse().unwrap_or(0);
        let amount_needed: u128 = amount_raw.parse().unwrap_or(0);

        if allowance < amount_needed {
            eprintln!("  Insufficient allowance (have {}, need {}). Requesting approval...",
                allowance_str, amount_raw);

            // Ask user to confirm before broadcasting the approve tx
            eprintln!("  [confirm] About to broadcast ERC-20 approve transaction for {} to 1inch router on chain {}.",
                src_symbol, chain_id);
            eprintln!("  [confirm] This will allow 1inch to spend your {} tokens.", src_symbol);

            let approve_resp = api::get_approve_tx(client, chain_id, &src_addr, None)?;
            let approve_to = approve_resp["to"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'to' in approve response"))?;
            let approve_data = approve_resp["data"].as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing 'data' in approve response"))?;

            let approve_result = onchainos::wallet_contract_call(
                chain_id,
                approve_to,
                approve_data,
                Some("0"),
                false,
            )?;

            let approve_hash = onchainos::extract_tx_hash(&approve_result);
            eprintln!("  Approve tx submitted: {}", approve_hash);

            // Wait for approval before proceeding to swap
            onchainos::wait_for_tx(&approve_hash, chain_id)?;
            eprintln!("  Approval confirmed. Proceeding to swap...");
        } else {
            eprintln!("  Allowance sufficient ({}).", allowance_str);
        }
    }

    // ── Get swap calldata ──
    eprintln!("  Fetching swap calldata from 1inch API...");
    let swap_resp = api::get_swap(
        client,
        chain_id,
        &src_addr,
        &dst_addr,
        &amount_raw,
        &wallet,
        slippage_percent,
        dry_run, // disableEstimate=true in dry-run
    )?;

    let tx_data = swap_resp["tx"]["data"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing tx.data in 1inch swap response"))?;
    let tx_to = swap_resp["tx"]["to"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing tx.to in 1inch swap response"))?;
    let tx_value = swap_resp["tx"]["value"].as_str().unwrap_or("0");

    if dry_run {
        let dry_output = serde_json::json!({
            "dry_run": true,
            "src": format!("{} {}", amount_human, src_symbol),
            "dst_estimate": format!("~{} {}", expected_out_human, dst_symbol),
            "slippage": format!("{}%", slippage_percent),
            "tx": {
                "to": tx_to,
                "data": tx_data,
                "value": tx_value,
                "chain_id": chain_id
            }
        });
        println!("{}", serde_json::to_string_pretty(&dry_output)?);
        eprintln!("  Dry-run mode: swap calldata generated. Broadcasting skipped.");
        return Ok(());
    }

    // ── Broadcast swap ──
    eprintln!("  [confirm] About to broadcast swap: {} {} -> ~{} {} on chain {} via 1inch.",
        amount_human, src_symbol, expected_out_human, dst_symbol, chain_id);
    eprintln!("  Ask user to confirm before proceeding with the swap transaction.");

    let value_to_send = if src_is_native { Some(tx_value) } else { Some("0") };

    let swap_result = onchainos::wallet_contract_call(
        chain_id,
        tx_to,
        tx_data,
        value_to_send,
        false,
    )?;

    let tx_hash = onchainos::extract_tx_hash(&swap_result);
    let explorer = config::explorer_url(chain_id, &tx_hash);
    let chain_name = config::get_chain_name(chain_id);

    let output = serde_json::json!({
        "txHash": tx_hash,
        "src": format!("{} {}", amount_human, src_symbol),
        "dst_estimate": format!("~{} {}", expected_out_human, dst_symbol),
        "chain": chain_name,
        "explorer": explorer
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    eprintln!("\n  Swap submitted! Sent {} {} -> ~{} {}.",
        amount_human, src_symbol, expected_out_human, dst_symbol);
    eprintln!("  txHash: {}", tx_hash);
    eprintln!("  View on {}: {}", chain_name, explorer);

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// get-allowance
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_get_allowance(
    client: &reqwest::blocking::Client,
    token_sym: &str,
    chain_id: u64,
) -> anyhow::Result<()> {
    config::validate_chain(chain_id)?;

    let (token_addr, token_dec) = config::resolve_token(token_sym, chain_id)?;

    if config::is_native_token(&token_addr) {
        let output = serde_json::json!({
            "token": token_sym.to_uppercase(),
            "allowance": "N/A",
            "note": "Native token (ETH/BNB/MATIC) does not require ERC-20 approval."
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    let wallet = onchainos::resolve_wallet(chain_id)?;

    let resp = api::get_allowance(client, chain_id, &token_addr, &wallet)?;
    let allowance_raw = resp["allowance"].as_str().unwrap_or("0");

    // Interpret the allowance value
    let allowance_display = if allowance_raw == "0" {
        "0 (no approval granted)".to_string()
    } else if allowance_raw == "115792089237316195423570985008687907853269984665640564039457584007913129639935" {
        "unlimited (uint256 max)".to_string()
    } else {
        config::from_minimal_units(allowance_raw, token_dec)
    };

    let chain_name = config::get_chain_name(chain_id);
    let token_label = if token_sym.starts_with("0x") {
        &token_sym[..10.min(token_sym.len())]
    } else {
        token_sym
    };

    let output = serde_json::json!({
        "token": token_label.to_uppercase(),
        "token_address": token_addr,
        "wallet": wallet,
        "allowance": allowance_display,
        "allowance_raw": allowance_raw,
        "chain": chain_name,
        "spender": config::ROUTER_V6
    });

    println!("{}", serde_json::to_string_pretty(&output)?);

    if allowance_raw == "0" {
        eprintln!("\n  No {} approval granted to the 1inch router on {}.", token_label.to_uppercase(), chain_name);
        eprintln!("  Run `1inch approve --token {} --chain {}` before swapping.", token_sym, chain_id);
    } else {
        eprintln!("\n  {} allowance on {}: {}", token_label.to_uppercase(), chain_name, allowance_display);
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// approve
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_approve(
    client: &reqwest::blocking::Client,
    token_sym: &str,
    amount_human: Option<&str>,
    chain_id: u64,
    dry_run: bool,
) -> anyhow::Result<()> {
    // ── dry_run guard BEFORE resolve_wallet ──
    if dry_run {
        eprintln!("  [dry-run] Dry-run mode active. No wallet resolution or transactions will occur.");
    }

    config::validate_chain(chain_id)?;

    let (token_addr, token_dec) = config::resolve_token(token_sym, chain_id)?;

    if config::is_native_token(&token_addr) {
        anyhow::bail!("Native token (ETH/BNB/MATIC) does not require ERC-20 approval.");
    }

    // Convert optional human amount to raw units
    let amount_raw: Option<String> = match amount_human {
        Some(a) => Some(config::to_minimal_units(a, token_dec)?),
        None => None,
    };

    let amount_label = amount_human.map(|a| a.to_string())
        .unwrap_or_else(|| "unlimited".to_string());

    eprintln!("  Fetching approve calldata for {} ({}) on chain {}...",
        token_sym.to_uppercase(), amount_label, chain_id);

    let approve_resp = api::get_approve_tx(
        client,
        chain_id,
        &token_addr,
        amount_raw.as_deref(),
    )?;

    let approve_to = approve_resp["to"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'to' in approve response"))?;
    let approve_data = approve_resp["data"].as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing 'data' in approve response"))?;

    if dry_run {
        let dry_output = serde_json::json!({
            "dry_run": true,
            "token": token_sym.to_uppercase(),
            "amount": amount_label,
            "spender": config::ROUTER_V6,
            "tx": {
                "to": approve_to,
                "data": approve_data,
                "value": "0",
                "chain_id": chain_id
            }
        });
        println!("{}", serde_json::to_string_pretty(&dry_output)?);
        eprintln!("  Dry-run mode: approve calldata generated. Broadcasting skipped.");
        return Ok(());
    }

    let chain_name = config::get_chain_name(chain_id);

    // Ask user to confirm before broadcasting the approve tx
    eprintln!("  [confirm] About to broadcast ERC-20 approve transaction.");
    eprintln!("  [confirm] Token: {} on {} | Amount: {} | Spender: 1inch Router V6 ({})",
        token_sym.to_uppercase(), chain_name, amount_label, config::ROUTER_V6);

    let result = onchainos::wallet_contract_call(
        chain_id,
        approve_to,
        approve_data,
        Some("0"),
        false,
    )?;

    let tx_hash = onchainos::extract_tx_hash(&result);
    let explorer = config::explorer_url(chain_id, &tx_hash);

    let output = serde_json::json!({
        "txHash": tx_hash,
        "token": token_sym.to_uppercase(),
        "amount_approved": amount_label,
        "spender": config::ROUTER_V6,
        "chain": chain_name,
        "explorer": explorer
    });

    println!("{}", serde_json::to_string_pretty(&output)?);
    eprintln!("\n  Approve transaction submitted!");
    eprintln!("  txHash: {}", tx_hash);
    eprintln!("  View on {}: {}", chain_name, explorer);

    Ok(())
}
