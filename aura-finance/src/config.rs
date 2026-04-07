/// RPC endpoint for Ethereum mainnet
pub const ETH_RPC: &str = "https://ethereum.publicnode.com";

/// Aura Finance contract addresses (Ethereum mainnet)
pub const BOOSTER: &str = "0xA57b8d98dAE62B26Ec3bcC4a365338157060B234";
pub const AURA_LOCKER: &str = "0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC";

/// Token addresses (Ethereum mainnet)
pub const AURA_TOKEN: &str = "0xC0c293ce456fF0ED870ADd98a0828Dd4d2903DBF";
pub const AURA_BAL: &str = "0x616e8BfA43F920657B3497DBf40D6b1A02D4608d";
pub const BAL_TOKEN: &str = "0xba100000625a3754423978a60c9317c58a424e3D";

/// Max pools to iterate over on-chain (prevents RPC waterfall)
pub const MAX_POOLS: u64 = 50;

/// Balancer REST API (for pool APY/TVL data)
pub const BALANCER_API_BASE: &str = "https://api.balancer.fi";
