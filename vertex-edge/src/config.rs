/// Per-chain configuration for Vertex Edge.
///
/// Endpoint contract addresses verified from vertex-rust-sdk deployment.json files.
/// Vertex uses a single Endpoint contract per chain for all collateral deposits.
/// Orders go through the off-chain engine gateway (REST API, no on-chain tx).
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub name: &'static str,
    pub gateway_url: &'static str,
    pub archive_url: &'static str,
    pub endpoint_contract: &'static str,
    pub usdc_address: &'static str,
}

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 42161,
        name: "Arbitrum One",
        gateway_url: "https://gateway.prod.vertexprotocol.com/v1",
        archive_url: "https://archive.prod.vertexprotocol.com/v1",
        endpoint_contract: "0xbbEE07B3e8121227AfCFe1E2B82772246226128e",
        usdc_address: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
    },
    ChainConfig {
        chain_id: 8453,
        name: "Base",
        gateway_url: "https://gateway.base-prod.vertexprotocol.com/v1",
        archive_url: "https://archive.base-prod.vertexprotocol.com/v1",
        endpoint_contract: "0x92C2201D48481e2d42772Da02485084A4407Bbe2",
        usdc_address: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
    },
    ChainConfig {
        chain_id: 5000,
        name: "Mantle",
        gateway_url: "https://gateway.mantle-prod.vertexprotocol.com/v1",
        archive_url: "https://archive.mantle-prod.vertexprotocol.com/v1",
        endpoint_contract: "0x526D7C7ea3677efF28CB5bA457f9d341F297Fd52",
        usdc_address: "0x09Bc4E0D864854c6aFB6eB9A9cdF58aC190D0dF9",
    },
    ChainConfig {
        chain_id: 1329,
        name: "Sei",
        gateway_url: "https://gateway.sei-prod.vertexprotocol.com/v1",
        archive_url: "https://archive.sei-prod.vertexprotocol.com/v1",
        endpoint_contract: "0x2777268EeE0d224F99013Bc4af24ec756007f1a6",
        usdc_address: "0x3894085Ef7Ff0f0aeDf52E2A2704928d1Ec074F1",
    },
    ChainConfig {
        chain_id: 146,
        name: "Sonic",
        gateway_url: "https://gateway.sonic-prod.vertexprotocol.com/v1",
        archive_url: "https://archive.sonic-prod.vertexprotocol.com/v1",
        endpoint_contract: "0x2f5F835d778eBE8c28fC743E50EB9a68Ca93c2Fa",
        usdc_address: "0x29219dd400f2Bf60E5a23d13Be72B486D4038894",
    },
];

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unsupported chain ID: {}. Supported chains: {}",
                chain_id,
                CHAINS
                    .iter()
                    .map(|c| format!("{} ({})", c.name, c.chain_id))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        })
}

/// Default subaccount name used by Vertex: "default" right-padded to 12 bytes with null bytes.
/// The sender field in all Vertex requests is 32 bytes: 20-byte address + 12-byte subaccount name.
pub const DEFAULT_SUBACCOUNT_NAME: &str = "default";

/// Product ID for USDC spot (used for collateral deposits).
pub const USDC_PRODUCT_ID: u32 = 0;

/// Default orderbook depth.
pub const DEFAULT_ORDERBOOK_DEPTH: u32 = 10;

/// Build a 32-byte subaccount hex string: 20-byte address + 12-byte name (right-padded with null bytes).
/// Returns lowercase hex string without 0x prefix, 64 chars total.
pub fn build_subaccount_hex(address: &str, name: &str) -> anyhow::Result<String> {
    let addr_clean = address.trim_start_matches("0x");
    if addr_clean.len() != 40 {
        anyhow::bail!("Invalid address length: expected 40 hex chars, got {}", addr_clean.len());
    }

    // Encode name as UTF-8 bytes, right-pad to exactly 12 bytes with null bytes
    let name_bytes = name.as_bytes();
    if name_bytes.len() > 12 {
        anyhow::bail!("Subaccount name too long: max 12 bytes, got {}", name_bytes.len());
    }
    let mut padded = [0u8; 12];
    padded[..name_bytes.len()].copy_from_slice(name_bytes);
    let name_hex = hex::encode(padded);

    Ok(format!("0x{}{}", addr_clean.to_lowercase(), name_hex))
}
