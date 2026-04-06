/// CREATE2 address prediction and salt grinding for Flap token creation.
///
/// Flap requires the predicted token address to end in a specific vanity suffix:
/// - Standard tokens (tokenVersion=1): address must end in `8888`
/// - Tax tokens (tokenVersion=4/5/6): address must end in `7777`
///
/// Formula (EIP-1014):
///   address = keccak256(0xff ++ deployer ++ salt ++ keccak256(initcode))[12:]
///
/// For Flap, the deployer is the Portal contract and the initcode is a minimal proxy
/// (ERC-1167 clone) of the token implementation address.

use tiny_keccak::{Hasher, Keccak};

const PORTAL_ADDR_BYTES: [u8; 20] = [
    0xe2, 0xce, 0x6a, 0xb8, 0x08, 0x74, 0xfa, 0x9f, 0xa2, 0xaa,
    0xe6, 0x5d, 0x27, 0x7d, 0xd6, 0xb8, 0xe6, 0x5c, 0x9d, 0xe0,
];

/// ERC-1167 minimal proxy initcode for a given implementation address.
/// The initcode is: 0x3d602d80600a3d3981f3363d3d373d3d3d363d73{impl}5af43d82803e903d91602b57fd5bf3
/// This is the standard ERC-1167 clone factory bytecode.
fn erc1167_initcode(impl_addr: &[u8; 20]) -> Vec<u8> {
    let mut code = Vec::new();
    // Creation code prefix
    code.extend_from_slice(&hex::decode("3d602d80600a3d3981f3363d3d373d3d3d363d73").unwrap());
    // Implementation address (20 bytes)
    code.extend_from_slice(impl_addr);
    // Creation code suffix
    code.extend_from_slice(&hex::decode("5af43d82803e903d91602b57fd5bf3").unwrap());
    code
}

fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Keccak::v256();
    hasher.update(data);
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    out
}

/// Predict the CREATE2 address for a given salt and implementation address.
pub fn predict_create2_address(salt: &[u8; 32], impl_addr: &[u8; 20]) -> [u8; 20] {
    let initcode = erc1167_initcode(impl_addr);
    let initcode_hash = keccak256(&initcode);

    let mut input = Vec::with_capacity(85);
    input.push(0xff);
    input.extend_from_slice(&PORTAL_ADDR_BYTES);
    input.extend_from_slice(salt);
    input.extend_from_slice(&initcode_hash);

    let hash = keccak256(&input);

    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);
    addr
}

/// Vanity suffix type.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VanitySuffix {
    /// Standard token: address ends in `8888` (last 2 bytes = 0x88, 0x88)
    Standard,
    /// Tax token: address ends in `7777` (last 2 bytes = 0x77, 0x77)
    Tax,
}

impl VanitySuffix {
    pub fn matches(&self, addr: &[u8; 20]) -> bool {
        match self {
            VanitySuffix::Standard => addr[18] == 0x88 && addr[19] == 0x88,
            VanitySuffix::Tax => addr[18] == 0x77 && addr[19] == 0x77,
        }
    }

    pub fn suffix_str(&self) -> &'static str {
        match self {
            VanitySuffix::Standard => "8888",
            VanitySuffix::Tax => "7777",
        }
    }
}

/// Parse a 20-byte implementation address from a hex string.
pub fn parse_impl_addr(s: &str) -> anyhow::Result<[u8; 20]> {
    let s = s.trim_start_matches("0x");
    if s.len() != 40 {
        anyhow::bail!("Invalid address length for impl: {}", s);
    }
    let bytes = hex::decode(s)?;
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

/// Grind for a salt (u256 as [u8;32]) such that the predicted CREATE2 address has the
/// desired vanity suffix. Returns (salt, predicted_address_hex).
///
/// Iterates salt as a little-endian counter starting from 0.
/// Expected iterations: ~65,536 on average (probability 1/65536 per trial for 2-byte suffix).
pub fn grind_salt(impl_addr: &[u8; 20], suffix: VanitySuffix) -> ([u8; 32], String) {
    let mut salt = [0u8; 32];
    let mut counter: u64 = 0;

    loop {
        // Write counter into salt as little-endian u64 in the first 8 bytes
        let ctr_bytes = counter.to_le_bytes();
        salt[0..8].copy_from_slice(&ctr_bytes);

        let addr = predict_create2_address(&salt, impl_addr);
        if suffix.matches(&addr) {
            let addr_hex = format!("0x{}", hex::encode(addr));
            return (salt, addr_hex);
        }

        counter = counter.wrapping_add(1);
    }
}
