/// EIP-712 order signing for Polymarket CTF Exchange.
///
/// Uses `onchainos wallet sign-message --type eip712` — no raw private key needed.
use anyhow::{Context, Result};

/// Parameters for an EIP-712 order.
pub struct OrderParams {
    pub salt: u128,
    pub maker: String,
    pub signer: String,
    pub taker: String,
    pub token_id: String,
    pub maker_amount: u64,
    pub taker_amount: u64,
    pub expiration: u64,
    pub nonce: u64,
    pub fee_rate_bps: u64,
    pub side: u8,           // 0=BUY, 1=SELL
    pub signature_type: u8, // 0=EOA
}

/// Build the EIP-712 JSON payload for an Order and sign it via onchainos.
/// Returns 0x-prefixed hex signature.
pub async fn sign_order_eip712(
    order: &OrderParams,
    wallet_address: &str,
    neg_risk: bool,
) -> Result<String> {
    use crate::config::Contracts;
    let verifying_contract = Contracts::exchange_for(neg_risk);

    let payload = serde_json::json!({
        "domain": {
            "name": "Polymarket CTF Exchange",
            "version": "1",
            "chainId": 137,
            "verifyingContract": verifying_contract
        },
        "types": {
            "Order": [
                {"name": "salt",          "type": "uint256"},
                {"name": "maker",         "type": "address"},
                {"name": "signer",        "type": "address"},
                {"name": "taker",         "type": "address"},
                {"name": "tokenId",       "type": "uint256"},
                {"name": "makerAmount",   "type": "uint256"},
                {"name": "takerAmount",   "type": "uint256"},
                {"name": "expiration",    "type": "uint256"},
                {"name": "nonce",         "type": "uint256"},
                {"name": "feeRateBps",    "type": "uint256"},
                {"name": "side",          "type": "uint8"},
                {"name": "signatureType", "type": "uint8"}
            ]
        },
        "primaryType": "Order",
        "message": {
            "salt":          order.salt.to_string(),
            "maker":         order.maker.clone(),
            "signer":        order.signer.clone(),
            "taker":         order.taker.clone(),
            "tokenId":       order.token_id.clone(),
            "makerAmount":   order.maker_amount.to_string(),
            "takerAmount":   order.taker_amount.to_string(),
            "expiration":    order.expiration.to_string(),
            "nonce":         order.nonce.to_string(),
            "feeRateBps":    order.fee_rate_bps.to_string(),
            "side":          order.side,
            "signatureType": order.signature_type
        }
    });

    let message_json = payload.to_string();
    sign_eip712_via_onchainos(wallet_address, &message_json).await
}

/// Call `onchainos wallet sign-message --type eip712 --chain 137 --from <addr> --message <json> --force`
/// Returns the 0x-prefixed signature string.
pub async fn sign_eip712_via_onchainos(wallet_address: &str, message_json: &str) -> Result<String> {
    let output = tokio::process::Command::new("onchainos")
        .args([
            "wallet",
            "sign-message",
            "--type",
            "eip712",
            "--chain",
            "137",
            "--from",
            wallet_address,
            "--message",
            message_json,
            "--force",
        ])
        .output()
        .await
        .context("spawning onchainos wallet sign-message")?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let v: serde_json::Value = serde_json::from_str(&stdout)
        .with_context(|| format!("parsing sign-message response: {}", stdout))?;

    if v["ok"].as_bool() != Some(true) {
        let msg = v["error"]
            .as_str()
            .or_else(|| v["message"].as_str())
            .unwrap_or("unknown error");
        return Err(anyhow::anyhow!("sign-message failed: {}", msg));
    }

    v["data"]["signature"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("no signature in sign-message response: {}", stdout))
}

/// Build the ClobAuth EIP-712 payload and sign it via onchainos.
/// Returns (address, signature_hex, timestamp, nonce).
pub async fn sign_clob_auth_eip712(
    wallet_address: &str,
    nonce: u64,
) -> Result<(String, String, u64, u64)> {
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let message_text = "This message attests that I control the given wallet";

    let payload = serde_json::json!({
        "domain": {
            "name": "ClobAuthDomain",
            "version": "1",
            "chainId": 137
        },
        "types": {
            "ClobAuth": [
                {"name": "address",   "type": "address"},
                {"name": "timestamp", "type": "string"},
                {"name": "nonce",     "type": "uint256"},
                {"name": "message",   "type": "string"}
            ]
        },
        "primaryType": "ClobAuth",
        "message": {
            "address":   wallet_address,
            "timestamp": timestamp.to_string(),
            "nonce":     nonce,
            "message":   message_text
        }
    });

    let message_json = payload.to_string();
    let sig = sign_eip712_via_onchainos(wallet_address, &message_json).await?;
    Ok((wallet_address.to_string(), sig, timestamp, nonce))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_order_params_salt_string() {
        let p = OrderParams {
            salt: 255,
            maker: "0x0000000000000000000000000000000000000001".to_string(),
            signer: "0x0000000000000000000000000000000000000001".to_string(),
            taker: "0x0000000000000000000000000000000000000000".to_string(),
            token_id: "12345".to_string(),
            maker_amount: 1_000_000,
            taker_amount: 1_000_000,
            expiration: 0,
            nonce: 0,
            fee_rate_bps: 0,
            side: 0,
            signature_type: 0,
        };
        // Verify salt serialises correctly
        assert_eq!(p.salt.to_string(), "255");
    }
}
