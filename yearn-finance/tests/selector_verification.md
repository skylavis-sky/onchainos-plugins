# Selector Verification Checklist — Yearn Finance

All selectors verified via `cast sig` on 2026-04-05.

| Function Signature | cast sig Result | Code Value | Status |
|-------------------|----------------|------------|--------|
| `deposit(uint256,address)` | `0x6e553f65` | `0x6e553f65` | ✅ |
| `redeem(uint256,address,address)` | `0xba087652` | `0xba087652` | ✅ |
| `approve(address,uint256)` | `0x095ea7b3` | `0x095ea7b3` | ✅ |
| `balanceOf(address)` | `0x70a08231` | `0x70a08231` | ✅ |
| `pricePerShare()` | `0x99530b06` | `0x99530b06` | ✅ |
| `totalAssets()` | `0x01e1d114` | `0x01e1d114` | ✅ |
| `asset()` | `0x38d52e0f` | `0x38d52e0f` | ✅ |

All 7 selectors verified correct.
