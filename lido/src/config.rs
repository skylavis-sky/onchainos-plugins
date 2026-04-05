/// Chain configuration and contract addresses for Lido plugin.
/// All addresses sourced from https://docs.lido.fi/deployed-contracts/

pub struct ChainConfig {
    pub rpc_url: &'static str,
    pub wsteth_address: &'static str,
}

// Ethereum-only contracts
pub const STETH_ADDRESS: &str = "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84";
pub const WSTETH_ETH_ADDRESS: &str = "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0";
pub const WITHDRAWAL_QUEUE_ADDRESS: &str = "0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1";

// Ethereum chain ID
pub const CHAIN_ETHEREUM: u64 = 1;

// RPC endpoints
pub const RPC_ETHEREUM: &str = "https://ethereum.publicnode.com";
pub const RPC_ARBITRUM: &str = "https://arb1.arbitrum.io/rpc";
pub const RPC_BASE: &str = "https://base-rpc.publicnode.com";
pub const RPC_OPTIMISM: &str = "https://mainnet.optimism.io";

// Lido REST API
pub const LIDO_APR_SMA_URL: &str = "https://eth-api.lido.fi/v1/protocol/steth/apr/sma";
pub const LIDO_APR_LAST_URL: &str = "https://eth-api.lido.fi/v1/protocol/steth/apr/last";
pub const LIDO_WQ_API_URL: &str = "https://wq-api.lido.fi/v2/request-time";

// Max/min withdrawal amounts
pub const MAX_WITHDRAWAL_PER_REQUEST: u128 = 1_000_000_000_000_000_000_000; // 1000 stETH in wei
pub const MIN_WITHDRAWAL_AMOUNT: u128 = 100; // 100 gwei

pub fn get_chain_config(chain_id: u64) -> Option<ChainConfig> {
    match chain_id {
        1 => Some(ChainConfig {
            rpc_url: RPC_ETHEREUM,
            wsteth_address: WSTETH_ETH_ADDRESS,
        }),
        42161 => Some(ChainConfig {
            rpc_url: RPC_ARBITRUM,
            wsteth_address: "0x5979D7b546E38E414F7E9822514be443A4800529",
        }),
        8453 => Some(ChainConfig {
            rpc_url: RPC_BASE,
            wsteth_address: "0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452",
        }),
        10 => Some(ChainConfig {
            rpc_url: RPC_OPTIMISM,
            wsteth_address: "0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb",
        }),
        _ => None,
    }
}
