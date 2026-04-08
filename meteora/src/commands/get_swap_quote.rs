use clap::Args;
use std::process::Command;
use crate::config::PRICE_IMPACT_WARN_THRESHOLD;

#[derive(Args, Debug)]
pub struct GetSwapQuoteArgs {
    /// Source token mint address (or 11111111111111111111111111111111 for native SOL)
    #[arg(long)]
    pub from_token: String,

    /// Destination token mint address
    #[arg(long)]
    pub to_token: String,

    /// Human-readable input amount (e.g. "1.5" for 1.5 SOL)
    #[arg(long)]
    pub amount: String,
}

pub async fn execute(args: &GetSwapQuoteArgs) -> anyhow::Result<()> {
    // Use onchainos swap quote for Solana
    let output = Command::new("onchainos")
        .args([
            "swap", "quote",
            "--chain", "solana",
            "--from", &args.from_token,
            "--to", &args.to_token,
            "--readable-amount", &args.amount,
        ])
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let raw: serde_json::Value = serde_json::from_str(&stdout).unwrap_or(serde_json::json!({
        "raw_stdout": stdout.to_string(),
        "raw_stderr": stderr.to_string(),
    }));

    // Extract price impact if available
    let price_impact = raw["data"]["priceImpactPercentage"]
        .as_f64()
        .or_else(|| raw["priceImpactPercentage"].as_f64())
        .or_else(|| raw["data"]["price_impact"].as_f64())
        .unwrap_or(0.0);

    let price_impact_warn = price_impact > PRICE_IMPACT_WARN_THRESHOLD;

    let out_amount = raw["data"]["toTokenAmount"]
        .as_str()
        .or_else(|| raw["data"]["outAmount"].as_str())
        .unwrap_or("unknown");

    let from_amount = raw["data"]["fromTokenAmount"]
        .as_str()
        .or_else(|| raw["data"]["inAmount"].as_str())
        .unwrap_or(&args.amount);

    let result = serde_json::json!({
        "ok": true,
        "quote": {
            "from_token": args.from_token,
            "to_token": args.to_token,
            "from_amount_readable": args.amount,
            "from_amount_raw": from_amount,
            "to_amount_raw": out_amount,
            "price_impact_pct": price_impact,
            "price_impact_warning": if price_impact_warn {
                Some(format!("High price impact: {:.2}%. Consider splitting your trade.", price_impact))
            } else {
                None
            },
        },
        "raw_quote": raw,
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
