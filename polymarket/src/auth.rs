/// Polymarket authentication helpers.
///
/// L1: EIP-712 ClobAuth signing → derive API keys
/// L2: HMAC-SHA256 request signing
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;

use crate::config::{save_credentials, Credentials, Urls};
use crate::signing::{eip712_digest, keccak256, private_key_to_address, sign_digest};

// ─── L1: ClobAuth EIP-712 ────────────────────────────────────────────────────

/// CLOB_AUTH_TYPEHASH
/// keccak256("ClobAuth(address address,string timestamp,uint256 nonce,string message)")
fn clob_auth_typehash() -> [u8; 32] {
    keccak256(b"ClobAuth(address address,string timestamp,uint256 nonce,string message)")
}

fn clob_auth_domain_separator() -> [u8; 32] {
    let domain_typehash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId)",
    );
    let name_hash = keccak256(b"ClobAuthDomain");
    let version_hash = keccak256(b"1");

    let mut words: Vec<[u8; 32]> = Vec::new();
    words.push(domain_typehash);
    words.push(name_hash);
    words.push(version_hash);
    // chainId = 137
    let mut chain_word = [0u8; 32];
    chain_word[31] = 137;
    words.push(chain_word);

    let mut encoded = Vec::with_capacity(words.len() * 32);
    for w in &words {
        encoded.extend_from_slice(w);
    }
    keccak256(&encoded)
}

fn address_word_local(hex_addr: &str) -> Result<[u8; 32]> {
    let clean = hex_addr.trim_start_matches("0x");
    let bytes = hex::decode(clean).with_context(|| format!("decoding address {}", hex_addr))?;
    anyhow::ensure!(bytes.len() == 20, "address must be 20 bytes");
    let mut w = [0u8; 32];
    w[12..32].copy_from_slice(&bytes);
    Ok(w)
}

fn u256_word_local(val: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    let bytes = val.to_be_bytes();
    w[24..32].copy_from_slice(&bytes);
    w
}

/// Build and sign a L1 ClobAuth EIP-712 message.
/// Returns (address, signature_hex, timestamp, nonce).
pub fn build_l1_auth(private_key_hex: &str, nonce: u64) -> Result<(String, String, u64, u64)> {
    let address = private_key_to_address(private_key_hex)?;
    let timestamp = chrono::Utc::now().timestamp() as u64;
    let message = "This message attests that I control the given wallet";

    let typehash = clob_auth_typehash();
    let domain_sep = clob_auth_domain_separator();

    // Struct fields
    let addr_word = address_word_local(&address)?;
    let timestamp_str = timestamp.to_string();
    let timestamp_hash = keccak256(timestamp_str.as_bytes());
    let nonce_word = u256_word_local(nonce);
    let message_hash = keccak256(message.as_bytes());

    let mut words: Vec<[u8; 32]> = Vec::new();
    words.push(typehash);
    words.push(addr_word);
    words.push(timestamp_hash);
    words.push(nonce_word);
    words.push(message_hash);

    let mut encoded = Vec::with_capacity(words.len() * 32);
    for w in &words {
        encoded.extend_from_slice(w);
    }
    let struct_hash = keccak256(&encoded);

    let digest = eip712_digest(&domain_sep, &struct_hash);
    let sig_bytes = sign_digest(private_key_hex, &digest)?;
    let sig_hex = format!("0x{}", hex::encode(sig_bytes));

    Ok((address, sig_hex, timestamp, nonce))
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
    private_key_hex: &str,
    nonce: u64,
) -> Result<Credentials> {
    let (address, sig, timestamp, nonce_used) = build_l1_auth(private_key_hex, nonce)?;
    let headers = l1_headers(&address, &sig, timestamp, nonce_used);

    let mut req = client.post(format!("{}/auth/api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

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
    private_key_hex: &str,
    nonce: u64,
) -> Result<Credentials> {
    let (address, sig, timestamp, _) = build_l1_auth(private_key_hex, nonce)?;
    let headers = l1_headers(&address, &sig, timestamp, nonce);

    let mut req = client.get(format!("{}/auth/derive-api-key", Urls::CLOB));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }
    let resp: serde_json::Value = req.send().await?.json().await?;

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

/// Load stored credentials or derive them from the private key.
pub async fn ensure_credentials(client: &Client, private_key_hex: &str) -> Result<(String, Credentials)> {
    let address = private_key_to_address(private_key_hex)?;

    // Check environment variables first
    let env_key = std::env::var("POLYMARKET_API_KEY").unwrap_or_default();
    let env_secret = std::env::var("POLYMARKET_SECRET").unwrap_or_default();
    let env_pass = std::env::var("POLYMARKET_PASSPHRASE").unwrap_or_default();

    if !env_key.is_empty() && !env_secret.is_empty() && !env_pass.is_empty() {
        let creds = Credentials {
            api_key: env_key,
            secret: env_secret,
            passphrase: env_pass,
            nonce: 0,
        };
        return Ok((address, creds));
    }

    // Try loading from file
    if let Some(creds) = crate::config::load_credentials()? {
        return Ok((address, creds));
    }

    // Derive new keys
    eprintln!("[polymarket] No stored credentials found, deriving API keys...");
    let creds = derive_api_key(client, private_key_hex, 0).await
        .or_else(|_| {
            // If derive fails (key may not exist yet), create new ones
            // Use blocking context is not ideal in async, so we use a workaround
            Err(anyhow::anyhow!("derive failed"))
        });

    match creds {
        Ok(c) => Ok((address, c)),
        Err(_) => {
            let c = create_api_key(client, private_key_hex, 0).await?;
            Ok((address, c))
        }
    }
}
