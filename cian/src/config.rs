// Chain IDs
pub const CHAIN_ETHEREUM: u64 = 1;
pub const CHAIN_ARBITRUM: u64 = 42161;
pub const CHAIN_BSC: u64 = 56;
pub const CHAIN_MANTLE: u64 = 5000;

// Function selectors
#[allow(dead_code)]
pub const SEL_APPROVE: &str = "095ea7b3";
pub const SEL_OPTIONAL_DEPOSIT: &str = "32507a5f";
#[allow(dead_code)]
pub const SEL_DEPOSIT: &str = "6e553f65";
pub const SEL_REQUEST_REDEEM_ETH: &str = "107703ab"; // requestRedeem(uint256,address)
pub const SEL_REQUEST_REDEEM_BTC: &str = "aa2f892d"; // requestRedeem(uint256)
#[allow(dead_code)]
pub const SEL_ASSET: &str = "38d52e0f";
#[allow(dead_code)]
pub const SEL_BALANCE_OF: &str = "70a08231";
#[allow(dead_code)]
pub const SEL_EXCHANGE_PRICE: &str = "9e65741e";
#[allow(dead_code)]
pub const SEL_MAX_DEPOSIT: &str = "402d267d";

// Zero address (referral default)
pub const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

// Max uint256 for approvals
pub const MAX_UINT256: u128 = u128::MAX;

// pumpBTC-type vault addresses (use requestRedeem(uint256) — single param)
// These use the BTC-class redeem signature
pub const PUMP_BTC_VAULTS: &[&str] = &[
    "0xd4cc9b31e9ef33e392ff2f81ad52be8523e0993b", // Ethereum pumpBTC
];

// Vault addresses per chain
// Ethereum (1)
pub const ETH_VAULT_STETH: &str = "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d";
#[allow(dead_code)]
pub const ETH_VAULT_RSETH: &str = "0xd87a19fF681AE98BF10d2220D1AE3Fbd374ADE4e";
#[allow(dead_code)]
pub const ETH_VAULT_BTCLST: &str = "0x6c77bdE03952BbcB923815d90A73a7eD7EC895D1";
#[allow(dead_code)]
pub const ETH_VAULT_UNIBTC: &str = "0xcc7E6dE27DdF225E24E8652F62101Dab4656E20A";
#[allow(dead_code)]
pub const ETH_VAULT_EZETH: &str = "0x3D086B688D7c0362BE4f9600d626f622792c4a20";
#[allow(dead_code)]
pub const ETH_VAULT_PUMPBTC: &str = "0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b";
#[allow(dead_code)]
pub const ETH_VAULT_FBTC: &str = "0x8D76e7847dFbEA6e9F4C235CADF51586bA3560A2";
#[allow(dead_code)]
pub const ETH_VAULT_WBETH: &str = "0x7bFC0E982985556D17539Adc630d8BF888d9004a";
#[allow(dead_code)]
pub const ETH_VAULT_USDC: &str = "0x7223d0bc232E369F1cbdB6ACB383E09aF4B09bD6";

// Arbitrum (42161)
#[allow(dead_code)]
pub const ARB_VAULT_RSETH: &str = "0x15cbFF12d53e7BdE3f1618844CaaEf99b2836d2A";

// BSC (56)
#[allow(dead_code)]
pub const BSC_VAULT_SLISBNB: &str = "0x406e1e0e3cb4201B4AEe409Ad2f6Cd56d3242De7";
#[allow(dead_code)]
pub const BSC_VAULT_BTCB: &str = "0x74D2Bef5Afe200DaCC76FE2D3C4022435b54CdbB";
#[allow(dead_code)]
pub const BSC_VAULT_USD1: &str = "0xD896bf804c01c4C0Fa5C42bF6A4b15C465009481";

// Mantle (5000)
#[allow(dead_code)]
pub const MANTLE_VAULT_BYBIT_USDT0: &str = "0x74D2Bef5Afe200DaCC76FE2D3C4022435b54CdbB";
#[allow(dead_code)]
pub const MANTLE_VAULT_BYBIT_USDC: &str = "0x6B2BA8F249cC1376f2A02A9FaF8BEcA5D7718DCf";

