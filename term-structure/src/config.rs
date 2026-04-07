#![allow(dead_code)]
/// Per-chain configuration for TermMax V2 (Term Structure).
///
/// Addresses sourced from term-structure/termmax-contract-v2 deployment files
/// and §5 of the design.md.
#[derive(Debug, Clone)]
pub struct ChainConfig {
    pub chain_id: u64,
    pub router_v1: &'static str,
    pub factory_v2: &'static str,
    pub termmax_viewer: &'static str,
    pub rpc_url: &'static str,
    pub name: &'static str,
}

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 42161,
        router_v1: "0x7fa333b184868d88aC78a82eC06d5e87d4B0322E",
        factory_v2: "0x18b8A9433dBefcd15370F10a75e28149bcc2e301",
        termmax_viewer: "0x012BFcbAC9EdEa04DFf07Cc61269E321f4595DfF",
        rpc_url: "https://arbitrum-one-rpc.publicnode.com",
        name: "Arbitrum One",
    },
    ChainConfig {
        chain_id: 1,
        router_v1: "0xC47591F5c023e44931c78D5A993834875b79FB11",
        factory_v2: "0xC53aB74EeB5E818147eb6d06134d81D3AC810987",
        termmax_viewer: "0xf574c1d7C18E250c341bdFb478cafefcaCbAbF09",
        rpc_url: "https://ethereum.publicnode.com",
        name: "Ethereum Mainnet",
    },
    ChainConfig {
        chain_id: 56,
        router_v1: "0xb7634dB4f4710bb992118bC37d1F63e00e2704A4",
        factory_v2: "0xdffE6De6de1dB8e1B5Ce77D3222eba401C2573b5",
        termmax_viewer: "0x80906014B577AFd760528FA8B32304A49806580C",
        rpc_url: "https://bsc-rpc.publicnode.com",
        name: "BNB Chain",
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

/// Known market addresses on Arbitrum (42161) — sourced from TermMax V2 deployment files.
/// Each market is a unique (collateral x underlying x maturity) deployment.
/// Markets with maturity < current Unix timestamp are expired.
///
/// Verified on-chain via tokens() and config() calls. Source:
///   term-structure/termmax-contract-v2/deployments/arb-mainnet/
///   arb-mainnet-v2-markets-*.env (block 385922276+)
///
/// Token addresses (Arbitrum One):
///   USDC   = 0xaf88d065e77c8cC2239327C5EDb3A432268e5831
///   WETH   = 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1
///   wstETH = 0x5979D7b546E38E414F7E9822514be443A4800529
///   weETH  = 0x35751007a407ca6FEffe80b3cB397736d2cf4dbe
#[derive(Debug, Clone)]
pub struct KnownMarket {
    pub address: &'static str,
    pub collateral_symbol: &'static str,
    pub underlying_symbol: &'static str,
    pub chain_id: u64,
    /// Human-readable maturity date
    pub maturity_label: &'static str,
    /// Unix timestamp of maturity (verified on-chain via config())
    pub maturity_ts: u64,
}

/// Arbitrum Mainnet known markets — verified from on-chain tokens() + config() calls.
/// All markets below have maturity = 1766714400 (2025-12-26).
/// Deployment batch 1 (block 385922276, ts 1759566742):
///   MARKET_ADDRESS_4..7
/// Deployment batch 2 (block approx 1760338083):
///   MARKET_ADDRESS_4..7 (second set with same maturity)
pub static KNOWN_MARKETS: &[KnownMarket] = &[
    // Batch 1: WETH collateral / USDC underlying (Dec 2025)
    KnownMarket {
        address: "0xB92A627a4E0a3968cB082968C88562018B248913",
        collateral_symbol: "WETH",
        underlying_symbol: "USDC",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 1: wstETH collateral / USDC underlying (Dec 2025)
    KnownMarket {
        address: "0x676978e9fA294409C6D90FD82C3a3aF4D9D140b9",
        collateral_symbol: "wstETH",
        underlying_symbol: "USDC",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 1: weETH collateral / WETH underlying (Dec 2025)
    KnownMarket {
        address: "0x0b5CDdBe339BE6A5843d8ABF3eD487360806E314",
        collateral_symbol: "weETH",
        underlying_symbol: "WETH",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 1: wstETH collateral / WETH underlying (Dec 2025)
    KnownMarket {
        address: "0xcF1Bb7e01385bf8bB6d59EcD2E0D44c915c6b30f",
        collateral_symbol: "wstETH",
        underlying_symbol: "WETH",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 2: WETH collateral / USDC underlying (Dec 2025)
    KnownMarket {
        address: "0x90B33de084741a557eD41F18466d192603E10315",
        collateral_symbol: "WETH",
        underlying_symbol: "USDC",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 2: wstETH collateral / USDC underlying (Dec 2025)
    KnownMarket {
        address: "0xFF8893147f50435077e17E8E86c0802608Bb22c9",
        collateral_symbol: "wstETH",
        underlying_symbol: "USDC",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 2: weETH collateral / WETH underlying (Dec 2025)
    KnownMarket {
        address: "0xE1406c76F486ED076edC864D45aCF0A1F4319F8f",
        collateral_symbol: "weETH",
        underlying_symbol: "WETH",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
    // Batch 2: wstETH collateral / WETH underlying (Dec 2025)
    KnownMarket {
        address: "0x0aC485114F6A135Dc10a731b859181c5F324fe55",
        collateral_symbol: "wstETH",
        underlying_symbol: "WETH",
        chain_id: 42161,
        maturity_label: "2025-12-26",
        maturity_ts: 1766714400,
    },
];

pub fn get_known_markets(chain_id: u64) -> Vec<&'static KnownMarket> {
    KNOWN_MARKETS
        .iter()
        .filter(|m| m.chain_id == chain_id)
        .collect()
}

/// Well-known token addresses on Arbitrum for decimal inference
pub fn token_decimals_by_symbol(symbol: &str) -> u8 {
    match symbol.to_uppercase().as_str() {
        "USDC" | "USDT" | "USDC.E" => 6,
        "WBTC" | "CBBTC" => 8,
        "WETH" | "ETH" | "WSTETH" | "ARB" | "WEETH" | "RETH" => 18,
        _ => 18,
    }
}

/// Resolve a well-known token address on Arbitrum by symbol
pub fn token_address_arbitrum(symbol: &str) -> Option<&'static str> {
    match symbol.to_uppercase().as_str() {
        "USDC" => Some("0xaf88d065e77c8cC2239327C5EDb3A432268e5831"),
        "USDT" => Some("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9"),
        "WETH" | "ETH" => Some("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1"),
        "WBTC" => Some("0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f"),
        "WSTETH" => Some("0x5979D7b546E38E414F7E9822514be443A4800529"),
        "ARB" => Some("0x912CE59144191C1204E64559FE8253a0e49E6548"),
        "WEETH" => Some("0x35751007a407ca6FEffe80b3cB397736d2cf4dbe"),
        _ => None,
    }
}

/// Slippage in basis points (0.5%)
pub const DEFAULT_SLIPPAGE_BPS: u64 = 50;
