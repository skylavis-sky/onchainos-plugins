/// EIP-712 order signing for Polymarket CTF Exchange.
///
/// Uses k256 for secp256k1 ECDSA and tiny-keccak for keccak256.
use anyhow::{Context, Result};
use k256::ecdsa::{RecoveryId, SigningKey};
#[allow(unused_imports)]
use k256::ecdsa::signature::hazmat::PrehashSigner;
use tiny_keccak::{Hasher, Keccak};

/// Compute keccak256 of input bytes.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut k = Keccak::v256();
    k.update(data);
    let mut out = [0u8; 32];
    k.finalize(&mut out);
    out
}

/// ABI-encode a sequence of 32-byte words (left-padded / right-padded as appropriate).
/// For EIP-712 struct encoding, every element is exactly 32 bytes.
fn abi_encode_words(words: &[[u8; 32]]) -> Vec<u8> {
    let mut out = Vec::with_capacity(words.len() * 32);
    for w in words {
        out.extend_from_slice(w);
    }
    out
}

fn u256_word(val: u128) -> [u8; 32] {
    let mut w = [0u8; 32];
    let bytes = val.to_be_bytes();
    w[16..32].copy_from_slice(&bytes);
    w
}

fn u256_from_bytes(bytes: &[u8]) -> [u8; 32] {
    let mut w = [0u8; 32];
    let start = 32usize.saturating_sub(bytes.len());
    let src_start = bytes.len().saturating_sub(32);
    w[start..].copy_from_slice(&bytes[src_start..]);
    w
}

pub fn address_word(hex_addr: &str) -> Result<[u8; 32]> {
    let clean = hex_addr.trim_start_matches("0x");
    let bytes = hex::decode(clean).with_context(|| format!("decoding address {}", hex_addr))?;
    anyhow::ensure!(bytes.len() == 20, "address must be 20 bytes: {}", hex_addr);
    let mut w = [0u8; 32];
    w[12..32].copy_from_slice(&bytes);
    Ok(w)
}

fn uint8_word(val: u8) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[31] = val;
    w
}

/// ORDER_TYPEHASH
/// keccak256("Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)")
fn order_typehash() -> [u8; 32] {
    keccak256(b"Order(uint256 salt,address maker,address signer,address taker,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint256 expiration,uint256 nonce,uint256 feeRateBps,uint8 side,uint8 signatureType)")
}

/// EIP-712 domain separator for CTF Exchange.
pub fn domain_separator(verifying_contract: &str) -> Result<[u8; 32]> {
    let domain_typehash = keccak256(
        b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
    );
    let name_hash = keccak256(b"Polymarket CTF Exchange");
    let version_hash = keccak256(b"1");
    let chain_id_word = u256_word(137);
    let contract_word = address_word(verifying_contract)?;

    let words = [
        domain_typehash,
        name_hash,
        version_hash,
        chain_id_word,
        contract_word,
    ];
    Ok(keccak256(&abi_encode_words(&words)))
}

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

/// Build the EIP-712 struct hash for an Order.
pub fn order_struct_hash(p: &OrderParams) -> Result<[u8; 32]> {
    let typehash = order_typehash();

    // token_id is a large uint256 string — parse as decimal
    let token_id_u256: u128 = p
        .token_id
        .parse::<u128>()
        .unwrap_or(0); // if very large, truncation is handled below

    // For very large token IDs that exceed u128, we need special handling
    let token_id_word = if p.token_id.len() > 38 {
        // Parse as big decimal and convert to 32 bytes
        decimal_str_to_u256_word(&p.token_id)?
    } else {
        u256_word(token_id_u256)
    };

    let words = [
        typehash,
        u256_word(p.salt),
        address_word(&p.maker)?,
        address_word(&p.signer)?,
        address_word(&p.taker)?,
        token_id_word,
        u256_word(p.maker_amount as u128),
        u256_word(p.taker_amount as u128),
        u256_word(p.expiration as u128),
        u256_word(p.nonce as u128),
        u256_word(p.fee_rate_bps as u128),
        uint8_word(p.side),
        uint8_word(p.signature_type),
    ];
    Ok(keccak256(&abi_encode_words(&words)))
}

