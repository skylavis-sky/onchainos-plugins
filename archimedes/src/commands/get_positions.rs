use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::format_18;
use crate::config::{CD_POSITION, POSITION_TOKEN, RPC_URL};
use crate::onchainos;
use crate::rpc;

/// Query all PositionToken NFTs held by the wallet and their CDPosition details.
///
/// Flow:
/// 1. Resolve wallet address (from --wallet arg or onchainos)
/// 2. Call PositionToken.getTokenIDsArray(wallet)
/// 3. For each tokenId, call CDPosition: getOUSDPrinciple, getOUSDInterestEarned,
///    getLvUSDBorrowed, getPositionExpireTime, getOUSDTotalIncludeInterest
pub async fn run(wallet: Option<&str>) -> anyhow::Result<Value> {
    let wallet_addr = if let Some(addr) = wallet {
        addr.to_string()
    } else {
        onchainos::resolve_wallet().context("Could not resolve wallet address")?
    };

    // Get all NFT token IDs
    let token_ids = rpc::get_token_ids_array(POSITION_TOKEN, &wallet_addr, RPC_URL)
        .await
        .context("Failed to fetch PositionToken IDs")?;

    if token_ids.is_empty() {
        return Ok(json!({
            "ok": true,
            "wallet": wallet_addr,
            "positionCount": 0,
            "positions": []
        }));
    }

    let mut positions = Vec::new();

    for nft_id in &token_ids {
        let id = *nft_id;

        // Fetch position details in parallel
        let principle_fut = rpc::get_ousd_principle(CD_POSITION, id, RPC_URL);
        let interest_fut = rpc::get_ousd_interest_earned(CD_POSITION, id, RPC_URL);
        let total_fut = rpc::get_ousd_total_include_interest(CD_POSITION, id, RPC_URL);
        let lvusd_fut = rpc::get_lvusd_borrowed(CD_POSITION, id, RPC_URL);
        let expire_fut = rpc::get_position_expire_time(CD_POSITION, id, RPC_URL);

        let (principle, interest, total, lvusd, expire) = tokio::join!(
            principle_fut,
            interest_fut,
            total_fut,
            lvusd_fut,
            expire_fut,
        );

        let principle = principle.unwrap_or(0);
        let interest = interest.unwrap_or(0);
        let total = total.unwrap_or(0);
        let lvusd = lvusd.unwrap_or(0);
        let expire = expire.unwrap_or(0);

        positions.push(json!({
            "tokenId": id.to_string(),
            "ousdPrinciple": format_18(principle),
            "ousdPrincipleRaw": principle.to_string(),
            "ousdInterestEarned": format_18(interest),
            "ousdInterestEarnedRaw": interest.to_string(),
            "ousdTotalWithInterest": format_18(total),
            "ousdTotalWithInterestRaw": total.to_string(),
            "lvUSDBorrowed": format_18(lvusd),
            "lvUSDBorrowedRaw": lvusd.to_string(),
            "expireTimestamp": expire.to_string(),
        }));
    }

    Ok(json!({
        "ok": true,
        "wallet": wallet_addr,
        "positionCount": positions.len(),
        "positions": positions
    }))
}
