// commands/get_vaults.rs — List Loopscale vaults with APY and TVL
use anyhow::Result;
use serde_json::json;

use crate::api;
use crate::config::{cbps_to_pct, MINT_USDC, MINT_WSOL, VAULT_SOL_PRIMARY};

/// The deposits API does not return principalMint — we infer it from known vault addresses.
/// The SOL vault is the only one known; everything else defaults to USDC.
fn infer_token(vault_addr: &str) -> (&'static str, &'static str) {
    if vault_addr == VAULT_SOL_PRIMARY {
        ("SOL", MINT_WSOL)
    } else {
        ("USDC", MINT_USDC)
    }
}

pub async fn run(token: Option<String>) -> Result<()> {
    let resp = api::get_vaults(vec![]).await?;

    // The response is an array of vault objects
    let vaults_raw = if let Some(arr) = resp.as_array() {
        arr.clone()
    } else if let Some(arr) = resp["vaults"].as_array() {
        arr.clone()
    } else {
        // Unexpected format — return raw
        println!("{}", json!({ "ok": true, "raw": resp }));
        return Ok(());
    };

    let token_filter = token.as_deref().map(|t| t.to_uppercase());

    let mut vaults: Vec<serde_json::Value> = vaults_raw
        .iter()
        .filter_map(|v| {
            let address = v["vaultAddress"].as_str()
                .or_else(|| v["address"].as_str())
                .unwrap_or("unknown");

            let (token_symbol, principal_mint) = infer_token(address);

            // Apply token filter if provided
            if let Some(ref filter) = token_filter {
                if filter != token_symbol && filter != principal_mint {
                    return None;
                }
            }

            // TVL: sum of userDeposits[].amountSupplied
            let tvl_lamports: u64 = v["userDeposits"]
                .as_array()
                .map(|deps| {
                    deps.iter().filter_map(|d| {
                        d["amountSupplied"].as_u64()
                            .or_else(|| d["amountSupplied"].as_str().and_then(|s| s.parse().ok()))
                    }).sum()
                })
                .unwrap_or(0);

            let decimals: f64 = if token_symbol == "SOL" { 1e9 } else { 1e6 };
            let tvl_ui = tvl_lamports as f64 / decimals;

            // APY from apy field (cBPS) — may not be present in deposits endpoint
            let apy_cbps = v["apy"].as_u64()
                .or_else(|| v["estimatedApy"].as_u64())
                .unwrap_or(0);
            let apy_pct = cbps_to_pct(apy_cbps);

            let depositor_count = v["userDeposits"].as_array().map(|a| a.len()).unwrap_or(0);

            Some(json!({
                "vault_address": address,
                "token": token_symbol,
                "principal_mint": principal_mint,
                "tvl": tvl_ui,
                "tvl_display": format!("{:.2} {}", tvl_ui, token_symbol),
                "apy_pct": if apy_cbps > 0 { format!("{:.2}%", apy_pct) } else { "n/a (query borrow quotes for rates)".to_string() },
                "apy_cbps": apy_cbps,
                "depositors": depositor_count
            }))
        })
        .collect();

    // Sort by TVL descending
    vaults.sort_by(|a, b| {
        let ta = a["tvl"].as_f64().unwrap_or(0.0);
        let tb = b["tvl"].as_f64().unwrap_or(0.0);
        tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Count by type
    let vaults_len = vaults.len();

    println!("{}", json!({
        "ok": true,
        "data": {
            "vaults": vaults,
            "count": vaults_len,
            "note": "TVL is sum of all depositor amounts. APY field not available from this endpoint — use get-quotes for borrow rates."
        }
    }));
    Ok(())
}
