// Maple Finance configuration
// Chain: Ethereum mainnet (1)
// Docs: https://docs.maple.finance/technical-resources/protocol-overview

pub const CHAIN_ID: u64 = 1;
pub const RPC_URL: &str = "https://ethereum.publicnode.com";
#[allow(dead_code)]
pub const GRAPHQL_URL: &str = "https://api.maple.finance/v2/graphql";

// Syrup pools — ERC-4626 vaults (pool contract address)
pub const SYRUP_USDC_POOL: &str = "0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b";
pub const SYRUP_USDT_POOL: &str = "0x356B8d89c1e1239Cbbb9dE4815c39A1474d5BA7D";

// SyrupRouters — handles approve + deposit in authorized manner
pub const SYRUP_USDC_ROUTER: &str = "0x134cCaaA4F1e4552eC8aEcb9E4A2360dDcF8df76";
pub const SYRUP_USDT_ROUTER: &str = "0xF007476Bb27430795138C511F18F821e8D1e5Ee2";

// Underlying tokens
pub const USDC_ADDRESS: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const USDT_ADDRESS: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";

// USDC decimals = 6, USDT decimals = 6
pub const USDC_DECIMALS: u32 = 6;
pub const USDT_DECIMALS: u32 = 6;

// Pool symbol names
pub const POOL_SYRUP_USDC: &str = "syrupUSDC";
pub const POOL_SYRUP_USDT: &str = "syrupUSDT";

pub struct PoolConfig {
    pub name: &'static str,
    pub pool: &'static str,
    pub router: &'static str,
    pub token: &'static str,
    pub token_symbol: &'static str,
    pub decimals: u32,
}

pub fn pools() -> Vec<PoolConfig> {
    vec![
        PoolConfig {
            name: POOL_SYRUP_USDC,
            pool: SYRUP_USDC_POOL,
            router: SYRUP_USDC_ROUTER,
            token: USDC_ADDRESS,
            token_symbol: "USDC",
            decimals: USDC_DECIMALS,
        },
        PoolConfig {
            name: POOL_SYRUP_USDT,
            pool: SYRUP_USDT_POOL,
            router: SYRUP_USDT_ROUTER,
            token: USDT_ADDRESS,
            token_symbol: "USDT",
            decimals: USDT_DECIMALS,
        },
    ]
}

/// Resolve pool config by name (case-insensitive)
pub fn resolve_pool(name: &str) -> Option<PoolConfig> {
    match name.to_lowercase().as_str() {
        "syrupusdc" | "usdc" => Some(PoolConfig {
            name: POOL_SYRUP_USDC,
            pool: SYRUP_USDC_POOL,
            router: SYRUP_USDC_ROUTER,
            token: USDC_ADDRESS,
            token_symbol: "USDC",
            decimals: USDC_DECIMALS,
        }),
        "syrupusdt" | "usdt" => Some(PoolConfig {
            name: POOL_SYRUP_USDT,
            pool: SYRUP_USDT_POOL,
            router: SYRUP_USDT_ROUTER,
            token: USDT_ADDRESS,
            token_symbol: "USDT",
            decimals: USDT_DECIMALS,
        }),
        _ => None,
    }
}
