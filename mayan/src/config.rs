// Constants and chain support configuration for Mayan plugin

pub const PRICE_API_BASE: &str = "https://price-api.mayan.finance/v3";
pub const EXPLORER_API_BASE: &str = "https://explorer-api.mayan.finance/v3";

/// Mayan Forwarder Contract — identical on all EVM chains
pub const MAYAN_FORWARDER_CONTRACT: &str = "0x337685fdaB40D39bd02028545a4FfA7D287cC3E2";

/// Swift v2 program (Solana)
pub const SWIFT_V2_PROGRAM_ID: &str = "mayan34VedncxdK2XobtvWFDXQASUTBXhUVzt2kKgny";
/// MCTP program (Solana)
pub const MCTP_PROGRAM_ID: &str = "dkpZqrxHFrhziEMQ931GLtfy11nFkCsfMftH9u6QwBU";
/// WH Swap program (Solana, legacy)
pub const MAYAN_PROGRAM_ID: &str = "FC4eXxkyrMPTjiYUpp4EAnkmwMbQyZ6NDCh1kfLn6vsf";

/// onchainos chain ID for Solana
pub const SOLANA_CHAIN_ID: u64 = 501;

/// Native ETH placeholder address (EVM)
pub const NATIVE_ETH_ADDR: &str = "0x0000000000000000000000000000000000000000";
/// Native SOL placeholder address (Solana)
pub const NATIVE_SOL_ADDR: &str = "11111111111111111111111111111111";

pub const DEFAULT_SLIPPAGE_BPS: u32 = 100;
#[allow(dead_code)]
pub const MAX_SLIPPAGE_BPS: u32 = 300;

/// ERC-20 approve(address,uint256) selector
#[allow(dead_code)]
pub const ERC20_APPROVE_SELECTOR: &str = "095ea7b3";
/// approve calldata: approve(forwarder, max_uint256)
pub const APPROVE_FORWARDER_CALLDATA: &str =
    "0x095ea7b3000000000000000000000000337685fdab40d39bd02028545a4ffa7d287cc3e2\
     ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

/// Map onchainos numeric chain ID -> Mayan chain name string
pub fn chain_id_to_mayan_name(chain_id: u64) -> Option<&'static str> {
    match chain_id {
        1 => Some("ethereum"),
        56 => Some("bsc"),
        137 => Some("polygon"),
        43114 => Some("avalanche"),
        42161 => Some("arbitrum"),
        10 => Some("optimism"),
        8453 => Some("base"),
        501 => Some("solana"),
        _ => None,
    }
}

/// Map Mayan chain name -> onchainos numeric chain ID
#[allow(dead_code)]
pub fn mayan_name_to_chain_id(name: &str) -> Option<u64> {
    match name.to_lowercase().as_str() {
        "ethereum" => Some(1),
        "bsc" => Some(56),
        "polygon" => Some(137),
        "avalanche" => Some(43114),
        "arbitrum" => Some(42161),
        "optimism" => Some(10),
        "base" => Some(8453),
        "solana" => Some(501),
        _ => None,
    }
}

/// Returns true if chain_id is an EVM chain (not Solana)
pub fn is_evm_chain(chain_id: u64) -> bool {
    chain_id != SOLANA_CHAIN_ID
}

/// Returns true if token address is native (ETH zero-address or SOL system program)
pub fn is_native_token(token: &str) -> bool {
    token == NATIVE_ETH_ADDR || token == NATIVE_SOL_ADDR
}
