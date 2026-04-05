# Selector Verification — Camelot V3

All selectors verified with `cast sig` (Foundry).

| Function | Canonical ABI Signature | Expected Selector | cast sig Result | Status |
|----------|------------------------|-------------------|-----------------|--------|
| exactInputSingle | `exactInputSingle((address,address,address,uint256,uint256,uint256,uint160))` | `0xbc651188` | `0xbc651188` | ✅ |
| quoteExactInputSingle | `quoteExactInputSingle(address,address,uint256,uint160)` | `0x2d9ebd1d` | `0x2d9ebd1d` | ✅ |
| poolByPair | `poolByPair(address,address)` | `0xd9a641e1` | `0xd9a641e1` | ✅ |
| positions | `positions(uint256)` | `0x99fbab88` | `0x99fbab88` | ✅ |
| mint | `mint((address,address,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` | `0xa232240b` | `0xa232240b` | ✅ |
| increaseLiquidity | `increaseLiquidity((uint256,uint256,uint256,uint256,uint256,uint256))` | `0x219f5d17` | `0x219f5d17` | ✅ |
| decreaseLiquidity | `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` | `0x0c49ccbe` | `0x0c49ccbe` | ✅ |
| collect | `collect((uint256,address,uint128,uint128))` | `0xfc6f7865` | `0xfc6f7865` | ✅ |
| burn | `burn(uint256)` | `0x42966c68` | `0x42966c68` | ✅ |
| approve | `approve(address,uint256)` | `0x095ea7b3` | `0x095ea7b3` | ✅ |
| allowance | `allowance(address,address)` | `0xdd62ed3e` | `0xdd62ed3e` | ✅ |
| balanceOf | `balanceOf(address)` | `0x70a08231` | `0x70a08231` | ✅ |
| tokenOfOwnerByIndex | `tokenOfOwnerByIndex(address,uint256)` | `0x2f745c59` | `0x2f745c59` | ✅ |

All 13 selectors verified ✅
