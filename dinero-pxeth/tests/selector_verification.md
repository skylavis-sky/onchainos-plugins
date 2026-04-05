# Selector 核对清单

All selectors verified with `cast sig`.

| 函数签名 | cast sig 结果 | 代码中的值 | 状态 |
|---------|-------------|----------|------|
| `deposit(address,bool)` | `0xadc9740c` | `adc9740c` | ✅ |
| `deposit(uint256,address)` | `0x6e553f65` | `6e553f65` | ✅ |
| `redeem(uint256,address,address)` | `0xba087652` | `ba087652` | ✅ |
| `convertToAssets(uint256)` | `0x07a2d13a` | `07a2d13a` | ✅ |
| `totalAssets()` | `0x01e1d114` | `01e1d114` | ✅ |
| `totalSupply()` | `0x18160ddd` | `18160ddd` | ✅ |
| `balanceOf(address)` | `0x70a08231` | `70a08231` | ✅ |
| `approve(address,uint256)` | `0x095ea7b3` | `095ea7b3` | ✅ |
| `paused()` | `0x5c975abb` | `5c975abb` | ✅ |

All 9 selectors verified ✅
