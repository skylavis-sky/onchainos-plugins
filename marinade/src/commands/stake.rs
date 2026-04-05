// stake command — stake SOL to receive mSOL via Jupiter routing
use crate::config::{MSOL_MINT, SOL_NATIVE};
use crate::onchainos;

pub async fn execute(amount: &str, slippage: f64, dry_run: bool) -> anyhow::Result<()> {
    // dry_run guard must be before resolve_wallet (wallet not needed for dry run)
    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "",
                "from": SOL_NATIVE,
                "to": MSOL_MINT,
                "amount_sol": amount,
                "description": "Dry run: stake SOL → mSOL via Marinade/Jupiter"
            }
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let slippage_str = format!("{:.1}", slippage);
    let result = onchainos::swap_execute(SOL_NATIVE, MSOL_MINT, amount, &slippage_str, false).await?;
    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "action": "stake",
            "from_token": "SOL",
            "to_token": "mSOL",
            "amount_sol": amount,
            "explorer": format!("https://solscan.io/tx/{}", tx_hash)
        },
        "raw": result
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