// RPC endpoints per chain
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        CHAIN_ETHEREUM => "https://1rpc.io/eth",
        CHAIN_ARBITRUM => "https://arb1.arbitrum.io/rpc",
        CHAIN_BSC => "https://bsc-rpc.publicnode.com",
        CHAIN_MANTLE => "https://rpc.mantle.xyz",
        _ => "https://cloudflare-eth.com",
    }
}

/// Vault registry entry
pub struct VaultInfo {
    pub chain_id: u64,
    pub address: &'static str,
    pub name: &'static str,
    pub strategy: &'static str,
}

/// All known CIAN vaults
pub const VAULTS: &[VaultInfo] = &[
    VaultInfo { chain_id: 1,     address: "0xB13aa2d0345b0439b064f26B82D8dCf3f508775d", name: "CIAN stETH Vault",    strategy: "ETH LST" },
    VaultInfo { chain_id: 1,     address: "0xd87a19fF681AE98BF10d2220D1AE3Fbd374ADE4e", name: "CIAN rsETH Vault",   strategy: "ETH LRT" },
    VaultInfo { chain_id: 1,     address: "0x3D086B688D7c0362BE4f9600d626f622792c4a20", name: "CIAN ezETH Vault",   strategy: "ETH LRT" },
    VaultInfo { chain_id: 1,     address: "0x6c77bdE03952BbcB923815d90A73a7eD7EC895D1", name: "CIAN BTC LST Vault", strategy: "BTC LST" },
    VaultInfo { chain_id: 1,     address: "0xcc7E6dE27DdF225E24E8652F62101Dab4656E20A", name: "CIAN uniBTC Vault",  strategy: "BTC LST" },
    VaultInfo { chain_id: 1,     address: "0xd4Cc9b31e9eF33E392FF2f81AD52BE8523e0993b", name: "CIAN pumpBTC Vault", strategy: "BTC LST" },
    VaultInfo { chain_id: 1,     address: "0x8D76e7847dFbEA6e9F4C235CADF51586bA3560A2", name: "CIAN FBTC Vault",    strategy: "BTC LST" },
    VaultInfo { chain_id: 1,     address: "0x7bFC0E982985556D17539Adc630d8BF888d9004a", name: "CIAN wBETH Vault",   strategy: "ETH LST" },
    VaultInfo { chain_id: 1,     address: "0x7223d0bc232E369F1cbdB6ACB383E09aF4B09bD6", name: "CIAN USDC Vault",    strategy: "Stablecoin" },
    VaultInfo { chain_id: 42161, address: "0x15cbFF12d53e7BdE3f1618844CaaEf99b2836d2A", name: "CIAN rsETH Vault",   strategy: "ETH LRT" },
    VaultInfo { chain_id: 56,    address: "0x406e1e0e3cb4201B4AEe409Ad2f6Cd56d3242De7", name: "CIAN slisBNB Vault", strategy: "BNB LST" },
    VaultInfo { chain_id: 56,    address: "0x74D2Bef5Afe200DaCC76FE2D3C4022435b54CdbB", name: "CIAN BTCB Vault",    strategy: "BTC" },
    VaultInfo { chain_id: 56,    address: "0xD896bf804c01c4C0Fa5C42bF6A4b15C465009481", name: "CIAN USD1 Vault",    strategy: "Stablecoin" },
    VaultInfo { chain_id: 5000,  address: "0x74D2Bef5Afe200DaCC76FE2D3C4022435b54CdbB", name: "CIAN Bybit USDT0",  strategy: "Stablecoin" },
    VaultInfo { chain_id: 5000,  address: "0x6B2BA8F249cC1376f2A02A9FaF8BEcA5D7718DCf", name: "CIAN Bybit USDC",   strategy: "Stablecoin" },
];

/// Human-readable chain name
pub fn chain_display_name(chain_id: u64) -> &'static str {
    match chain_id {
        CHAIN_ETHEREUM => "Ethereum",
        CHAIN_ARBITRUM => "Arbitrum",
        CHAIN_BSC => "BSC",
        CHAIN_MANTLE => "Mantle",
        _ => "Unknown",
    }
}

/// Detect if a vault uses the BTC-class requestRedeem(uint256) signature
pub fn is_btc_class_vault(vault_addr: &str) -> bool {
    let lower = vault_addr.to_lowercase();
    PUMP_BTC_VAULTS.iter().any(|v| v.to_lowercase() == lower)
}
