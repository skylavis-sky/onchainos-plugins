/// Polymarket authentication helpers.
///
/// L1: EIP-712 ClobAuth signing via `onchainos wallet sign-message` → derive API keys
/// L2: HMAC-SHA256 request signing
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;

use crate::config::{save_credentials, Credentials, Urls};
use crate::signing::sign_clob_auth_eip712;

// ─── L1: ClobAuth EIP-712 ────────────────────────────────────────────────────

/// Build and sign a L1 ClobAuth EIP-712 message via onchainos.
/// Returns (address, signature_hex, timestamp, nonce).
pub async fn build_l1_auth(wallet_address: &str, nonce: u64) -> Result<(String, String, u64, u64)> {
    sign_clob_auth_eip712(wallet_address, nonce).await
}

/// Build L1 HTTP headers.
pub fn l1_headers(address: &str, sig: &str, timestamp: u64, nonce: u64) -> Vec<(String, String)> {
    vec![
        ("POLY_ADDRESS".to_string(), address.to_string()),
        ("POLY_SIGNATURE".to_string(), sig.to_string()),
        ("POLY_TIMESTAMP".to_string(), timestamp.to_string()),
        ("POLY_NONCE".to_string(), nonce.to_string()),
    ]
}

// ─── L2: HMAC-SHA256 ─────────────────────────────────────────────────────────

type HmacSha256 = Hmac<Sha256>;

/// Compute HMAC-SHA256 signature for a CLOB API request.
/// message = timestamp + method.to_uppercase() + request_path + body
/// Returns base64url-encoded signature.
pub fn hmac_signature(secret_b64url: &str, timestamp: u64, method: &str, path: &str, body: &str) -> Result<String> {
    let secret_bytes = general_purpose::URL_SAFE_NO_PAD
        .decode(secret_b64url)
        .with_context(|| "decoding base64url secret")?;

    let message = format!("{}{}{}{}", timestamp, method.to_uppercase(), path, body);

    let mut mac = HmacSha256::new_from_slice(&secret_bytes)
        .context("creating HMAC")?;
    mac.update(message.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(general_purpose::URL_SAFE_NO_PAD.encode(result))
}

/// Build L2 HTTP headers for an authenticated CLOB request.
pub fn l2_headers(
    address: &str,
    api_key: &str,
    secret: &str,
    passphrase: &str,
    method: &str,
    path: &str,
    body: &str,
) -> Result<Vec<(String, String)>> {
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let sig = hmac_signature(secret, timestamp, method, path, body)?;
    Ok(vec![
        ("POLY_ADDRESS".to_string(), address.to_string()),
        ("POLY_SIGNATURE".to_string(), sig),
        ("POLY_TIMESTAMP".to_string(), timestamp.to_string()),
        ("POLY_API_KEY".to_string(), api_key.to_string()),
        ("POLY_PASSPHRASE".to_string(), passphrase.to_string()),
    ])
}

// ─── API key management ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ApiKeyResponse {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub secret: String,
    pub passphrase: String,
}

/// Create new API keys using L1 auth. Stores them persistently.
pub async fn create_api_key(
    client: &Client,
    wallet_address: &str,
    nonce: u64,
) -> Result<Credentials> {
    let (address, sig, timestamp, nonce_used) = build_l1_auth(wallet_address, nonce).await?;
    let headers = l1_headers(&address, &sig, timestamp, nonce_used);

    let mut req = client.post(format!("{}/auth/api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if let Some(err) = resp.get("error").and_then(|e| e.as_str()) {
        anyhow::bail!(
            "Polymarket API key creation failed: {}\n\
             \n\
             Polymarket requires standard EIP-712 wallet signatures (ClobAuth).\n\
             To use buy/sell commands, set credentials via environment variables:\n\
             \n\
             export POLYMARKET_API_KEY=<your-api-key>\n\
             export POLYMARKET_SECRET=<your-secret>\n\
             export POLYMARKET_PASSPHRASE=<your-passphrase>\n\
             \n\
             Generate credentials using: pip install py-clob-client && python3 -c \"\n\
             from py_clob_client.client import ClobClient\n\
             client = ClobClient('https://clob.polymarket.com', key='<YOUR_PRIVATE_KEY>', chain_id=137)\n\
             creds = client.create_or_derive_api_creds()\n\
             print('API_KEY:', creds.api_key)\n\
             print('SECRET:', creds.api_secret)\n\
             print('PASSPHRASE:', creds.passphrase)\n\
             \"",
            err
        );
    }

    let api_key_resp: ApiKeyResponse = serde_json::from_value(resp)
        .context("parsing api-key response")?;

    let creds = Credentials {
        api_key: api_key_resp.api_key,
        secret: api_key_resp.secret,
        passphrase: api_key_resp.passphrase,
        nonce,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Derive existing API keys using L1 auth + same nonce.
pub async fn derive_api_key(
    client: &Client,
    wallet_address: &str,
    nonce: u64,
) -> Result<Credentials> {
    let (address, sig, timestamp, _) = build_l1_auth(wallet_address, nonce).await?;
    let headers = l1_headers(&address, &sig, timestamp, nonce);

    let mut req = client.get(format!("{}/auth/derive-api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

    if resp.get("error").is_some() {
        anyhow::bail!("derive-api-key rejected: {}", resp);
    }

    let api_key_resp: ApiKeyResponse = serde_json::from_value(resp)
        .context("parsing derive-api-key response")?;

    let creds = Credentials {
        api_key: api_key_resp.api_key,
        secret: api_key_resp.secret,
        passphrase: api_key_resp.passphrase,
        nonce,
    };
    save_credentials(&creds)?;
    Ok(creds)
}

/// Load stored credentials or derive them from the onchainos wallet.
pub async fn ensure_credentials(client: &Client, wallet_address: &str) -> Result<Credentials> {
    // Check environment variables first
    let env_key = std::env::var("POLYMARKET_API_KEY").unwrap_or_default();
    let env_secret = std::env::var("POLYMARKET_SECRET").unwrap_or_default();
    let env_pass = std::env::var("POLYMARKET_PASSPHRASE").unwrap_or_default();

    if !env_key.is_empty() && !env_secret.is_empty() && !env_pass.is_empty() {
        return Ok(Credentials {
            api_key: env_key,
            secret: env_secret,
            passphrase: env_pass,
            nonce: 0,
        });
    }

    // Try loading from file
    if let Some(creds) = crate::config::load_credentials()? {
        return Ok(creds);
    }

    // Attempt auto-derivation via onchainos sign-message
    // NOTE: This may fail if onchainos's EIP-712 signing is incompatible with
    // Polymarket's ClobAuth verification. In that case, set credentials manually
    // via POLYMARKET_API_KEY / POLYMARKET_SECRET / POLYMARKET_PASSPHRASE env vars.
    eprintln!("[polymarket] No stored credentials found, attempting to derive API keys...");
    match derive_api_key(client, wallet_address, 0).await {
        Ok(c) => Ok(c),
        Err(_) => {
            create_api_key(client, wallet_address, 0).await
        }
    }
}
