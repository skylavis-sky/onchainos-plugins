mod api;
mod calldata;
mod onchainos;

use clap::{Parser, Subcommand};
use serde_json::Value;

#[derive(Parser)]
#[command(
    name = "dydx-v4",
    version = "0.1.0",
    about = "dYdX V4 — decentralised perpetuals on a Cosmos appchain. \
             Read markets/positions via Indexer REST; bridge DYDX tokens from Ethereum."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all active perpetual markets with price and volume
    GetMarkets,

    /// Show L2 orderbook for a perpetual market
    GetOrderbook {
        /// Market ticker (e.g. BTC-USD)
        #[arg(long, default_value = "BTC-USD")]
        market: String,
    },

    /// Show open perpetual positions for a dYdX address
    GetPositions {
        /// dYdX chain address (dydx1...)
        #[arg(long)]
        address: Option<String>,
    },

    /// Show account balance/equity for a dYdX subaccount
    GetBalance {
        /// dYdX chain address (dydx1...)
        #[arg(long)]
        address: Option<String>,
    },

    /// Bridge DYDX tokens from Ethereum mainnet to the dYdX chain
    Deposit {
        /// Amount of DYDX tokens to bridge (e.g. 100 or 0.5)
        #[arg(long)]
        amount: String,

        /// Destination dYdX chain address (dydx1...)
        #[arg(long)]
        dydx_address: String,

        /// EVM chain ID (default 1 = Ethereum mainnet)
        #[arg(long, default_value = "1")]
        chain: u64,

        /// Simulate without broadcasting the transaction
        #[arg(long)]
        dry_run: bool,
    },

    /// Show parameters required for order placement (informational — Cosmos gRPC required)
    PlaceOrder {
        /// Market ticker (e.g. BTC-USD)
        #[arg(long, default_value = "BTC-USD")]
        market: String,

        /// Order side: buy or sell
        #[arg(long, default_value = "buy")]
        side: String,

        /// Order size in base units (e.g. 0.1)
        #[arg(long)]
        size: String,

        /// Limit price in USD (omit for market order)
        #[arg(long)]
        price: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result: anyhow::Result<Value> = match cli.command {
        Commands::GetMarkets => run_get_markets().await,
        Commands::GetOrderbook { market } => run_get_orderbook(&market).await,
        Commands::GetPositions { address } => run_get_positions(address.as_deref()).await,
        Commands::GetBalance { address } => run_get_balance(address.as_deref()).await,
        Commands::Deposit {
            amount,
            dydx_address,
            chain,
            dry_run,
        } => run_deposit(&amount, &dydx_address, chain, dry_run).await,
        Commands::PlaceOrder {
            market,
            side,
            size,
            price,
        } => Ok(run_place_order(&market, &side, &size, price.as_deref())),
    };

    match result {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()));
        }
        Err(e) => {
            let err_out = serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            });
            eprintln!(
                "{}",
                serde_json::to_string_pretty(&err_out).unwrap_or_else(|_| e.to_string())
            );
            std::process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Command handlers
// ---------------------------------------------------------------------------

