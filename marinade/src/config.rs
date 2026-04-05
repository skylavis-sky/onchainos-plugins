// Marinade Finance — Key Addresses and Configuration

/// Solana chain ID
pub const SOLANA_CHAIN_ID: u64 = 501;

/// mSOL mint address
pub const MSOL_MINT: &str = "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So";

/// Native SOL address used by Jupiter routing
pub const SOL_NATIVE: &str = "11111111111111111111111111111111";

/// Marinade liquid staking program ID
pub const MARINADE_PROGRAM_ID: &str = "MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD";

/// Marinade state account address
pub const MARINADE_STATE_ACCOUNT: &str = "8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC";

/// Solana mainnet RPC URL
pub const SOLANA_RPC_URL: &str = "https://api.mainnet-beta.solana.com";

/// Marinade REST API — mSOL/SOL price
pub const MARINADE_PRICE_API: &str = "https://api.marinade.finance/msol/price_sol";

/// Approximate staking APY (Marinade ~7%, kept as static reference)
pub const APPROX_STAKING_APY: &str = "~7%";
