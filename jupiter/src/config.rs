/// Jupiter plugin configuration — token mint addresses and constants

pub const SOLANA_CHAIN_ID: &str = "501";

// Jupiter v6 Program ID
pub const JUPITER_PROGRAM_ID: &str = "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4";

// Common token mint addresses on Solana mainnet
pub const SOL_MINT: &str = "So11111111111111111111111111111111111111112";
pub const USDC_MINT: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const USDT_MINT: &str = "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB";
pub const JUP_MINT: &str = "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN";

// Jupiter API endpoints
pub const SWAP_API_BASE: &str = "https://api.jup.ag/swap/v2";
pub const PRICE_API_BASE: &str = "https://api.jup.ag/price/v3";
pub const TOKENS_SEARCH_API: &str = "https://api.jup.ag/tokens/v2/search";

// Defaults
pub const DEFAULT_SLIPPAGE_BPS: u32 = 50;
pub const DEFAULT_TOKENS_LIMIT: usize = 20;

/// Resolve a token symbol to its mint address.
/// Returns the input unchanged if it is not a known symbol (treated as raw mint).
pub fn resolve_mint(symbol_or_mint: &str) -> &str {
    match symbol_or_mint.to_uppercase().as_str() {
        "SOL" => SOL_MINT,
        "USDC" => USDC_MINT,
        "USDT" => USDT_MINT,
        "JUP" => JUP_MINT,
        _ => symbol_or_mint,
    }
}

/// Return token decimals for known mints; default 9 (SOL decimals).
pub fn token_decimals(mint: &str) -> u32 {
    match mint {
        USDC_MINT | USDT_MINT => 6,
        _ => 9, // SOL and most SPL tokens use 9 decimals
    }
}

/// Convert a UI float amount to raw atomic units using the token's decimal count.
pub fn to_raw_amount(amount: f64, mint: &str) -> u64 {
    let decimals = token_decimals(mint);
    let factor = 10_u64.pow(decimals);
    (amount * factor as f64).round() as u64
}

/// Convert raw atomic units back to UI amount string.
pub fn from_raw_amount(raw: u64, mint: &str) -> f64 {
    let decimals = token_decimals(mint);
    let factor = 10_u64.pow(decimals) as f64;
    raw as f64 / factor
}
