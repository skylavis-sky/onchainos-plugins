// Contract addresses and constants for Solv SolvBTC plugin

pub const CHAIN_ARBITRUM: u64 = 42161;
pub const CHAIN_ETHEREUM: u64 = 1;

// Arbitrum (42161)
pub const ARB_SOLVBTC_TOKEN: &str = "0x3647c54c4c2C65bC7a2D63c0Da2809B399DBBDC0";
pub const ARB_WBTC_TOKEN: &str = "0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f";
pub const ARB_ROUTER_V2: &str = "0x92E8A4407FD1ae7a53a32f1f832184edF071080A";

// Ethereum (1)
pub const ETH_SOLVBTC_TOKEN: &str = "0x7a56e1c57c7475ccf742a1832b028f0456652f97";
pub const ETH_XSOLVBTC_TOKEN: &str = "0xd9d920aa40f578ab794426f5c90f6c731d159def";
pub const ETH_WBTC_TOKEN: &str = "0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599";
pub const ETH_ROUTER_V2: &str = "0x3d93B9e8F0886358570646dAd9421564C5fE6334";
pub const ETH_XSOLVBTC_POOL: &str = "0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86";

// Token decimals
#[allow(dead_code)]
pub const WBTC_DECIMALS: u32 = 8;
pub const SOLVBTC_DECIMALS: u32 = 18;
#[allow(dead_code)]
pub const XSOLVBTC_DECIMALS: u32 = 18;

// DeFiLlama coin keys
pub const DEFI_LLAMA_SOLVBTC_ARB: &str =
    "arbitrum:0x3647c54c4c2c65bc7a2d63c0da2809b399dbbdc0";
pub const DEFI_LLAMA_SOLVBTC_ETH: &str =
    "ethereum:0x7a56e1c57c7475ccf742a1832b028f0456652f97";
pub const DEFI_LLAMA_XSOLVBTC_ETH: &str =
    "ethereum:0xd9d920aa40f578ab794426f5c90f6c731d159def";
pub const DEFI_LLAMA_PROTOCOL_SLUG: &str = "solv-protocol";

// Function selectors
pub const SEL_APPROVE: &str = "095ea7b3";
pub const SEL_ROUTER_DEPOSIT: &str = "672262e5";
pub const SEL_ROUTER_WITHDRAW_REQUEST: &str = "d2cfd97d";
pub const SEL_ROUTER_CANCEL_WITHDRAW: &str = "42c7774b";
pub const SEL_XPOOL_DEPOSIT: &str = "b6b55f25";
pub const SEL_XPOOL_WITHDRAW: &str = "2e1a7d4d";
pub const SEL_BALANCE_OF: &str = "70a08231";

// xSolvBTC withdraw fee: 5/10000 = 0.05%
pub const XSOLVBTC_WITHDRAW_FEE_RATE: u64 = 5;
pub const XSOLVBTC_WITHDRAW_FEE_DENOM: u64 = 10_000;

/// Return (solvbtc_addr, wbtc_addr, router_v2_addr) for a given chain.
pub fn chain_contracts(chain_id: u64) -> anyhow::Result<(&'static str, &'static str, &'static str)> {
    match chain_id {
        CHAIN_ARBITRUM => Ok((ARB_SOLVBTC_TOKEN, ARB_WBTC_TOKEN, ARB_ROUTER_V2)),
        CHAIN_ETHEREUM => Ok((ETH_SOLVBTC_TOKEN, ETH_WBTC_TOKEN, ETH_ROUTER_V2)),
        other => anyhow::bail!("Unsupported chain ID {}. Supported: 1 (Ethereum), 42161 (Arbitrum)", other),
    }
}
