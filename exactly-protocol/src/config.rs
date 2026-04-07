/// Per-chain configuration for Exactly Protocol.
///
/// Addresses sourced from design.md (verified against Exactly Finance GitHub and Etherscan).
/// MarketWETH and MarketETHRouter share the same address on Optimism and Ethereum mainnet
/// due to deterministic CREATE2 deployment — always pair addresses with correct chain_id.

#[derive(Debug, Clone)]
pub struct MarketConfig {
    pub symbol: &'static str,
    pub market_address: &'static str,
    pub asset_address: &'static str,
    pub decimals: u8,
}

#[derive(Debug, Clone)]
pub struct ChainConfig {
    #[allow(dead_code)]
    pub chain_id: u64,
    pub name: &'static str,
    pub rpc_url: &'static str,
    pub previewer: &'static str,
    pub auditor: &'static str,
    #[allow(dead_code)]
    pub eth_router: &'static str,
    pub markets: &'static [MarketConfig],
}

/// Optimism (chain 10) markets
static OPTIMISM_MARKETS: &[MarketConfig] = &[
    MarketConfig {
        symbol: "WETH",
        market_address: "0xc4d4500326981eacD020e20A81b1c479c161c7EF",
        asset_address: "0x4200000000000000000000000000000000000006",
        decimals: 18,
    },
    MarketConfig {
        symbol: "USDC",
        market_address: "0x6926B434CCe9b5b7966aE1BfEef6D0A7DCF3A8bb",
        asset_address: "0x0b2c639c533813f4aa9d7837caf62653d097ff85",
        decimals: 6,
    },
    MarketConfig {
        symbol: "OP",
        market_address: "0xa430A427bd00210506589906a71B54d6C256CEdb",
        asset_address: "0x4200000000000000000000000000000000000042",
        decimals: 18,
    },
    MarketConfig {
        symbol: "wstETH",
        market_address: "0x22ab31Cd55130435b5efBf9224b6a9d5EC36533F",
        asset_address: "0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb",
        decimals: 18,
    },
    MarketConfig {
        symbol: "WBTC",
        market_address: "0x6f748FD65d7c71949BA6641B3248C4C191F3b322",
        asset_address: "0x68f180fcCe6836688e9084f035309E29Bf0A2095",
        decimals: 8,
    },
];

/// Ethereum mainnet (chain 1) markets
static ETHEREUM_MARKETS: &[MarketConfig] = &[
    MarketConfig {
        symbol: "WETH",
        market_address: "0xc4d4500326981eacD020e20A81b1c479c161c7EF",
        asset_address: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        decimals: 18,
    },
    MarketConfig {
        symbol: "USDC",
        market_address: "0x660e2fC185a9fFE722aF253329CEaAD4C9F6F928",
        asset_address: "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        decimals: 6,
    },
    MarketConfig {
        symbol: "wstETH",
        market_address: "0x3843c41DA1d7909C86faD51c47B9A97Cf62a29e1",
        asset_address: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0",
        decimals: 18,
    },
    MarketConfig {
        symbol: "WBTC",
        market_address: "0x8644c0FDED361D1920e068bA4B09996e26729435",
        asset_address: "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
        decimals: 8,
    },
];

static OPTIMISM_CONFIG: ChainConfig = ChainConfig {
    chain_id: 10,
    name: "Optimism",
    rpc_url: "https://mainnet.optimism.io",
    previewer: "0x328834775A18A4c942F30bfd091259ade4355C2a",
    auditor: "0xaEb62e6F27BC103702E7BC879AE98bceA56f027E",
    eth_router: "0x29bAbFF3eBA7B517a75109EA8fd6D1eAb4A10258",
    markets: OPTIMISM_MARKETS,
};

static ETHEREUM_CONFIG: ChainConfig = ChainConfig {
    chain_id: 1,
    name: "Ethereum Mainnet",
    rpc_url: "https://ethereum.publicnode.com",
    previewer: "0x5fE09baAa75fd107a8dF8565813f66b3603a13D3",
    auditor: "0x310A2694521f75C7B2b64b5937C16CE65C3EFE01",
    eth_router: "0x29bAbFF3eBA7B517a75109EA8fd6D1eAb4A10258",
    markets: ETHEREUM_MARKETS,
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        10 => Ok(&OPTIMISM_CONFIG),
        1 => Ok(&ETHEREUM_CONFIG),
        _ => anyhow::bail!(
            "Unsupported chain ID: {}. Supported chains: Optimism (10), Ethereum Mainnet (1)",
            chain_id
        ),
    }
}

/// Resolve a market by symbol or address on the given chain.
pub fn resolve_market(chain_id: u64, market_sym_or_addr: &str) -> anyhow::Result<&'static MarketConfig> {
    let cfg = get_chain_config(chain_id)?;
    // Try by symbol (case-insensitive)
    if let Some(m) = cfg.markets.iter().find(|m| {
        m.symbol.eq_ignore_ascii_case(market_sym_or_addr)
    }) {
        return Ok(m);
    }
    // Try by market address
    if let Some(m) = cfg.markets.iter().find(|m| {
        m.market_address.eq_ignore_ascii_case(market_sym_or_addr)
    }) {
        return Ok(m);
    }
    anyhow::bail!(
        "Unknown market '{}' on chain {}. Available: {}",
        market_sym_or_addr,
        chain_id,
        cfg.markets.iter().map(|m| m.symbol).collect::<Vec<_>>().join(", ")
    )
}

/// Convert human-readable amount to minimal units given decimals.
pub fn human_to_minimal(amount: f64, decimals: u8) -> u128 {
    let factor = 10u128.pow(decimals as u32);
    (amount * factor as f64) as u128
}

/// Convert minimal units back to human-readable f64 given decimals.
#[allow(dead_code)]
pub fn minimal_to_human(amount: u128, decimals: u8) -> f64 {
    let factor = 10u128.pow(decimals as u32) as f64;
    amount as f64 / factor
}

/// Apply slippage downward (for min amounts): amount * (10000 - bps) / 10000
pub fn apply_slippage_min(amount: u128, bps: u32) -> u128 {
    amount * (10000 - bps as u128) / 10000
}

/// Apply slippage upward (for max amounts): amount * (10000 + bps) / 10000
pub fn apply_slippage_max(amount: u128, bps: u32) -> u128 {
    amount * (10000 + bps as u128) / 10000
}

/// Default slippage: 100 bps = 1%
pub const SLIPPAGE_BPS: u32 = 100;
