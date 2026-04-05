// positions command — query user mSOL holdings
use crate::api;
use crate::onchainos;

pub async fn execute() -> anyhow::Result<()> {
    let wallet = onchainos::resolve_wallet_solana()?;

    let (msol_balance, token_account) = api::fetch_msol_balance(&wallet).await?;
    let msol_price_sol = api::fetch_msol_price_sol().await?;
    let sol_value = msol_balance * msol_price_sol;

    let result = serde_json::json!({
        "ok": true,
        "data": {
            "wallet": wallet,
            "msol_balance": msol_balance,
            "sol_value": sol_value,
            "msol_token_account": if token_account.is_empty() { serde_json::Value::Null } else { token_account.into() },
            "msol_mint": "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So",
            "note": if msol_balance == 0.0 { "No mSOL found. Stake SOL to receive mSOL." } else { "mSOL balance reflects staked SOL + accrued rewards." }
        }
    });
    println!("{}", serde_json::to_string_pretty(&result)?);
    Ok(())
}
