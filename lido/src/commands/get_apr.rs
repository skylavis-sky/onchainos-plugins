// commands/get_apr.rs — Query current Lido stETH APR
use anyhow::Result;
use serde_json::json;

pub async fn run() -> Result<()> {
    let apr = crate::api::get_apr_sma().await?;
    println!(
        "{}",
        json!({
            "ok": true,
            "data": {
                "smaApr": apr,
                "description": "7-day moving average APR for stETH liquid staking on Lido",
                "note": "Lido charges a 10% protocol fee on staking rewards"
            }
        })
    );
    Ok(())
}
