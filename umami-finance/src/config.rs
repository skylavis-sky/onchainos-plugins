/// Chain and contract configuration for Umami Finance GM Vaults on Arbitrum

pub const ARBITRUM_CHAIN_ID: u64 = 42161;
pub const ARBITRUM_RPC: &str = "https://arb1.arbitrum.io/rpc";

/// GM Vault addresses on Arbitrum
pub struct VaultInfo {
    pub name: &'static str,
    pub address: &'static str,
    pub asset_address: &'static str,
    pub asset_symbol: &'static str,
    pub asset_decimals: u32,
    pub description: &'static str,
}

pub const VAULTS: &[VaultInfo] = &[
    VaultInfo {
        name: "gmUSDC-eth",
        address: "0x959f3807f0Aa7921E18c78B00B2819ba91E52FeF",
        asset_address: "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        asset_symbol: "USDC",
        asset_decimals: 6,
        description: "GM USDC Vault (ETH-backed) — earns yield from ETH/XRP/DOGE/LTC GMX V2 markets",
    },
    VaultInfo {
        name: "gmUSDC-btc",
        address: "0x5f851F67D24419982EcD7b7765deFD64fBb50a97",
        asset_address: "0xaf88d065e77c8cc2239327c5edb3a432268e5831",
        asset_symbol: "USDC",
        asset_decimals: 6,
        description: "GM USDC Vault (BTC-backed) — earns yield from BTC GMX V2 markets",
    },
    VaultInfo {
        name: "gmWETH",
        address: "0x4bCA8D73561aaEee2D3a584b9F4665310de1dD69",
        asset_address: "0x82af49447d8a07e3bd95bd0d56f35241523fbab1",
        asset_symbol: "WETH",
        asset_decimals: 18,
        description: "GM WETH Vault — earns yield in WETH from GMX V2 markets",
    },
    VaultInfo {
        name: "gmWBTC",
        address: "0xcd8011AaB161A75058eAb24e0965BAb0b918aF29",
        asset_address: "0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f",
        asset_symbol: "WBTC",
        asset_decimals: 8,
        description: "GM WBTC Vault — earns yield in WBTC from GMX V2 markets",
    },
];

/// Find vault by name (case-insensitive) or address
pub fn find_vault(identifier: &str) -> Option<&'static VaultInfo> {
    let lower = identifier.to_lowercase();
    VAULTS.iter().find(|v| {
        v.name.to_lowercase() == lower
            || v.address.to_lowercase() == lower
            || v.asset_symbol.to_lowercase() == lower
    })
}
