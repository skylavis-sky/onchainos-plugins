# Compound V2 Function Selector Verification

All selectors verified using Python `eth_utils.keccak` (keccak256 of function signature).

## Verification Method

```python
from eth_utils import keccak

def selector(sig):
    return '0x' + keccak(text=sig).hex()[:8]
```

## Results

| Function Signature | Selector | Used In |
|-------------------|---------|---------|
| `mint(uint256)` | `0xa0712d68` | supply ERC20 |
| `mint()` | `0x1249c58b` | supply ETH (payable) |
| `redeem(uint256)` | `0xdb006a75` | redeem |
| `redeemUnderlying(uint256)` | `0x852a12e3` | (available) |
| `borrow(uint256)` | `0xc5ebeaec` | borrow (dry-run) |
| `repayBorrow(uint256)` | `0x0e752702` | repay ERC20 (dry-run) |
| `repayBorrow()` | `0x4e4d9fea` | repay ETH (dry-run, payable) |
| `claimComp(address)` | `0xe9af0292` | claim-comp |
| `getAllMarkets()` | `0xb0772d0b` | (reference) |
| `supplyRatePerBlock()` | `0xae9d70b0` | markets |
| `borrowRatePerBlock()` | `0xf8f9da28` | markets |
| `exchangeRateCurrent()` | `0xbd6d894d` | markets, positions |
| `exchangeRateStored()` | `0x182df0f5` | (reference) |
| `balanceOf(address)` | `0x70a08231` | positions, supply, redeem |
| `borrowBalanceCurrent(address)` | `0x17bfdfbc` | positions |
| `approve(address,uint256)` | `0x095ea7b3` | supply ERC20, repay ERC20 |
| `decimals()` | `0x313ce567` | (reference) |
| `underlying()` | `0x6f307dc3` | (reference) |
| `getAccountLiquidity(address)` | `0x5ec88c79` | (reference) |

## Notes

- `cast sig` was not available in this environment; selectors computed via Python eth_utils
- All selectors match the Compound V2 ABI as documented at https://docs.compound.finance/v2/
- `mint()` for cETH is payable (no amount parameter; ETH sent as transaction value)
- `repayBorrow()` for cETH is payable (ETH sent as transaction value)
- ERC20 `approve` selector `0x095ea7b3` is standard ERC-20