/// Parse a decimal string that may exceed u128 into a 32-byte big-endian word.
fn decimal_str_to_u256_word(s: &str) -> Result<[u8; 32]> {
    // Compute s mod 2^256 by successive multiply-add
    let mut result = [0u8; 32]; // big-endian u256
    for ch in s.chars() {
        let digit = ch.to_digit(10).context("invalid decimal digit")? as u64;
        // result = result * 10 + digit
        let mut carry = digit;
        for i in (0..32).rev() {
            let prod = (result[i] as u64) * 10 + carry;
            result[i] = (prod & 0xff) as u8;
            carry = prod >> 8;
        }
    }
    Ok(result)
}

/// Compute the final EIP-712 digest to sign.
pub fn eip712_digest(domain_sep: &[u8; 32], struct_hash: &[u8; 32]) -> [u8; 32] {
    let mut data = Vec::with_capacity(66);
    data.push(0x19);
    data.push(0x01);
    data.extend_from_slice(domain_sep);
    data.extend_from_slice(struct_hash);
    keccak256(&data)
}

/// Sign a 32-byte digest with a secp256k1 private key.
/// Returns 65-byte signature: r(32) || s(32) || v(1) with v ∈ {27, 28}.
pub fn sign_digest(private_key_hex: &str, digest: &[u8; 32]) -> Result<Vec<u8>> {
    let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
        .context("decoding private key hex")?;
    let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into())
        .context("creating signing key")?;

    let (sig, recid): (k256::ecdsa::Signature, RecoveryId) = signing_key
        .sign_prehash_recoverable(digest)
        .context("signing digest")?;

    let sig_bytes = sig.to_bytes();
    let mut out = Vec::with_capacity(65);
    out.extend_from_slice(&sig_bytes);
    // Ethereum v = 27 + recovery_id
    out.push(27 + recid.to_byte());
    Ok(out)
}

/// High-level: sign an order and return 0x-prefixed hex signature.
pub fn sign_order(
    private_key_hex: &str,
    params: &OrderParams,
    neg_risk: bool,
) -> Result<String> {
    use crate::config::Contracts;
    let exchange = Contracts::exchange_for(neg_risk);
    let dom_sep = domain_separator(exchange)?;
    let struct_hash = order_struct_hash(params)?;
    let digest = eip712_digest(&dom_sep, &struct_hash);
    let sig_bytes = sign_digest(private_key_hex, &digest)?;
    Ok(format!("0x{}", hex::encode(sig_bytes)))
}

/// Derive Ethereum address from private key hex.
pub fn private_key_to_address(private_key_hex: &str) -> Result<String> {
    #[allow(unused_imports)]
    use k256::elliptic_curve::sec1::ToEncodedPoint;
    let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
        .context("decoding private key")?;
    let signing_key = SigningKey::from_bytes(key_bytes.as_slice().into())
        .context("creating signing key")?;
    let verifying_key = signing_key.verifying_key();
    let point = verifying_key.to_encoded_point(false); // uncompressed
    let pubkey_bytes = &point.as_bytes()[1..]; // skip 0x04 prefix → 64 bytes
    let hash = keccak256(pubkey_bytes);
    let addr_bytes = &hash[12..]; // last 20 bytes
    Ok(format!("0x{}", hex::encode(addr_bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decimal_str_to_u256_word_small() {
        let w = decimal_str_to_u256_word("255").unwrap();
        assert_eq!(w[31], 0xff);
    }

    #[test]
    fn test_address_word() {
        let w = address_word("0x4bFb41d5B3570DeFd03C39a9A4D8dE6Bd8B8982E").unwrap();
        assert_eq!(&w[..12], &[0u8; 12]);
        assert_eq!(w[12], 0x4b);
    }
}
