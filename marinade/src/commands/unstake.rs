// unstake command — swap mSOL back to SOL via Jupiter routing
use crate::config::{MSOL_MINT, SOL_NATIVE};
use crate::onchainos;

pub async fn execute(amount: &str, slippage: f64, dry_run: bool) -> anyhow::Result<()> {
    // dry_run guard before resolve_wallet
    if dry_run {
        let result = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "txHash": "",
                "from": MSOL_MINT,
                "to": SOL_NATIVE,
                "amount_msol": amount,
                "description": "Dry run: unstake mSOL → SOL via Jupiter"
            }
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
        return Ok(());
    }

    let slippage_str = format!("{:.1}", slippage);
    let result = onchainos::swap_execute(MSOL_MINT, SOL_NATIVE, amount, &slippage_str, false).await?;
    let tx_hash = onchainos::extract_tx_hash(&result);

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "txHash": tx_hash,
            "action": "unstake",
            "from_token": "mSOL",
            "to_token": "SOL",
            "amount_msol": amount,
            "explorer": format!("https://solscan.io/tx/{}", tx_hash)
        },
        "raw": result
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
