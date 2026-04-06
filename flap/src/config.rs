/// BSC chain ID used by onchainos
pub const BSC_CHAIN_ID: &str = "56";

/// Flap Portal proxy contract address on BSC mainnet
pub const PORTAL_ADDRESS: &str = "0xe2cE6ab80874Fa9Fa2aAE65D277Dd6B8e65C9De0";

/// Flap Portal implementation address (used for CREATE2 initcode hash)
pub const PORTAL_IMPL_ADDRESS: &str = "0xe6b2abbf364eccbee54c3a9debeb28826d5b7533";

/// Standard token implementation address (TOKEN_V2_PERMIT)
pub const STANDARD_TOKEN_IMPL: &str = "0x8b4329947e34b6d56d71a3385cac122bade7d78d";

/// Tax token V3 implementation address (TOKEN_TAXED_V3)
pub const TAX_TOKEN_V3_IMPL: &str = "0x024f18294970B5c76c0691b87f138A0317156422";

/// TaxTokenHelper contract address on BSC mainnet
pub const TAX_TOKEN_HELPER_ADDRESS: &str = "0x53841c73217735F37BC1775538b03b23feFD8346";

/// Default BSC RPC endpoint
pub const DEFAULT_RPC_URL: &str = "https://bsc-rpc.publicnode.com";

/// Flap metadata upload endpoint
pub const METADATA_UPLOAD_URL: &str = "https://funcs.flap.sh/api/upload";

/// Default slippage in basis points (500 = 5%)
pub const DEFAULT_SLIPPAGE_BPS: u64 = 500;

/// Sell tax warning threshold in basis points (500 = 5%)
pub const SELL_TAX_WARNING_THRESHOLD_BPS: u16 = 500;

/// Graduation threshold in token units: 80% of 1B total supply = 800M tokens (in token wei units: 800M * 1e18)
/// When circulatingSupply reaches this, the bonding curve graduates to DEX.
pub const GRADUATION_SUPPLY_THRESHOLD: u128 = 800_000_000_000_000_000_000_000_000; // 800M * 1e18

// Function selectors
pub const SELECTOR_NEW_TOKEN_V6: [u8; 4] = [0x8c, 0xb5, 0x77, 0x2c];
pub const SELECTOR_SWAP_EXACT_INPUT: [u8; 4] = [0xef, 0x7e, 0xc2, 0xe7];
pub const SELECTOR_APPROVE: [u8; 4] = [0x09, 0x5e, 0xa7, 0xb3];
pub const SELECTOR_GET_TOKEN_V8_SAFE: [u8; 4] = [0x62, 0xfa, 0xfc, 0xca];
pub const SELECTOR_QUOTE_EXACT_INPUT: [u8; 4] = [0xfc, 0x84, 0x7c, 0x2b];
