/// Per-chain configuration for ZeroLend (Aave V3 fork).
///
/// POOL_ADDRESSES_PROVIDER addresses are the immutable registry entry points —
/// safe to store in config. Pool address itself must ALWAYS be resolved at
/// runtime via PoolAddressesProvider.getPool().
///
/// Addresses sourced from docs.zerolend.xyz/security/deployed-addresses (April 2026).
///
/// Note: ZeroLend is an Aave V3 fork with no ABI changes — all function selectors
/// and calldata encoding are identical to Aave V3.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub pool_addresses_provider: &'static str,
    pub rpc_url: &'static str,
    pub name: &'static str,
}

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 324,
        pool_addresses_provider: "0x4f285Ea117eF0067B59853D6d16a5dE8088bA259",
        rpc_url: "https://mainnet.era.zksync.io",
        name: "zkSync Era",
    },
    ChainConfig {
        chain_id: 59144,
        pool_addresses_provider: "0xC44827C51d00381ed4C52646aeAB45b455d200eB",
        rpc_url: "https://rpc.linea.build",
        name: "Linea",
    },
    ChainConfig {
        chain_id: 81457,
        pool_addresses_provider: "0xb0811a1FC9Fb9972ee683Ba04c32Cb828Bcf587B",
        rpc_url: "https://rpc.blast.io",
        name: "Blast",
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

/// Interest rate mode constants
pub const INTEREST_RATE_MODE_VARIABLE: u128 = 2;
/// Stable rate (deprecated in V3.1+) — blocked in borrow command
#[allow(dead_code)]
pub const INTEREST_RATE_MODE_STABLE: u128 = 1;

/// Aave referral code (0 = no referral)
pub const REFERRAL_CODE: u16 = 0;

/// Health factor thresholds (scaled 1e18 on-chain, these are human-readable)
pub const HF_WARN_THRESHOLD: f64 = 1.1;
#[allow(dead_code)]
pub const HF_DANGER_THRESHOLD: f64 = 1.05;

