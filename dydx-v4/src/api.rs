use anyhow::Result;
use serde_json::Value;

const INDEXER_BASE: &str = "https://indexer.dydx.trade/v4";

/// GET /perpetualMarkets — all perpetual markets
pub async fn get_markets() -> Result<Value> {
    let url = format!("{}/perpetualMarkets", INDEXER_BASE);
    let resp = reqwest::get(&url).await?.json::<Value>().await?;
    Ok(resp)
}

/// GET /orderbooks/perpetualMarket/{ticker} — L2 orderbook
pub async fn get_orderbook(market: &str) -> Result<Value> {
    let url = format!("{}/orderbooks/perpetualMarket/{}", INDEXER_BASE, market);
    let resp = reqwest::get(&url).await?.json::<Value>().await?;
    Ok(resp)
}

/// GET /perpetualPositions — open positions for an address
pub async fn get_positions(address: &str) -> Result<Value> {
    let url = format!(
        "{}/perpetualPositions?address={}&subaccountNumber=0&status=OPEN",
        INDEXER_BASE, address
    );
    let resp = reqwest::get(&url).await?.json::<Value>().await?;
    Ok(resp)
}

/// GET /addresses/{address}/subaccountNumber/0 — account balance/equity
pub async fn get_balance(address: &str) -> Result<Value> {
    let url = format!(
        "{}/addresses/{}/subaccountNumber/0",
        INDEXER_BASE, address
    );
    let resp = reqwest::get(&url).await?.json::<Value>().await?;
    Ok(resp)
}
