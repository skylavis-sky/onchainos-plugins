/// Solana chain ID used by onchainos
pub const SOLANA_CHAIN_ID: &str = "501";

/// pump.fun program address on Solana mainnet
pub const PUMPFUN_PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

/// Default Solana RPC endpoint (public mainnet-beta)
pub const DEFAULT_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Default slippage in basis points (100 = 1%)
pub const DEFAULT_SLIPPAGE_BPS: u64 = 100;

/// Default compute unit limit for priority fee
pub const DEFAULT_PRIORITY_FEE_UNIT_LIMIT: u32 = 200_000;

/// Default micro-lamports per compute unit
pub const DEFAULT_PRIORITY_FEE_UNIT_PRICE: u64 = 1_000;

/// Fee basis points used in sell price calculation (pump.fun standard 1%)
pub const FEE_BASIS_POINTS: u64 = 100;

/// Approximate SOL threshold for bonding curve graduation (~85 SOL in lamports)
pub const GRADUATION_SOL_THRESHOLD: u64 = 85_000_000_000;
