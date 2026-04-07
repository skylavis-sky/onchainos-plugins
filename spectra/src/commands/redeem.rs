use anyhow::Result;
use serde_json::Value;

use crate::config::rpc_url;
use crate::onchainos::{
    decode_uint, encode_address, encode_uint256, eth_call,
    extract_tx_hash_or_err, resolve_wallet, wallet_contract_call,
};

pub async fn run(
    chain_id: u64,
    pt_address: &str,
    shares: &str,           // PT amount in wei
    receiver: Option<&str>,
    owner: Option<&str>,
    from: Option<&str>,
    slippage: f64,
    dry_run: bool,
) -> Result<Value> {
    let rpc = rpc_url(chain_id);

    // Resolve wallet
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        let w = resolve_wallet(chain_id).unwrap_or_default();
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet. Pass --from or ensure onchainos is logged in.");
        }
        w
    };
    let rcv = receiver.unwrap_or(&wallet);
    let own = owner.unwrap_or(&wallet);

    // Check maturity
    let maturity_hex = eth_call(rpc, pt_address, "0x204f83f9").await?;
    let maturity = decode_uint(&maturity_hex) as u64;
    let now_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let is_expired = maturity > 0 && now_ts >= maturity;
    let shares_u128: u128 = shares
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid shares amount: {}", shares))?;

    let (calldata, operation, estimated_out) = if is_expired {
        // Post-expiry: redeem(uint256 shares, address receiver, address owner, uint256 minAssets) => 0x9f40a7b3
        // previewRedeem(uint256) => 0x4cdad506
        let preview_calldata = format!("0x4cdad506{}", encode_uint256(shares_u128));
        let estimated = eth_call(rpc, pt_address, &preview_calldata)
            .await
            .map(|h| decode_uint(&h))
            .unwrap_or(0);
        let min_assets = (estimated as f64 * (1.0 - slippage)) as u128;

        let cd = format!(
            "0x9f40a7b3{}{}{}{}",
            encode_uint256(shares_u128),
            encode_address(rcv),
            encode_address(own),
            encode_uint256(min_assets)
        );
        (cd, "redeem (post-expiry)", estimated)
    } else {
        // Pre-expiry: withdraw(uint256 assets, address receiver, address owner) => 0xb460af94
        // User must also have equal YT balance. The assets param is underlying units, not PT shares.
        // Use previewRedeem as an estimate for assets returned.
        let preview_calldata = format!("0x4cdad506{}", encode_uint256(shares_u128));
        let estimated = eth_call(rpc, pt_address, &preview_calldata)
            .await
            .map(|h| decode_uint(&h))
            .unwrap_or(shares_u128); // fallback: 1:1

        // For withdraw, assets = amount of underlying to receive (not PT shares).
        // Apply slippage to the estimated assets.
        let assets_with_slippage = (estimated as f64 * (1.0 - slippage)) as u128;

        let cd = format!(
            "0xb460af94{}{}{}",
            encode_uint256(assets_with_slippage),
            encode_address(rcv),
            encode_address(own)
        );
        (cd, "withdraw (pre-expiry, requires equal YT)", estimated)
    };

    let tx_result = wallet_contract_call(
        chain_id,
        pt_address,
        &calldata,
        Some(&wallet),
        None,
        true,
        dry_run,
    )
    .await?;
    let tx_hash = extract_tx_hash_or_err(&tx_result);

    Ok(serde_json::json!({
        "ok": true,
        "operation": operation,
        "chain_id": chain_id,
        "pt": pt_address,
        "shares_raw": shares,
        "expired": is_expired,
        "maturity_ts": maturity,
        "estimated_underlying_out": estimated_out.to_string(),
        "slippage": slippage,
        "receiver": rcv,
        "owner": own,
        "wallet": wallet,
        "tx_hash": tx_hash,
        "calldata": calldata,
        "dry_run": dry_run
    }))
}
