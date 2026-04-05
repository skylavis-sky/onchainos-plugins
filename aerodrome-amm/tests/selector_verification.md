# Function Selector Verification — aerodrome-amm

All selectors computed via `keccak256(signature).hex()[:8]` using Python `eth_hash`:

## Router (0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43)

| Selector | Function Signature | Used In |
|----------|-------------------|---------|
| `0xcac88ea9` | `swapExactTokensForTokens(uint256,uint256,(address,address,bool,address)[],address,uint256)` | swap.rs |
| `0x5a47ddc3` | `addLiquidity(address,address,bool,uint256,uint256,uint256,uint256,address,uint256)` | add_liquidity.rs |
| `0x0dede6c4` | `removeLiquidity(address,address,bool,uint256,uint256,uint256,address,uint256)` | remove_liquidity.rs |
| `0x5509a1ac` | `getAmountsOut(uint256,(address,address,bool,address)[])` | rpc.rs |
| `0xce700c29` | `quoteAddLiquidity(address,address,bool,address,uint256,uint256)` | rpc.rs |
| `0xc92de3ec` | `quoteRemoveLiquidity(address,address,bool,address,uint256)` | (available) |
| `0x903638a4` | `swapExactETHForTokens(uint256,(address,address,bool,address)[],address,uint256)` | (available) |
| `0xc6b7f1b6` | `swapExactTokensForETH(uint256,uint256,(address,address,bool,address)[],address,uint256)` | (available) |
| `0xb7e0d4c0` | `addLiquidityETH(address,bool,uint256,uint256,uint256,address,uint256)` | (available) |
| `0xd7b0e0a5` | `removeLiquidityETH(address,bool,uint256,uint256,uint256,address,uint256)` | (available) |
| `0x874029d9` | `poolFor(address,address,bool,address)` | (available) |

## PoolFactory (0x420DD381b31aEf6683db6B902084cB0FFECe40Da)

| Selector | Function Signature | Used In |
|----------|-------------------|---------|
| `0x79bc57d5` | `getPool(address,address,bool)` | rpc.rs |
| `0x41d1de97` | `allPools(uint256)` | rpc.rs |
| `0xefde4e64` | `allPoolsLength()` | rpc.rs |

## Pool (ERC-20 LP Token)

| Selector | Function Signature | Used In |
|----------|-------------------|---------|
| `0x0dfe1681` | `token0()` | rpc.rs |
| `0xd21220a7` | `token1()` | rpc.rs |
| `0x22be3de1` | `stable()` | (available) |
| `0x0902f1ac` | `getReserves()` | rpc.rs |
| `0x18160ddd` | `totalSupply()` | rpc.rs |
| `0x70a08231` | `balanceOf(address)` | rpc.rs |
| `0xdd62ed3e` | `allowance(address,address)` | rpc.rs |
| `0x095ea7b3` | `approve(address,uint256)` | config.rs |

## Voter (0x16613524E02ad97eDfeF371bC883F2F5d6C480A5)

| Selector | Function Signature | Used In |
|----------|-------------------|---------|
| `0xb9a09fd5` | `gauges(address)` | rpc.rs |

## Gauge (address via Voter.gauges)

| Selector | Function Signature | Used In |
|----------|-------------------|---------|
| `0xc00007b0` | `getReward(address)` | claim_rewards.rs |
| `0x008cc262` | `earned(address)` | rpc.rs |
| `0xb6b55f25` | `deposit(uint256)` | (available) |
| `0x2e1a7d4d` | `withdraw(uint256)` | (available) |

## Verification Method

```python
from eth_hash.auto import keccak
def selector(sig):
    return '0x' + keccak(sig.encode()).hex()[:8]
```

All selectors verified against deployed contracts on Base mainnet.
