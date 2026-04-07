// src/config.rs — Loopscale constants and known addresses

pub const _CHAIN_SOLANA: u64 = 501;

// Known vault addresses (mainnet, confirmed April 2026)
pub const VAULT_USDC_PRIMARY: &str = "AXanCP4dJHtWd7zY4X7nwxN5t5Gysfy2uG3XTxSmXdaB";
pub const _VAULT_USDC_SECONDARY: &str = "7PeYxZpM2dpc4RRDQovexMJ6tkSVLWtRN4mbNywsU3e6";
pub const VAULT_SOL_PRIMARY: &str = "U1h9yhtpZgZsgVzMZe1iSpa6DSTBkSH89Egt59MXRYe";

// Token mints
pub const MINT_USDC: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const MINT_WSOL: &str = "So11111111111111111111111111111111111111112";

// Amount conversion
pub const USDC_DECIMALS: u64 = 1_000_000;       // 6 decimals
pub const SOL_DECIMALS: u64 = 1_000_000_000;    // 9 decimals

/// Convert UI amount to lamports based on token symbol or mint.
pub fn to_lamports(amount: f64, token: &str) -> u64 {
    let token_upper = token.to_uppercase();
    if token_upper == "SOL" || token == MINT_WSOL {
        (amount * SOL_DECIMALS as f64) as u64
    } else {
        // Default: USDC (6 decimals)
        (amount * USDC_DECIMALS as f64) as u64
    }
}

/// Convert lamports to UI amount based on token symbol.
pub fn from_lamports(lamports: u64, token: &str) -> f64 {
    let token_upper = token.to_uppercase();
    if token_upper == "SOL" || token == MINT_WSOL {
        lamports as f64 / SOL_DECIMALS as f64
    } else {
        lamports as f64 / USDC_DECIMALS as f64
    }
}

/// Convert cBPS rate to percentage (display).
/// e.g. 100_000 cBPS = 10% APY
pub fn cbps_to_pct(cbps: u64) -> f64 {
    cbps as f64 / 1_000_000.0 * 100.0
}

/// Resolve token mint from symbol.
pub fn token_to_mint(token: &str) -> &'static str {
    match token.to_uppercase().as_str() {
        "SOL" => MINT_WSOL,
        "USDC" => MINT_USDC,
        _ => MINT_USDC, // fallback
    }
}

/// Resolve default vault by token symbol.
pub fn default_vault_for_token(token: &str) -> &'static str {
    match token.to_uppercase().as_str() {
        "SOL" => VAULT_SOL_PRIMARY,
        _ => VAULT_USDC_PRIMARY,
    }
}
