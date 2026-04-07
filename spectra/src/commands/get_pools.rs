use anyhow::Result;
use serde_json::Value;

use crate::config::{registry_address, rpc_url, KNOWN_BASE_POOLS};
use crate::onchainos::{decode_uint, eth_call};

/// Fetch the current buildId from the Spectra app HTML and construct the pools.json URL
async fn fetch_spectra_build_id(client: &reqwest::Client) -> Result<String> {
    let html = client
        .get("https://app.spectra.finance/pools")
        .header("User-Agent", "Mozilla/5.0 (compatible; spectra-plugin/0.1)")
        .send()
        .await?
        .text()
        .await?;

    // Look for /_next/static/<buildId>/_buildManifest.js pattern
    if let Some(start) = html.find("/_next/static/") {
        let rest = &html[start + "/_next/static/".len()..];
        if let Some(end) = rest.find('/') {
            let build_id = &rest[..end];
            if !build_id.is_empty() && build_id.len() < 64 {
                return Ok(build_id.to_string());
            }
        }
    }
    anyhow::bail!("Could not extract buildId from app.spectra.finance");
}

/// Try to fetch pool list from the Spectra app Next.js data endpoint
async fn fetch_pools_from_api(client: &reqwest::Client, chain_id: u64) -> Result<Vec<Value>> {
    let build_id = fetch_spectra_build_id(client).await?;
    let url = format!(
        "https://app.spectra.finance/_next/data/{}/pools.json",
        build_id
    );
    let resp: Value = client
        .get(&url)
        .header("User-Agent", "Mozilla/5.0 (compatible; spectra-plugin/0.1)")
        .send()
        .await?
        .json()
        .await?;

    let pools = resp["pageProps"]["pools"]
        .as_array()
        .or_else(|| resp["pageProps"]["data"]["pools"].as_array())
        .cloned()
        .unwrap_or_default();

    let filtered: Vec<Value> = pools
        .into_iter()
        .filter(|p| {
            // filter by chainId if the field exists; default to include
            p["chainId"]
                .as_u64()
                .map(|c| c == chain_id)
                .unwrap_or(true)
        })
        .collect();

    Ok(filtered)
}

/// On-chain fallback: use KNOWN_BASE_POOLS + live maturity check per PT.
/// Also verify registry is live by calling pTCount().
async fn fetch_pools_onchain(chain_id: u64) -> Result<Vec<Value>> {
    let rpc = rpc_url(chain_id);
    let registry = registry_address(chain_id);

    // Verify registry is live — pTCount()
    let count_hex = eth_call(rpc, registry, "0x704bdadc").await?;
    let total_pt_count = decode_uint(&count_hex) as u64;

    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut pools: Vec<Value> = Vec::new();

    for known in KNOWN_BASE_POOLS {
        // Verify maturity live
        let maturity = eth_call(rpc, known.pt, "0x204f83f9")
            .await
            .map(|h| decode_uint(&h) as u64)
            .unwrap_or(known.maturity);

        let is_active = maturity > now_ts;
        let days_to_maturity = if maturity > now_ts {
            (maturity - now_ts) / 86400
        } else {
            0
        };

        pools.push(serde_json::json!({
            "name": known.name,
            "pt": known.pt,
            "yt": known.yt,
            "ibt": known.ibt,
            "underlying": known.underlying,
            "curve_pool": known.curve_pool,
            "maturity_ts": maturity,
            "days_to_maturity": days_to_maturity,
            "active": is_active,
            "chain_id": chain_id,
            "total_registered_pts": total_pt_count,
            "note": "on-chain fallback — showing top TVL pools; API unavailable"
        }));
    }

    Ok(pools)
}

pub async fn run(chain_id: u64, active_only: bool, limit: usize) -> Result<Value> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    // Try API first, fall back to on-chain
    let mut pools = match fetch_pools_from_api(&client, chain_id).await {
        Ok(p) if !p.is_empty() => {
            // Normalize API response to our schema
            p.into_iter()
                .map(|pool| {
                    let now_ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let maturity = pool["maturity"]
                        .as_u64()
                        .or_else(|| pool["expiryTimestamp"].as_u64())
                        .unwrap_or(0);
                    let is_active = maturity > now_ts;
                    let days = if maturity > now_ts { (maturity - now_ts) / 86400 } else { 0 };
                    serde_json::json!({
                        "name": pool["name"].as_str().unwrap_or("Unknown"),
                        "pt": pool["address"].as_str()
                            .or_else(|| pool["ptAddress"].as_str())
                            .unwrap_or(""),
                        "yt": pool["ytAddress"].as_str().unwrap_or(""),
                        "ibt": pool["ibtAddress"].as_str().unwrap_or(""),
                        "underlying": pool["underlyingAddress"].as_str().unwrap_or(""),
                        "curve_pool": pool["lpAddress"].as_str().unwrap_or(""),
                        "maturity_ts": maturity,
                        "days_to_maturity": days,
                        "active": is_active,
                        "apy": pool["apy"].as_f64().or_else(|| pool["impliedApy"].as_f64()),
                        "tvl_usd": pool["tvlInUnderlying"].as_f64().or_else(|| pool["tvl"].as_f64()),
                        "chain_id": chain_id,
                        "source": "api"
                    })
                })
                .collect::<Vec<Value>>()
        }
        _ => {
            // On-chain fallback
            let onchain = fetch_pools_onchain(chain_id).await?;
            onchain
                .into_iter()
                .map(|mut p| {
                    p["source"] = "onchain".into();
                    // Attach known curve pool if available
                    let pt = p["pt"].as_str().unwrap_or("").to_lowercase();
                    if let Some(known) = KNOWN_BASE_POOLS
                        .iter()
                        .find(|k| k.pt.to_lowercase() == pt)
                    {
                        p["curve_pool"] = known.curve_pool.into();
                    }
                    p
                })
                .collect()
        }
    };

    if active_only {
        pools.retain(|p| p["active"].as_bool().unwrap_or(true));
    }

    pools.truncate(limit);

    Ok(serde_json::json!({
        "ok": true,
        "chain_id": chain_id,
        "count": pools.len(),
        "pools": pools
    }))
}
