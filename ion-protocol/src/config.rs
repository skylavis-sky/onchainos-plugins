/// Pool configuration for Ion Protocol.
///
/// Ion Protocol deploys one IonPool per collateral/lend pair.
/// All pools are on Ethereum Mainnet (chain 1).
///
/// Addresses verified from:
///   - Ion Protocol docs and on-chain deployment
///   - design.md §2 Pool Registry
#[derive(Debug, Clone)]
pub struct PoolConfig {
    pub name: &'static str,
    pub ion_pool: &'static str,
    pub gem_join: &'static str,
    pub collateral: &'static str,
    pub collateral_symbol: &'static str,
    pub lend_token: &'static str,
    pub lend_symbol: &'static str,
    /// ilkIndex within this specific IonPool (always 0 for current deployments)
    pub ilk_index: u8,
}

pub static POOLS: &[PoolConfig] = &[
    PoolConfig {
        name: "rsETH/wstETH",
        ion_pool: "0x0000000000E33e35EE6052fae87bfcFac61b1da9",
        gem_join: "0x3bC3AC09d1ee05393F2848d82cb420f347954432",
        collateral: "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7",
        collateral_symbol: "rsETH",
        lend_token: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0",
        lend_symbol: "wstETH",
        ilk_index: 0,
    },
    PoolConfig {
        name: "rswETH/wstETH",
        ion_pool: "0x00000000007C8105548f9d0eE081987378a6bE93",
        gem_join: "0xD696f9EA3299113324B9065ab19b70758256cf16",
        collateral: "0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0",
        collateral_symbol: "rswETH",
        lend_token: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0",
        lend_symbol: "wstETH",
        ilk_index: 0,
    },
    PoolConfig {
        name: "ezETH/WETH",
        ion_pool: "0x00000000008a3A77bd91bC738Ed2Efaa262c3763",
        gem_join: "0xe3692b2E55Eb2494cA73610c3b027F53815CCD39",
        collateral: "0xbf5495Efe5DB9ce00f80364C8B423567e58d2110",
        collateral_symbol: "ezETH",
        lend_token: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        lend_symbol: "WETH",
        ilk_index: 0,
    },
    PoolConfig {
        name: "weETH/wstETH",
        ion_pool: "0x0000000000eaEbd95dAfcA37A39fd09745739b78",
        gem_join: "0x3f6119b0328c27190be39597213ea1729f061876",
        collateral: "0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee",
        collateral_symbol: "weETH",
        lend_token: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0",
        lend_symbol: "wstETH",
        ilk_index: 0,
    },
];

/// Only chain 1 (Ethereum Mainnet) is supported.
pub const CHAIN_ID: u64 = 1;
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

/// RAY precision constant (1e27)
pub const RAY: u128 = 1_000_000_000_000_000_000_000_000_000;

/// WAD precision constant (1e18)
pub const WAD: u128 = 1_000_000_000_000_000_000;

#[allow(dead_code)]
pub fn get_pool_by_collateral(symbol: &str) -> anyhow::Result<&'static PoolConfig> {
    POOLS
        .iter()
        .find(|p| p.collateral_symbol.eq_ignore_ascii_case(symbol) || p.name.eq_ignore_ascii_case(symbol))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown pool/collateral '{}'. Valid options: {}",
                symbol,
                POOLS.iter().map(|p| p.collateral_symbol).collect::<Vec<_>>().join(", ")
            )
        })
}

pub fn get_pool_by_name(name: &str) -> anyhow::Result<&'static PoolConfig> {
    POOLS
        .iter()
        .find(|p| {
            p.name.eq_ignore_ascii_case(name)
                || p.collateral_symbol.eq_ignore_ascii_case(name)
                || p.ion_pool.eq_ignore_ascii_case(name)
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown pool '{}'. Valid pools: {}",
                name,
                POOLS.iter().map(|p| p.name).collect::<Vec<_>>().join(", ")
            )
        })
}
