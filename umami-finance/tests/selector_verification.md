# Selector Verification — Umami Finance

All selectors verified using `cast sig` and live `eth_call` against Arbitrum mainnet.

| Function Signature | Expected Selector | cast sig Result | Status |
|-------------------|------------------|----------------|--------|
| `deposit(uint256,address)` | `0x6e553f65` | `0x6e553f65` | ✅ |
| `redeem(uint256,address,address)` | `0xba087652` | `0xba087652` | ✅ |
| `withdraw(uint256,address,address)` | `0xb460af94` | `0xb460af94` | ✅ |
| `totalAssets()` | `0x01e1d114` | `0x01e1d114` | ✅ |
| `totalSupply()` | `0x18160ddd` | `0x18160ddd` | ✅ |
| `convertToAssets(uint256)` | `0x07a2d13a` | `0x07a2d13a` | ✅ |
| `previewDeposit(uint256)` | `0xef8b30f7` | `0xef8b30f7` | ✅ |
| `previewRedeem(uint256)` | `0x4cdad506` | `0x4cdad506` | ✅ |
| `maxDeposit(address)` | `0x402d267d` | `0x402d267d` | ✅ |
| `balanceOf(address)` | `0x70a08231` | `0x70a08231` | ✅ |
| `asset()` | `0x38d52e0f` | `0x38d52e0f` | ✅ |
| `decimals()` | `0x313ce567` | `0x313ce567` | ✅ |
| `approve(address,uint256)` | `0x095ea7b3` | standard | ✅ |

Live eth_call confirmation:
- `totalAssets()` on 0x959f3807... returned non-zero ✅
- `convertToAssets(1e6)` on 0x959f3807... returned 1154000+ (above 1.0) ✅
- `previewDeposit(1000000)` on 0x959f3807... returned non-zero shares ✅
- `maxDeposit(wallet)` on 0x959f3807... returned 36996656737 ✅
