# Selector Verification Checklist

All EVM function selectors verified via `cast sig`.

| 函数签名 | cast sig 结果 | 代码中的值 (config.rs) | 状态 |
|---------|-------------|----------------------|------|
| `deposit()` | `0xd0e30db0` | `d0e30db0` (SEL_DEPOSIT) | ✅ |
| `depositWithReferral(address)` | `0xc18d7cb7` | `c18d7cb7` (SEL_DEPOSIT_WITH_REFERRAL) | ✅ |
| `swETHToETHRate()` | `0xd68b2cb6` | `d68b2cb6` (SEL_SWETH_TO_ETH_RATE) | ✅ |
| `ethToSwETHRate()` | `0x0de3ff57` | `0de3ff57` (SEL_ETH_TO_SWETH_RATE) | ✅ |
| `rswETHToETHRate()` | `0xa7b9544e` | `a7b9544e` (SEL_RSWETH_TO_ETH_RATE) | ✅ |
| `ethToRswETHRate()` | `0x780a47e0` | `780a47e0` (SEL_ETH_TO_RSWETH_RATE) | ✅ |
| `balanceOf(address)` | `0x70a08231` | `70a08231` (SEL_BALANCE_OF) | ✅ |
| `totalSupply()` | `0x18160ddd` | `18160ddd` (SEL_TOTAL_SUPPLY) | ✅ |

Live eth_call verification:
- `swETHToETHRate()` → `1119021048244912839` (1.119 ETH per swETH) ✅
- `rswETHToETHRate()` → `1069026676734286391` (1.069 ETH per rswETH) ✅
- `ethToRswETHRate()` → `935430351518306006` (0.935 rswETH per ETH) ✅
