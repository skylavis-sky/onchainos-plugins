/// deposit: deposit assets into a Umami GM vault (ERC-4626)

use crate::{config, onchainos, rpc};
use anyhow::Result;

pub async fn execute(
    vault_identifier: &str,
    amount: f64,
    chain_id: u64,
    from: Option<&str>,
    dry_run: bool,
) -> Result<()> {
    let rpc_url = config::ARBITRUM_RPC;

    let vault = config::find_vault(vault_identifier)
        .ok_or_else(|| anyhow::anyhow!("Unknown vault: {}. Use list-vaults to see available vaults.", vault_identifier))?;

    let decimals = vault.asset_decimals;
    let amount_raw = (amount * 10f64.powi(decimals as i32)) as u128;

    if amount_raw == 0 {
        anyhow::bail!("Amount too small");
    }

    // Build calldata for preview (always safe)
    let approve_calldata = onchainos::build_approve_calldata(vault.address, amount_raw);
    let deposit_calldata = onchainos::build_deposit_calldata(amount_raw, "0x0000000000000000000000000000000000000001"); // placeholder for dry_run

    if dry_run {
        // Return preview without touching wallet
        let preview_shares = rpc::preview_deposit(rpc_url, vault.address, amount_raw).await.unwrap_or(0);
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": true,
                "dry_run": true,
                "vault": vault.name,
                "asset": vault.asset_symbol,
                "amount_human": format!("{} {}", amount, vault.asset_symbol),
                "amount_raw": amount_raw.to_string(),
                "preview_shares": preview_shares.to_string(),
                "approve_calldata": approve_calldata,
                "deposit_calldata": deposit_calldata,
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

    // Check allowance before approving (avoid replacement tx underpriced)
    let current_allowance = rpc::allowance(rpc_url, vault.asset_address, &wallet, vault.address).await.unwrap_or(0);
    let mut approve_tx = None;

    if current_allowance < amount_raw {
        // Step 1: ERC-20 approve vault to spend the asset
        let approve_cd = onchainos::build_approve_calldata(vault.address, u128::MAX);
        let approve_result = onchainos::wallet_contract_call(
            chain_id,
            vault.asset_address,
            &approve_cd,
            Some(&wallet),
            None,
            false,
        ).await?;
        approve_tx = Some(onchainos::extract_tx_hash(&approve_result));
    }

    // Step 2: deposit into vault
    let deposit_cd = onchainos::build_deposit_calldata(amount_raw, &wallet);
    let deposit_result = onchainos::wallet_contract_call(
        chain_id,
        vault.address,
        &deposit_cd,
        Some(&wallet),
        None,
        false,
    ).await?;

    let deposit_tx = onchainos::extract_tx_hash(&deposit_result);

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "ok": true,
            "vault": vault.name,
            "asset": vault.asset_symbol,
            "amount_human": format!("{} {}", amount, vault.asset_symbol),
            "amount_raw": amount_raw.to_string(),
            "wallet": wallet,
            "approve_txHash": approve_tx,
            "deposit_txHash": deposit_tx,
            "explorer": format!("https://arbiscan.io/tx/{}", deposit_tx)
        }))?
    );
    Ok(())
}
