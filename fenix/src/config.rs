// config.rs — Fenix Finance constants (Blast/81457 only)

pub const CHAIN_ID: u64 = 81457;
pub const RPC_URL: &str = "https://rpc.blast.io";
pub const EXPLORER_TX: &str = "https://blastscan.io/tx/";

// Contract addresses
pub const SWAP_ROUTER: &str = "0x2df37Cb897fdffc6B4b03d8252d85BE7C6dA9d00";
pub const NFPM: &str = "0x8881b3Fb762d1D50e6172f621F107E24299AA1Cd";
pub const QUOTER_V2: &str = "0x94Ca5B835186A37A99776780BF976fAB81D84ED8";
pub const FACTORY: &str = "0x7a44CD060afC1B6F4c80A2B9b37f4473E74E25Df";

// Known token addresses on Blast
pub const WETH: &str = "0x4300000000000000000000000000000000000004";
pub const USDB: &str = "0x4300000000000000000000000000000000000003";
pub const FNX: &str = "0x52f847356b38720B55ee18Cb3e094ca11C85A192";

// GraphQL endpoint
pub const GRAPHQL_URL: &str = "https://api.goldsky.com/api/public/project_clxadvm41bujy01ui2qalezdn/subgraphs/fenix-v3-dex/latest/gn";

/// Resolve a token symbol or hex address to its checksummed address.
/// Returns the input unchanged if it is already a 0x-prefixed address.
pub fn resolve_token(symbol_or_addr: &str) -> String {
    match symbol_or_addr.to_uppercase().as_str() {
        "WETH" => WETH.to_string(),
        "USDB" => USDB.to_string(),
        "FNX" => FNX.to_string(),
        _ => symbol_or_addr.to_string(),
    }
}

pub fn explorer_url(tx_hash: &str) -> String {
    format!("{}{}", EXPLORER_TX, tx_hash)
}
