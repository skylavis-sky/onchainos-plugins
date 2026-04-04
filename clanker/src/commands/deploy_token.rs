// src/commands/deploy_token.rs — deploy a new ERC-20 token via Clanker REST API
use crate::api::{self, DeployTokenRequest, PoolConfig, RewardConfig, TokenConfig, VaultConfig};
use crate::onchainos;
use anyhow::{bail, Result};
use uuid::Uuid;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    chain_id: u64,
    api_key: &str,
    name: &str,
    symbol: &str,
    from: Option<&str>,
    image_url: Option<&str>,
    description: Option<&str>,
    vault_percentage: Option<u32>,
    vault_lockup_days: Option<u32>,
    dry_run: bool,
) -> Result<()> {
    // ── 1. Resolve wallet address ─────────────────────────────────────────
    let wallet = from
        .map(|s| s.to_string())
        .unwrap_or_else(|| onchainos::resolve_wallet(chain_id).unwrap_or_default());
    if wallet.is_empty() {
        bail!("Cannot determine wallet address — pass --from or ensure onchainos is logged in");
    }

    // ── 2. Validate API key ───────────────────────────────────────────────
    if api_key.is_empty() {
        bail!(
            "Clanker API key is required for deployment. \
             Pass --api-key <KEY> or set CLANKER_API_KEY env var. \
             Obtain a partner API key from the Clanker team."
        );
    }

    // ── 3. Build request ──────────────────────────────────────────────────
    let request_key = Uuid::new_v4().to_string().replace('-', "");

    let vault = match (vault_percentage, vault_lockup_days) {
        (Some(pct), Some(days)) => Some(VaultConfig {
            percentage: pct,
            lockup_duration: days,
            vesting_duration: None,
        }),
        _ => None,
    };

    let req = DeployTokenRequest {
        token: TokenConfig {
            name: name.to_string(),
            symbol: symbol.to_string(),
            token_admin: wallet.clone(),
            request_key: request_key.clone(),
            image: image_url.map(|s| s.to_string()),
            description: description.map(|s| s.to_string()),
        },
        rewards: vec![RewardConfig {
            admin: wallet.clone(),
            recipient: wallet.clone(),
            allocation: 100,
        }],
        chain_id: Some(chain_id),
        pool: Some(PoolConfig {
            pool_type: "standard".to_string(),
            initial_market_cap: None,
        }),
        vault,
    };

    // ── 4. Dry-run preview ────────────────────────────────────────────────
    if dry_run {
        let preview = serde_json::json!({
            "ok": true,
            "dry_run": true,
            "data": {
                "action": "deploy_token",
                "chain_id": chain_id,
                "name": name,
                "symbol": symbol,
                "token_admin": wallet,
                "reward_recipient": wallet,
                "request_key": request_key,
                "vault": req.vault,
                "api_endpoint": "POST https://www.clanker.world/api/tokens/deploy",
                "note": "Real deployment would be submitted after user confirmation"
            }
        });
        println!("{}", serde_json::to_string_pretty(&preview)?);
        return Ok(());
    }

    // ── 5. Call REST API ──────────────────────────────────────────────────
    // Deploy is a server-side on-chain tx (Clanker's deployer wallet).
    // User confirmation is expected to be handled by the agent before calling this command.
    let result = api::deploy_token(api_key, &req).await?;

    let success = result["success"].as_bool().unwrap_or(false);
    let expected_address = result["expectedAddress"]
        .as_str()
        .or_else(|| result["expected_address"].as_str())
        .unwrap_or("unknown");
    let message = result["message"].as_str().unwrap_or("");

    if !success {
        let output = serde_json::json!({
            "ok": false,
            "error": message,
            "raw": result
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
        return Ok(());
    }

    // Validate expected address format (should be 0x + 40 hex chars)
    if !expected_address.starts_with("0x") || expected_address.len() != 42 {
        eprintln!(
            "Warning: expectedAddress '{}' does not look like a valid EVM address",
            expected_address
        );
    }

    let output = serde_json::json!({
        "ok": true,
        "data": {
            "name": name,
            "symbol": symbol,
            "chain_id": chain_id,
            "expected_address": expected_address,
            "token_admin": wallet,
            "reward_recipient": wallet,
            "request_key": request_key,
            "message": message,
            "next_step": format!(
                "Token deployment enqueued. You can verify deployment at: \
                 https://basescan.org/address/{} (check back in ~30s)",
                expected_address
            )
        }
    });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
