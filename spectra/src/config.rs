/// Spectra Registry addresses per chain
pub fn registry_address(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "0x786da12e9836a9ff9b7d92e8bac1c849e2ace378",
        42161 => "0x22Ce9e1C5D2D1b32FE70E1B40E64671B7f6E9B35", // Arbitrum registry (resolve on-chain if needed)
        1 => "0xecc1b6D59FA89F6AF4c64b83e3d609B0dD374da6", // Ethereum mainnet registry
        _ => "0x786da12e9836a9ff9b7d92e8bac1c849e2ace378", // fallback to Base
    }
}

/// Spectra Router addresses per chain
pub fn router_address(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "0xc03309de321a4d3df734f5609b80cc731ae28e6d",
        42161 => "0xB1025B15B3aE2c9cB4DAeAe2F62fC1ECB4E5EE0C", // Arbitrum router
        1 => "0xd08f218ef5B2E96534c66C6bD5573b13c50d5a89", // Ethereum mainnet router
        _ => "0xc03309de321a4d3df734f5609b80cc731ae28e6d", // fallback to Base
    }
}

/// Return RPC URL for the given chain ID
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "https://cloudflare-eth.com",
        8453 => "https://base-rpc.publicnode.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        _ => "https://base-rpc.publicnode.com",
    }
}

/// Default slippage tolerance (0.5%)
pub const DEFAULT_SLIPPAGE: f64 = 0.005;

/// Slippage for AMM swaps (1%)
pub const SWAP_SLIPPAGE: f64 = 0.01;

/// Warn if price impact exceeds 3%
pub const MAX_PRICE_IMPACT_WARN: f64 = 0.03;

/// Block if price impact exceeds 10%
pub const MAX_PRICE_IMPACT_BLOCK: f64 = 0.10;

// ─── Router Command byte values (from Commands.sol) ─────────────────────────
/// transfer tokens from sender to router
pub const CMD_TRANSFER_FROM: u8 = 0x00;
/// swap on Curve StableSwap NG pool (weETH pool on Base uses this)
pub const CMD_CURVE_SWAP_SNG: u8 = 0x1E;
/// swap on legacy Curve CryptoSwap pool
pub const CMD_CURVE_SWAP: u8 = 0x03;
/// deposit underlying asset into IBT (ERC-4626 wrap)
pub const CMD_DEPOSIT_ASSET_IN_IBT: u8 = 0x04;
/// deposit underlying asset into PT
pub const CMD_DEPOSIT_ASSET_IN_PT: u8 = 0x05;
/// deposit IBT into PT
pub const CMD_DEPOSIT_IBT_IN_PT: u8 = 0x06;
/// redeem IBT for underlying
pub const CMD_REDEEM_IBT_FOR_ASSET: u8 = 0x07;
/// redeem PT for underlying
pub const CMD_REDEEM_PT_FOR_ASSET: u8 = 0x08;
/// redeem PT for IBT
pub const CMD_REDEEM_PT_FOR_IBT: u8 = 0x09;

// ─── Known Base pools (for get-pools fallback & swap) ───────────────────────
pub struct KnownPool {
    pub name: &'static str,
    pub pt: &'static str,
    pub yt: &'static str,
    pub ibt: &'static str,
    pub underlying: &'static str,
    pub curve_pool: &'static str,
    pub maturity: u64, // Unix timestamp
    pub curve_pool_type: &'static str, // "sng" | "ng" | "legacy"
}

pub const KNOWN_BASE_POOLS: &[KnownPool] = &[
    KnownPool {
        name: "weETH (Ether.fi)",
        pt: "0x07f58450a39d07f9583c188a2a4a441fac358100",
        yt: "0xd29fb7faFdBee7164C781A56A623b38E040030bB",
        ibt: "0x22f757c0b434d93c93d9653f26c9441d8d06c8ec",
        underlying: "0x4200000000000000000000000000000000000006", // WETH
        curve_pool: "0x3870a9498cd7ced8d134f19b0092931ef83aec1e",
        maturity: 1752537600, // Jul 15 2026 UTC
        curve_pool_type: "sng",
    },
    KnownPool {
        name: "sjEUR (Jarvis)",
        pt: "0x3928cbccc982efbadbc004977827325b0be4c346",
        yt: "0x97b6D8d8534455d9A9A36ca7a95CC862c9c05E0B",
        ibt: "0x89cc2a57223fa803852b6b4e65c6e376d49909f9",
        underlying: "0xfcde3a0b7e4c4780afde0b41c4df32b85ba6de36",
        curve_pool: "0xa86bee5400d9f58aa2ff168fed6ab4bcb36bcc91",
        maturity: 1752624000, // Jul 16 2026 UTC
        curve_pool_type: "sng",
    },
    KnownPool {
        name: "wsuperOETHb (Origin)",
        pt: "0x1dc1b09d656c07404aa2747a9930c0b4d297b4f3",
        yt: "0x3CdC2D0AE59bE92A4fd7bc92B66C215609857B2b",
        ibt: "0x7fcd174e80f264448ebee8c88a7c4476aaf58ea6",
        underlying: "0xdbfea8154f0a4e049663b072d8573220f95b36d3",
        curve_pool: "0xd296a4ec9cde7f864c87f1d37a9529fb02ceb129",
        maturity: 1748736000, // Jun 1 2026 UTC
        curve_pool_type: "sng",
    },
];