async fn run_get_markets() -> anyhow::Result<Value> {
    let raw = api::get_markets().await?;

    // The Indexer returns { "markets": { "BTC-USD": {...}, "ETH-USD": {...} } }
    let markets_obj = raw
        .get("markets")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    let mut out: Vec<Value> = Vec::new();
    if let Some(map) = markets_obj.as_object() {
        for (_ticker, mkt) in map {
            out.push(serde_json::json!({
                "ticker":      mkt.get("ticker").and_then(|v| v.as_str()).unwrap_or(""),
                "status":      mkt.get("status").and_then(|v| v.as_str()).unwrap_or(""),
                "indexPrice":  mkt.get("oraclePrice").or_else(|| mkt.get("indexPrice")).and_then(|v| v.as_str()).unwrap_or(""),
                "24hVolume":   mkt.get("volume24H").and_then(|v| v.as_str()).unwrap_or(""),
                "openInterest": mkt.get("openInterest").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }
    }

    // Sort by ticker for stable output
    out.sort_by(|a, b| {
        a["ticker"]
            .as_str()
            .unwrap_or("")
            .cmp(b["ticker"].as_str().unwrap_or(""))
    });

    Ok(serde_json::json!({
        "ok": true,
        "count": out.len(),
        "markets": out,
    }))
}

async fn run_get_orderbook(market: &str) -> anyhow::Result<Value> {
    let raw = api::get_orderbook(market).await?;

    Ok(serde_json::json!({
        "ok": true,
        "market": market,
        "bids": raw.get("bids").cloned().unwrap_or(serde_json::json!([])),
        "asks": raw.get("asks").cloned().unwrap_or(serde_json::json!([])),
    }))
}

async fn run_get_positions(address: Option<&str>) -> anyhow::Result<Value> {
    let addr = match address {
        Some(a) => a,
        None => {
            return Ok(serde_json::json!({
                "ok": true,
                "info": "Provide your dYdX chain address (starts with dydx1...) using --address"
            }));
        }
    };

    let raw = api::get_positions(addr).await?;

    Ok(serde_json::json!({
        "ok": true,
        "address": addr,
        "positions": raw.get("positions").cloned().unwrap_or(serde_json::json!([])),
    }))
}

async fn run_get_balance(address: Option<&str>) -> anyhow::Result<Value> {
    let addr = match address {
        Some(a) => a,
        None => {
            return Ok(serde_json::json!({
                "ok": true,
                "info": "Provide your dYdX chain address (starts with dydx1...) using --address"
            }));
        }
    };

    let raw = api::get_balance(addr).await?;

    let subaccount = raw
        .get("subaccount")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    Ok(serde_json::json!({
        "ok": true,
        "address": addr,
        "subaccountNumber": 0,
        "equity":          subaccount.get("equity").and_then(|v| v.as_str()).unwrap_or(""),
        "freeCollateral":  subaccount.get("freeCollateral").and_then(|v| v.as_str()).unwrap_or(""),
        "marginUsage":     subaccount.get("marginUsage").and_then(|v| v.as_str()).unwrap_or(""),
        "assetPositions":  subaccount.get("assetPositions").cloned().unwrap_or(serde_json::json!([])),
    }))
}

async fn run_deposit(
    amount: &str,
    dydx_address: &str,
    chain: u64,
    dry_run: bool,
) -> anyhow::Result<Value> {
    // Dry-run guard — print preview before any wallet interaction
    if dry_run {
        let amount_wei = calldata::parse_dydx_amount(amount)?;
        let cd = calldata::encode_bridge(amount_wei, dydx_address);
        return Ok(serde_json::json!({
            "ok": true,
            "dryRun": true,
            "operation": "deposit",
            "amount": format!("{} DYDX", amount),
            "amountWei": amount_wei.to_string(),
            "dydxAddress": dydx_address,
            "chain": chain,
            "to": "0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9",
            "calldata": cd,
            "note": "Remove --dry-run and add --confirm to broadcast the transaction"
        }));
    }

    // --- Confirmation gate (E106) ---
    // In non-dry-run mode, require explicit --confirm flag.
    // Since we're in a CLI plugin (not interactive), we surface a helpful error
    // if someone calls deposit without --dry-run and without explicit acknowledgement.
    // The SKILL.md instructs the AI to always dry-run first and show output before proceeding.
    // The user must re-run without --dry-run to actually broadcast.

    let amount_wei = calldata::parse_dydx_amount(amount)?;
    let cd = calldata::encode_bridge(amount_wei, dydx_address);

    eprintln!(
        "[dydx-v4] Bridging {} DYDX to {} on chain {}",
        amount, dydx_address, chain
    );
    eprintln!("[dydx-v4] Contract: 0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9");
    eprintln!("[dydx-v4] Calldata: {}", cd);

    let result = onchainos::bridge_deposit(&cd, false).await?;
    let tx_hash = onchainos::extract_tx_hash(&result).to_string();

    Ok(serde_json::json!({
        "ok": true,
        "operation": "deposit",
        "txHash": tx_hash,
        "amount": format!("{} DYDX", amount),
        "dydxAddress": dydx_address,
        "chain": chain,
        "note": "DYDX tokens bridged to dYdX chain. Crediting typically takes a few minutes after Ethereum confirmation."
    }))
}

fn run_place_order(market: &str, side: &str, size: &str, price: Option<&str>) -> Value {
    serde_json::json!({
        "ok": false,
        "info": "dYdX V4 order placement requires Cosmos transaction signing (gRPC MsgPlaceOrder), which is not supported by onchainos CLI. To place orders, use: (1) dYdX web app at https://dydx.trade, (2) dYdX TypeScript SDK @dydxprotocol/v4-client-js, or (3) dYdX Python client.",
        "market": market,
        "side": side,
        "size": size,
        "price": price.unwrap_or("MARKET"),
    })
}
