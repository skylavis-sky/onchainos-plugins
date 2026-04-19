/// Chain configuration and contract addresses for Uniswap V3.
/// Base has DIFFERENT addresses than all other chains — do not mix them.

pub struct ChainConfig {
    pub chain_id: u64,
    pub rpc_url: &'static str,
    pub factory: &'static str,
    pub swap_router02: &'static str,
    pub quoter_v2: &'static str,
    pub nfpm: &'static str, // NonfungiblePositionManager
    pub weth: &'static str,
}

pub const ETHEREUM: ChainConfig = ChainConfig {
    chain_id: 1,
    rpc_url: "https://ethereum.publicnode.com",
    factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
    swap_router02: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
    quoter_v2: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
    nfpm: "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
    weth: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
};

pub const ARBITRUM: ChainConfig = ChainConfig {
    chain_id: 42161,
    rpc_url: "https://arbitrum-one-rpc.publicnode.com",
    factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
    swap_router02: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
    quoter_v2: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
    nfpm: "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
    weth: "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
};

// Base has DIFFERENT contract addresses than all other chains
pub const BASE: ChainConfig = ChainConfig {
    chain_id: 8453,
    rpc_url: "https://base-rpc.publicnode.com",
    factory: "0x33128a8fC17869897dcE68Ed026d694621f6FDfD",
    swap_router02: "0x2626664c2603336E57B271c5C0b26F421741e481",
    quoter_v2: "0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a",
    nfpm: "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1",
    weth: "0x4200000000000000000000000000000000000006",
};

pub const OPTIMISM: ChainConfig = ChainConfig {
    chain_id: 10,
    rpc_url: "https://optimism.publicnode.com",
    factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
    swap_router02: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
    quoter_v2: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
    nfpm: "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
    weth: "0x4200000000000000000000000000000000000006",
};

pub const POLYGON: ChainConfig = ChainConfig {
    chain_id: 137,
    rpc_url: "https://polygon-bor-rpc.publicnode.com",
    factory: "0x1F98431c8aD98523631AE4a59f267346ea31F984",
    swap_router02: "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",
    quoter_v2: "0x61fFE014bA17989E743c5F6cB21bF9697530B21e",
    nfpm: "0xC36442b4a4522E871399CD717aBDD847Ab11FE88",
    weth: "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
};

pub fn get_chain_config(chain_id: u64) -> anyhow::Result<&'static ChainConfig> {
    match chain_id {
        1 => Ok(&ETHEREUM),
        42161 => Ok(&ARBITRUM),
        8453 => Ok(&BASE),
        10 => Ok(&OPTIMISM),
        137 => Ok(&POLYGON),
        _ => anyhow::bail!(
            "Unsupported chain ID: {}. Supported: 1 (Ethereum), 10 (Optimism), 137 (Polygon), 8453 (Base), 42161 (Arbitrum)",
            chain_id
        ),
    }
}

/// Uniswap V3 fee tiers and their tick spacing.
pub fn tick_spacing(fee: u32) -> anyhow::Result<i32> {
    match fee {
        100 => Ok(1),
        500 => Ok(10),
        3000 => Ok(60),
        10000 => Ok(200),
        _ => anyhow::bail!("Unknown fee tier: {}. Valid: 100, 500, 3000, 10000", fee),
    }
}

/// Default full-range tick bounds for each fee tier.
/// These are the largest tick values that are multiples of the tick spacing.
pub fn full_range_ticks(fee: u32) -> anyhow::Result<(i32, i32)> {
    match fee {
        100 => Ok((-887272, 887272)),
        500 => Ok((-887270, 887270)),
        3000 => Ok((-887220, 887220)),
        10000 => Ok((-887200, 887200)),
        _ => anyhow::bail!("Unknown fee tier: {} for tick range calculation", fee),
    }
}

/// Resolve a token symbol to its canonical address for the given chain.
/// If the input is already a 0x address, it is returned as-is.
pub fn resolve_token_address(symbol_or_addr: &str, chain_id: u64) -> anyhow::Result<String> {
    if symbol_or_addr.starts_with("0x") || symbol_or_addr.starts_with("0X") {
        return Ok(symbol_or_addr.to_string());
    }
    let sym = symbol_or_addr.to_uppercase();
    let addr = match (chain_id, sym.as_str()) {
        // Ethereum (1)
        (1, "WETH") | (1, "ETH") => "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        (1, "USDC") => "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        (1, "USDT") => "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        (1, "DAI") => "0x6B175474E89094C44Da98b954EedeAC495271d0F",
        (1, "WBTC") => "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599",
        (1, "UNI") => "0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984",
        // Arbitrum (42161)
        (42161, "WETH") | (42161, "ETH") => "0x82aF49447D8a07e3bd95BD0d56f35241523fBab1",
        (42161, "USDC") => "0xaf88d065e77c8cC2239327C5EDb3A432268e5831",
        (42161, "USDC.E") | (42161, "USDC_E") => "0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8",
        (42161, "USDT") => "0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9",
        (42161, "ARB") => "0x912CE59144191C1204E64559FE8253a0e49E6548",
        // Base (8453)
        (8453, "WETH") | (8453, "ETH") => "0x4200000000000000000000000000000000000006",
        (8453, "USDC") => "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        (8453, "CBETH") => "0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22",
        // Optimism (10)
        (10, "WETH") | (10, "ETH") => "0x4200000000000000000000000000000000000006",
        (10, "USDC") => "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
        (10, "OP") => "0x4200000000000000000000000000000000000042",
        // Polygon (137)
        (137, "WMATIC") | (137, "MATIC") => "0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270",
        (137, "USDC") => "0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174",
        (137, "USDC.E") | (137, "USDC_E") => "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359",
        (137, "WETH") | (137, "ETH") => "0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619",
        _ => anyhow::bail!(
            "Unknown token symbol '{}' on chain {}. Please use a full 0x address.",
            symbol_or_addr, chain_id
        ),
    };
    Ok(addr.to_string())
}

/// Convert human-readable token amount to minimal units (wei/atomic).
pub fn human_to_minimal(amount: &str, decimals: u8) -> anyhow::Result<u128> {
    let f: f64 = amount
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount: {}", amount))?;
    if f < 0.0 {
        anyhow::bail!("Amount must be non-negative");
    }
    Ok((f * 10f64.powi(decimals as i32)) as u128)
}

/// Explorer URL for a transaction hash on the given chain.
pub fn explorer_url(chain_id: u64, tx_hash: &str) -> String {
    let base = match chain_id {
        1 => "https://etherscan.io/tx/",
        42161 => "https://arbiscan.io/tx/",
        8453 => "https://basescan.org/tx/",
        10 => "https://optimistic.etherscan.io/tx/",
        137 => "https://polygonscan.com/tx/",
        _ => "https://etherscan.io/tx/",
    };
    format!("{}{}", base, tx_hash)
}
