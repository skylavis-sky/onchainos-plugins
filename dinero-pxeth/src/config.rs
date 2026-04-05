/// Ethereum mainnet chain ID
pub const CHAIN_ID: u64 = 1;

/// PirexEth main contract — deposit ETH to receive pxETH
/// ⚠️ Currently PAUSED — deposit/redeem functions revert with whenNotPaused
pub const PIREXETH: &str = "0xD664b74274DfEB538d9baC494F3a4760828B02b0";

/// pxETH ERC-20 token
pub const PXETH_TOKEN: &str = "0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6";

/// apxETH ERC-4626 vault — deposit pxETH to earn auto-compounding yield
/// ✅ Active — no pause mechanism
pub const APXETH_VAULT: &str = "0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6";

// === Function Selectors (verified with `cast sig`) ===

/// deposit(address,bool) — PirexEth: deposit ETH, receive pxETH or apxETH
/// cast sig "deposit(address,bool)" → 0xadc9740c ✅
/// ⚠️ Currently paused on PirexEth contract
pub const SEL_PIREX_DEPOSIT: &str = "adc9740c";

/// deposit(uint256,address) — ERC-4626 deposit (apxETH vault)
/// cast sig "deposit(uint256,address)" → 0x6e553f65 ✅
pub const SEL_DEPOSIT: &str = "6e553f65";

/// redeem(uint256,address,address) — ERC-4626 redeem (apxETH vault)
/// cast sig "redeem(uint256,address,address)" → 0xba087652 ✅
pub const SEL_REDEEM: &str = "ba087652";

/// convertToAssets(uint256) — ERC-4626 price query
/// cast sig "convertToAssets(uint256)" → 0x07a2d13a ✅
pub const SEL_CONVERT_TO_ASSETS: &str = "07a2d13a";

/// totalAssets() — ERC-4626 total pxETH in vault
/// cast sig "totalAssets()" → 0x01e1d114 ✅
pub const SEL_TOTAL_ASSETS: &str = "01e1d114";

/// totalSupply() — ERC-20 total supply
/// cast sig "totalSupply()" → 0x18160ddd ✅
pub const SEL_TOTAL_SUPPLY: &str = "18160ddd";

/// balanceOf(address) — ERC-20 balance
/// cast sig "balanceOf(address)" → 0x70a08231 ✅
pub const SEL_BALANCE_OF: &str = "70a08231";

/// approve(address,uint256) — ERC-20 approve
/// cast sig "approve(address,uint256)" → 0x095ea7b3 ✅
pub const SEL_APPROVE: &str = "095ea7b3";

/// paused() — PirexEth pause state check
/// cast sig "paused()" → 0x5c975abb ✅
pub const SEL_PAUSED: &str = "5c975abb";
