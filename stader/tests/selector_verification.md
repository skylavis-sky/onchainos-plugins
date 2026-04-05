# Selector Verification Checklist

All selectors verified with `cast sig` on 2026-04-05.

## StaderStakePoolsManager (0xcf5EA1b38380f6aF39068375516Daf40Ed70D299)

| Function Signature | cast sig Result | Code Value | Status |
|---|---|---|---|
| `deposit(address)` | `0xf340fa01` | `0xf340fa01` | ✅ |
| `getExchangeRate()` | `0xe6aa216c` | `0xe6aa216c` | ✅ |
| `convertToShares(uint256)` | `0xc6e6f592` | `0xc6e6f592` | ✅ |
| `convertToAssets(uint256)` | `0x07a2d13a` | `0x07a2d13a` | ✅ |
| `maxDeposit()` | `0x6083e59a` | `0x6083e59a` | ✅ |
| `minDeposit()` | `0x41b3d185` | `0x41b3d185` | ✅ |
| `previewDeposit(uint256)` | `0xef8b30f7` | `0xef8b30f7` | ✅ |
| `totalAssets()` | `0x01e1d114` | `0x01e1d114` | ✅ |
| `isVaultHealthy()` | `0xd5c9cfb0` | `0xd5c9cfb0` | ✅ |

## UserWithdrawManager (0x9F0491B32DBce587c50c4C43AB303b06478193A7)

| Function Signature | cast sig Result | Code Value | Status |
|---|---|---|---|
| `requestWithdraw(uint256,address)` | `0xccc143b8` | `0xccc143b8` | ✅ |
| `claim(uint256)` | `0x379607f5` | `0x379607f5` | ✅ |
| `userWithdrawRequests(uint256)` | `0x911f7acd` | `0x911f7acd` | ✅ |
| `nextRequestId()` | `0x6a84a985` | `0x6a84a985` | ✅ |
| `nextRequestIdToFinalize()` | `0xbbb84362` | `0xbbb84362` | ✅ |
| `getRequestIdsByUser(address)` | `0x7a99ab07` | `0x7a99ab07` | ✅ |
| `finalizeUserWithdrawalRequest()` | `0xad8a16dc` | `0xad8a16dc` | ✅ |

## ETHx Token (0xA35b1B31Ce002FBF2058D22F30f95D405200A15b)

| Function Signature | cast sig Result | Code Value | Status |
|---|---|---|---|
| `approve(address,uint256)` | `0x095ea7b3` | `0x095ea7b3` | ✅ |
| `balanceOf(address)` | `0x70a08231` | `0x70a08231` | ✅ |
| `allowance(address,address)` | `0xdd62ed3e` | `0xdd62ed3e` | ✅ |

All 20 selectors verified. No mismatches.
