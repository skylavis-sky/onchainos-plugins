# Selector Verification — Maple Finance

All selectors verified with `cast sig` (Foundry).

| Function | Selector | Verified |
|----------|----------|---------|
| `deposit(uint256,address)` (ERC-4626 Pool direct) | `0x6e553f65` | ✅ `cast sig "deposit(uint256,address)"` |
| `deposit(uint256,bytes32)` (SyrupRouter) | `0xc9630cb0` | ✅ `cast sig "deposit(uint256,bytes32)"` |
| `requestRedeem(uint256,address)` (Pool) | `0x107703ab` | ✅ `cast sig "requestRedeem(uint256,address)"` |
| `redeem(uint256,address,address)` | `0xba087652` | ✅ `cast sig "redeem(uint256,address,address)"` |
| `balanceOf(address)` | `0x70a08231` | ✅ `cast sig "balanceOf(address)"` |
| `balanceOfAssets(address)` | `0x9159b206` | ✅ `cast sig "balanceOfAssets(address)"` |
| `totalAssets()` | `0x01e1d114` | ✅ `cast sig "totalAssets()"` |
| `convertToAssets(uint256)` | `0x07a2d13a` | ✅ `cast sig "convertToAssets(uint256)"` |
| `convertToExitAssets(uint256)` | `0x50496cbd` | ✅ `cast sig "convertToExitAssets(uint256)"` |
| `totalSupply()` | `0x18160ddd` | ✅ `cast sig "totalSupply()"` |
| `approve(address,uint256)` (ERC-20) | `0x095ea7b3` | ✅ standard ERC-20 selector |
| `allowance(address,address)` (ERC-20) | `0xdd62ed3e` | ✅ standard ERC-20 selector |

## Validation Notes

- L3 dry-run tests confirmed calldata selectors match expected values:
  - `deposit --dry-run` produces `0xc9630cb0...` ✅
  - `withdraw --dry-run` produces `0x107703ab...` ✅
- `convertToExitAssets` verified live on-chain: calling with 1,000,000 shares on syrupUSDC returns 1,158,428 (exchange rate > 1.0 confirms yield accrual)
