// commands/get_pools.rs — Fetch Fenix V3 pool list via Goldsky GraphQL
use crate::config;
use anyhow::Result;

pub async fn run(limit: usize) -> Result<()> {
    let client = reqwest::Client::new();

    let query = format!(
        r#"{{ pools(first: {}, orderBy: totalValueLockedUSD, orderDirection: desc) {{ id token0 {{ symbol address }} token1 {{ symbol address }} totalValueLockedUSD feesUSD volumeUSD }} }}"#,
        limit
    );

    let body = serde_json::json!({ "query": query });

    let http_resp = client
        .post(config::GRAPHQL_URL)
        .json(&body)
        .send()
        .await?;

    if !http_resp.status().is_success() {
        println!(
            "{}",
            serde_json::json!({
                "ok": false,
                "error": "Fenix subgraph unavailable (Goldsky endpoint returned HTTP {}). Use get-quote to check prices directly.",
                "subgraph_url": config::GRAPHQL_URL
            })
        );
        return Ok(());
    }

    let resp: serde_json::Value = http_resp.json().await?;

    if let Some(errors) = resp.get("errors") {
        anyhow::bail!("GraphQL error: {}", errors);
    }

    let pools = resp["data"]["pools"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("No pools data in GraphQL response"))?;

    let display: Vec<_> = pools
        .iter()
        .map(|p| {
            let tvl: f64 = p["totalValueLockedUSD"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let fees: f64 = p["feesUSD"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let volume: f64 = p["volumeUSD"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            serde_json::json!({
                "address": p["id"],
                "token0": {
                    "symbol": p["token0"]["symbol"],
                    "address": p["token0"]["address"]
                },
                "token1": {
                    "symbol": p["token1"]["symbol"],
                    "address": p["token1"]["address"]
                },
                "tvl_usd": format!("{:.2}", tvl),
                "fees_usd": format!("{:.2}", fees),
                "volume_usd": format!("{:.2}", volume)
            })
        })
        .collect();

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "chain": "blast",
            "chain_id": 81457,
            "count": display.len(),
            "pools": display
        })
    );
    Ok(())
}
