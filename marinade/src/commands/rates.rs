// rates command — query mSOL/SOL exchange rate and staking info
use crate::api;
use crate::config::APPROX_STAKING_APY;

pub async fn execute() -> anyhow::Result<()> {
    let msol_price_sol = api::fetch_msol_price_sol().await?;
    let sol_per_msol = 1.0 / msol_price_sol;
    let total_msol_supply = api::fetch_msol_total_supply().await?;
    let total_sol_staked = total_msol_supply * msol_price_sol;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "msol_per_sol": msol_price_sol,
            "sol_per_msol": sol_per_msol,
            "total_msol_supply": total_msol_supply,
            "total_sol_staked_approx": total_sol_staked,
            "staking_apy": APPROX_STAKING_APY,
            "description": "Stake SOL to get mSOL. mSOL auto-accrues staking rewards over time.",
            "protocol": "Marinade Finance",
            "chain": "Solana (501)"
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
