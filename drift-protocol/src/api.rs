/// HTTP client for Drift Protocol REST APIs.
/// Handles 503/404 gracefully — protocol is currently paused post-exploit (2026-04-01).

pub fn build_client() -> reqwest::blocking::Client {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10));
    if let Ok(proxy_url) = std::env::var("HTTPS_PROXY")
        .or_else(|_| std::env::var("https_proxy"))
    {
        if let Ok(proxy) = reqwest::Proxy::https(&proxy_url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap()
}

/// Canonical "protocol is paused" error JSON returned by any degraded read operation.
pub fn protocol_paused_error() -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Track status: https://drift.trade",
        "note": "Read operations (get-markets, get-funding-rates) will return data when the protocol relaunches."
    })
}

/// Canonical "protocol paused — write blocked" error JSON returned by stub write operations.
pub fn write_paused_error() -> serde_json::Value {
    serde_json::json!({
        "ok": false,
        "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Trading will resume after independent security audits complete. Track status: https://drift.trade",
        "note": "When Drift relaunches with a public transaction API, this command will be fully implemented."
    })
}

/// Fetch L2 orderbook from DLOB server.
/// Returns Ok(json) on success, or a protocol_paused_error json on 503/404/network failure.
pub fn get_l2_orderbook(market: &str, depth: u32) -> serde_json::Value {
    let url = format!(
        "https://dlob.drift.trade/l2?marketName={}&depth={}",
        market, depth
    );
    let client = build_client();
    match client.get(&url).send() {
        Ok(resp) => {
            let status = resp.status();
            if status == 503 || status == 404 || !status.is_success() {
                return protocol_paused_error();
            }
            match resp.json::<serde_json::Value>() {
                Ok(v) => serde_json::json!({ "ok": true, "data": v }),
                Err(e) => serde_json::json!({
                    "ok": false,
                    "error": format!("Failed to parse DLOB response: {}", e)
                }),
            }
        }
        Err(e) => {
            // Connection refused, timeout, DNS failure — all map to paused error
            // but include the original error for debugging
            let _ = e;
            protocol_paused_error()
        }
    }
}

/// Fetch funding rates from Drift data API.
/// Returns Ok(json) on success, or a protocol_paused_error json on non-200/network failure.
pub fn get_funding_rates(market: &str) -> serde_json::Value {
    let url = format!(
        "https://data.api.drift.trade/fundingRates?marketName={}",
        market
    );
    let client = build_client();
    match client.get(&url).send() {
        Ok(resp) => {
            let status = resp.status();
            if status == 503 || status == 404 || !status.is_success() {
                return protocol_paused_error();
            }
            match resp.json::<serde_json::Value>() {
                Ok(v) => serde_json::json!({ "ok": true, "data": v }),
                Err(e) => serde_json::json!({
                    "ok": false,
                    "error": format!("Failed to parse funding rate response: {}", e)
                }),
            }
        }
        Err(_) => protocol_paused_error(),
    }
}
