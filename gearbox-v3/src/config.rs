/// Per-chain configuration for Gearbox V3.
///
/// Addresses sourced from Gearbox stateArbitrum.json (block 239832594, Aug 2024).
/// DataCompressor and AddressProvider are stable — safe to hard-code.
/// CreditFacadeV3 / CreditManagerV3 addresses come from DataCompressor at runtime.

#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub data_compressor: &'static str,
    pub rpc_url: &'static str,
    pub name: &'static str,
}

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 42161,
        data_compressor: "0x88aa4FbF86392cBF6f6517790E288314DE03E181",
        rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        name: "Arbitrum One",
    },
    ChainConfig {
        chain_id: 1,
        data_compressor: "0x0000000000000000000000000000000000000000",
        rpc_url: "https://ethereum.publicnode.com",
        name: "Ethereum Mainnet",
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

/// Well-known Credit Managers on Arbitrum (Trade USDC Tier 2 is default for testing).
/// These are hard-coded as fallback; runtime enumeration via DataCompressor is preferred.
#[allow(dead_code)]
pub mod arbitrum {
    /// Trade USDC Tier 2 — minDebt 1000 USDC, maxDebt 20000 USDC (best for testing)
    pub const CREDIT_FACADE_USDC_TIER2: &str = "0x3974888520a637ce73bdcb2ee28a396f4b303876";
    pub const CREDIT_MANAGER_USDC_TIER2: &str = "0xb780dd9cec259a0bbf7b32587802f33730353e86";
    /// Native USDC on Arbitrum
    pub const USDC_ADDR: &str = "0xaf88d065e77c8cC2239327C5EDb3A432268e5831";
}

/// Referral code (0 = no referral)
pub const REFERRAL_CODE: u64 = 0;

/// Health factor warning threshold
#[allow(dead_code)]
pub const HF_WARN_THRESHOLD: f64 = 1.1;
