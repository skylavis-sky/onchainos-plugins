/// Chain configuration and token address maps for the 1inch plugin.
///
/// 1inch Router V6 is deployed at the SAME address on all supported chains.
/// Token addresses are chain-specific.

pub const ROUTER_V6: &str = "0x111111125421cA6dc452d289314280a0f8842A65";

/// Sentinel address used by 1inch for the native chain token (ETH/BNB/MATIC).
pub const NATIVE_TOKEN: &str = "0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

pub struct ChainConfig {
    pub chain_id: u64,
    pub name: &'static str,
    pub explorer: &'static str,
}

pub const SUPPORTED_CHAINS: &[ChainConfig] = &[
    ChainConfig { chain_id: 1,     name: "Ethereum",  explorer: "https://etherscan.io/tx/" },
    ChainConfig { chain_id: 42161, name: "Arbitrum",  explorer: "https://arbiscan.io/tx/" },
    ChainConfig { chain_id: 8453,  name: "Base",      explorer: "https://basescan.org/tx/" },
    ChainConfig { chain_id: 56,    name: "BSC",       explorer: "https://bscscan.com/tx/" },
    ChainConfig { chain_id: 137,   name: "Polygon",   explorer: "https://polygonscan.com/tx/" },
];

pub fn get_chain_name(chain_id: u64) -> &'static str {
    for c in SUPPORTED_CHAINS {
        if c.chain_id == chain_id {
            return c.name;
        }
    }
    "Unknown"
}

pub fn explorer_url(chain_id: u64, tx_hash: &str) -> String {
    for c in SUPPORTED_CHAINS {
        if c.chain_id == chain_id {
            return format!("{}{}", c.explorer, tx_hash);
        }
    }
    format!("https://etherscan.io/tx/{}", tx_hash)
}

pub fn validate_chain(chain_id: u64) -> anyhow::Result<()> {
    for c in SUPPORTED_CHAINS {
        if c.chain_id == chain_id {
            return Ok(());
        }
    }
    anyhow::bail!(
        "Unsupported chain ID: {}. Supported: 1 (Ethereum), 42161 (Arbitrum), 8453 (Base), 56 (BSC), 137 (Polygon)",
        chain_id
    )
}

/// Resolve a token symbol to its address for the given chain.
/// If the input already starts with "0x" it is returned as-is.
pub fn resolve_token(symbol_or_addr: &str, chain_id: u64) -> anyhow::Result<(String, u8)> {
    // Already an address
    if symbol_or_addr.starts_with("0x") || symbol_or_addr.starts_with("0X") {
        // Decimals unknown for raw addresses — caller must handle
        return Ok((symbol_or_addr.to_string(), 18));
    }

    let sym = symbol_or_addr.to_uppercase();

    // Native token sentinel (same across all chains)
    if matches!(sym.as_str(), "ETH" | "BNB" | "MATIC" | "POL") {
        return Ok((NATIVE_TOKEN.to_string(), 18));
    }

    let result: Option<(&str, u8)> = match (chain_id, sym.as_str()) {
        // ── Ethereum (1) ──
        (1, "WETH")  => Some(("0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2", 18)),
        (1, "USDC")  => Some(("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48", 6)),
        (1, "USDT")  => Some(("0xdAC17F958D2ee523a2206206994597C13D831ec7", 6)),
        (1, "DAI")   => Some(("0x6B175474E89094C44Da98b954EedeAC495271d0F", 18)),
        (1, "WBTC")  => Some(("0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599", 8)),
        (1, "1INCH") => Some(("0x111111111117dC0aa78b770fA6A738034120C302", 18)),

        // ── Arbitrum (42161) ──
        (42161, "WETH")            => Some(("0x82aF49447D8a07e3bd95BD0d56f35241523fBab1", 18)),
        (42161, "USDC")            => Some(("0xaf88d065e77c8cC2239327C5EDb3A432268e5831", 6)),
        (42161, "USDC.E" | "USDC_E") => Some(("0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8", 6)),
        (42161, "USDT")            => Some(("0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9", 6)),
        (42161, "ARB")             => Some(("0x912CE59144191C1204E64559FE8253a0e49E6548", 18)),

        // ── Base (8453) ──
        (8453, "WETH")  => Some(("0x4200000000000000000000000000000000000006", 18)),
        (8453, "USDC")  => Some(("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", 6)),
        (8453, "CBETH") => Some(("0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22", 18)),

        // ── BSC (56) ──
        (56, "WBNB") => Some(("0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c", 18)),
        (56, "USDC") => Some(("0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d", 18)),
        (56, "USDT") => Some(("0x55d398326f99059fF775485246999027B3197955", 18)),
        (56, "BUSD") => Some(("0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56", 18)),

        // ── Polygon (137) ──
        (137, "WMATIC")               => Some(("0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270", 18)),
        (137, "USDC")                 => Some(("0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174", 6)),
        (137, "USDC.E" | "USDC_E")   => Some(("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359", 6)),
        (137, "USDT")                 => Some(("0xc2132D05D31c914a87C6611C10748AEb04B58e8F", 6)),
        (137, "WETH")                 => Some(("0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619", 18)),

        _ => None,
    };

    match result {
        Some((addr, dec)) => Ok((addr.to_string(), dec)),
        None => anyhow::bail!(
            "Unknown token '{}' on chain {}. Use a full 0x address or check supported tokens.",
            symbol_or_addr, chain_id
        ),
    }
}

/// Convert a human-readable amount (e.g. "1.5") to the smallest token unit as a string.
pub fn to_minimal_units(amount: &str, decimals: u8) -> anyhow::Result<String> {
    let f: f64 = amount
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount: '{}'", amount))?;
    if f < 0.0 {
        anyhow::bail!("Amount must be non-negative");
    }
    // Use u128 to avoid overflow for large amounts
    let raw = (f * 10f64.powi(decimals as i32)).round() as u128;
    Ok(raw.to_string())
}

/// Convert smallest-unit amount string back to human-readable, given decimals.
pub fn from_minimal_units(raw: &str, decimals: u8) -> String {
    let v: u128 = raw.parse().unwrap_or(0);
    let divisor = 10u128.pow(decimals as u32);
    let whole = v / divisor;
    let frac = v % divisor;
    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:0>width$}", frac, width = decimals as usize);
        let frac_trim = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, frac_trim)
    }
}

/// Return true if the token address is the native sentinel (ETH/BNB/MATIC).
pub fn is_native_token(addr: &str) -> bool {
    addr.eq_ignore_ascii_case(NATIVE_TOKEN)
}
