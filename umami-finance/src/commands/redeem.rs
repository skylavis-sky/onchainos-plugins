/// redeem: redeem shares from a Umami GM vault (ERC-4626)

use crate::{config, onchainos, rpc};
use anyhow::Result;

pub async fn execute(
    vault_identifier: &str,
    shares: Option<f64>,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::ARBITRUM_RPC;

    let vault = config::find_vault(vault_identifier)
        .ok_or_else(|| anyhow::anyhow!("Unknown vault: {}. Use list-vaults to see available vaults.", vault_identifier))?;

    let decimals = vault.asset_decimals;

    if dry_run {
        // In dry_run mode, use a placeholder share amount
        let shares_raw = shares
            .map(|s| (s * 10f64.powi(decimals as i32)) as u128)
            .unwrap_or(10u128.pow(decimals));

        let placeholder_receiver = "0x0000000000000000000000000000000000000000";
        let redeem_calldata = onchainos::build_redeem_calldata(shares_raw, placeholder_receiver, placeholder_receiver);

        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "vault": vault.name,
                "asset": vault.asset_symbol,
                "shares_raw": shares_raw.to_string(),
                "redeem_calldata": redeem_calldata,
                "data": {
                    "txHash": "0x0000000000000000000000000000000000000000000000000000000000000000"
                }
            }))?
        );
        return Ok(());
    }

    // Resolve wallet AFTER dry_run guard
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        let w = onchainos::resolve_wallet(chain_id)?;
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet address. Pass --from <address> or ensure onchainos is logged in.");
        }
        w
    };

    // Determine share amount
    let shares_raw = if let Some(s) = shares {
        (s * 10f64.powi(decimals as i32)) as u128
    } else {
        // Redeem all shares
        rpc::balance_of(rpc_url, vault.address, &wallet).await?
    };

    if shares_raw == 0 {
        anyhow::bail!("No shares to redeem in vault {} for wallet {}", vault.name, wallet);
    }

    // Preview redeem
    let assets_out = rpc::preview_redeem(rpc_url, vault.address, shares_raw).await.unwrap_or(0);
    let assets_human = format!(
        "{:.6} {}",
        assets_out as f64 / 10f64.powi(decimals as i32),
        vault.asset_symbol
    );

    // Execute redeem
    let redeem_cd = onchainos::build_redeem_calldata(shares_raw, &wallet, &wallet);
    let redeem_result = onchainos::wallet_contract_call(
        chain_id,
        vault.address,
        &redeem_cd,
        Some(&wallet),
        None,
        false,
    ).await?;

    let tx_hash = onchainos::extract_tx_hash(&redeem_result);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "vault": vault.name,
            "shares_raw": shares_raw.to_string(),
            "assets_out_raw": assets_out.to_string(),
            "assets_out_human": assets_human,
            "wallet": wallet,
            "txHash": tx_hash,
            "explorer": format!("https://arbiscan.io/tx/{}", tx_hash)
        }))?
    );
    Ok(())
}
