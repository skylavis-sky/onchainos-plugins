/// deBridge DLN API base URL
pub const DEBRIDGE_API_BASE: &str = "https://dln.debridge.finance/v1.0";

// ---------------------------------------------------------------------------
// deBridge internal chain IDs (used in API calls)
// ---------------------------------------------------------------------------
pub const DEBRIDGE_CHAIN_ID_ETH: &str = "1";
pub const DEBRIDGE_CHAIN_ID_ARBITRUM: &str = "42161";
pub const DEBRIDGE_CHAIN_ID_BASE: &str = "8453";
pub const DEBRIDGE_CHAIN_ID_OPTIMISM: &str = "10";
pub const DEBRIDGE_CHAIN_ID_BSC: &str = "56";
pub const DEBRIDGE_CHAIN_ID_POLYGON: &str = "137";
pub const DEBRIDGE_CHAIN_ID_AVALANCHE: &str = "43114";
/// Solana's deBridge internal chain ID (NOT standard chain ID 501)
pub const DEBRIDGE_CHAIN_ID_SOLANA: &str = "7565164";

// ---------------------------------------------------------------------------
// Contract addresses
// ---------------------------------------------------------------------------
/// DlnSource EVM — same address on all supported EVM chains
pub const DLN_SOURCE_EVM: &str = "0xeF4fB24aD0916217251F553c0596F8Edc630EB66";
/// DlnDestination EVM — same address on all supported EVM chains
pub const DLN_DESTINATION_EVM: &str = "0xe7351fd770a37282b91d153ee690b63579d6dd7f";
/// Solana DlnSource program ID
pub const DLN_SOURCE_SOLANA: &str = "src5qyZHqTqecJV4aY6Cb6zDZLMDzrDKKezs22MPHr4";
/// Solana DlnDestination program ID
pub const DLN_DESTINATION_SOLANA: &str = "dst5MGcFPoBeREFAA5E3tU5ij8m5uVYwkzkSAbsLbNo";

// ---------------------------------------------------------------------------
// Native token identifiers
// ---------------------------------------------------------------------------
/// Native ETH/EVM zero address
pub const NATIVE_EVM: &str = "0x0000000000000000000000000000000000000000";
/// Native SOL system program
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";

// ---------------------------------------------------------------------------
// Well-known token addresses
// ---------------------------------------------------------------------------
pub const USDC_ARBITRUM: &str = "0xaf88d065e77c8cc2239327c5edb3a432268e5831";
pub const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
pub const USDC_ETHEREUM: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
pub const USDC_SOLANA: &str = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const WETH_ARBITRUM: &str = "0x82af49447d8a07e3bd95bd0d56f35241523fbab1";
pub const WETH_BASE: &str = "0x4200000000000000000000000000000000000006";

// ---------------------------------------------------------------------------
// Function selectors
// ---------------------------------------------------------------------------
/// ERC-20 approve(address,uint256) selector
pub const APPROVE_SELECTOR: &str = "0x095ea7b3";
/// ERC-20 allowance(address,address) selector
pub const ALLOWANCE_SELECTOR: &str = "0xdd62ed3e";

// ---------------------------------------------------------------------------
// Timing constants
// ---------------------------------------------------------------------------
/// Sleep between approve and createOrder to avoid nonce collision
pub const APPROVE_DELAY_SECS: u64 = 3;

// ---------------------------------------------------------------------------
// Chain ID conversion helpers
// ---------------------------------------------------------------------------

/// Convert onchainos chain ID to deBridge API chain ID string.
/// EVM IDs are identical; Solana maps 501 -> "7565164".
pub fn onchainos_to_debridge_chain(onchainos_id: u64) -> String {
    match onchainos_id {
        501 => DEBRIDGE_CHAIN_ID_SOLANA.to_string(),
        other => other.to_string(),
    }
}

/// Return true if the given onchainos chain ID is Solana.
pub fn is_solana(onchainos_id: u64) -> bool {
    onchainos_id == 501
}

/// Public RPC endpoints for EVM chains (for allowance checks).
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://ethereum.publicnode.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        8453 => "https://base-rpc.publicnode.com",
        10 => "https://mainnet.optimism.io",
        56 => "https://bsc-rpc.publicnode.com",
        137 => "https://polygon-rpc.com",
        43114 => "https://api.avax.network/ext/bc/C/rpc",
        _ => "https://ethereum.publicnode.com",
    }
}
