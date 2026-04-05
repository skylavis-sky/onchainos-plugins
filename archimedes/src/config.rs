/// Archimedes Finance — Ethereum mainnet contract addresses.
///
/// All addresses verified against:
///   - scripts/ProjectInit/MainnetDeployment/DeployedStore.ts in thisisarchimedes/Archimedes_Finance
///   - PRD design.md contract table

// ── Core protocol contracts ──────────────────────────────────────────────────

/// LeverageEngine proxy (entry point for open/close positions)
pub const LEVERAGE_ENGINE: &str = "0x03dc7Fa99B986B7E6bFA195f39085425d8172E29";

/// Zapper proxy (converts stablecoins → OUSD, calls LeverageEngine)
pub const ZAPPER: &str = "0x624f570C24d61Ba5BF8FBFF17AA39BFc0a7b05d8";

/// Coordinator (tracks available lvUSD leverage)
pub const COORDINATOR: &str = "0x58c968fADa478adb995b59Ba9e46e3Db4d6B579d";

/// CDPosition (per-position accounting: OUSD principal, interest, lvUSD debt)
pub const CD_POSITION: &str = "0x229a9733063eAD8A1f769fd920eb60133fCCa3Ef";

/// ParameterStore (protocol parameters: max cycles, ARCH ratio, fees)
pub const PARAMETER_STORE: &str = "0xcc6Ea29928A1F6bc4796464F41b29b6d2E0ee42C";

/// PositionToken ERC-721 (NFT representing each leveraged position)
/// Resolved from DeployedStore.ts: positionTokenAddress
pub const POSITION_TOKEN: &str = "0x14c6A3C8DBa317B87ab71E90E264D0eA7877139D";

// ── Protocol tokens ──────────────────────────────────────────────────────────

/// ARCH governance / fee token
pub const ARCH_TOKEN: &str = "0x73C69d24ad28e2d43D03CBf35F79fE26EBDE1011";

/// lvUSD — Archimedes synthetic stablecoin used for leverage
#[allow(dead_code)]
pub const LV_USD_TOKEN: &str = "0x94A18d9FE00bab617fAD8B49b11e9F1f64Db6b36";

/// OUSD — Origin Dollar, the collateral token
#[allow(dead_code)]
pub const OUSD_TOKEN: &str = "0x2A8e1E676Ec238d8A992307B495b45B3fEAa5e86";

// ── Supported input stablecoins ───────────────────────────────────────────────

pub const USDC: &str = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
pub const USDT: &str = "0xdAC17F958D2ee523a2206206994597C13D831ec7";
pub const DAI: &str = "0x6B175474E89094C44Da98b954EedeAC495271d0F";

// ── Chain config ──────────────────────────────────────────────────────────────

pub const CHAIN_ID: u64 = 1;
pub const RPC_URL: &str = "https://ethereum.publicnode.com";

// ── Helper: map symbol to (address, decimals) ─────────────────────────────────

/// Returns (contract_address, decimals) for a supported stablecoin symbol or address.
pub fn resolve_stablecoin(token: &str) -> anyhow::Result<(&'static str, u8)> {
    match token.to_uppercase().as_str() {
        "USDC" => Ok((USDC, 6)),
        "USDT" => Ok((USDT, 6)),
        "DAI" => Ok((DAI, 18)),
        _ => {
            // Accept raw address — attempt to match
            let lower = token.to_lowercase();
            if lower == USDC.to_lowercase() {
                return Ok((USDC, 6));
            }
            if lower == USDT.to_lowercase() {
                return Ok((USDT, 6));
            }
            if lower == DAI.to_lowercase() {
                return Ok((DAI, 18));
            }
            anyhow::bail!(
                "Unsupported stablecoin '{}'. Supported: USDC, USDT, DAI",
                token
            )
        }
    }
}
