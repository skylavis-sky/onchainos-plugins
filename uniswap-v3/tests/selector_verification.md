# Uniswap V3 — Selector Verification

All selectors verified using Ethereum Keccak-256 (NOT Python hashlib.sha3_256 which is NIST SHA3).
Verification method: `python3 -c "from eth_hash.auto import keccak; print(keccak(b'<sig>').hex()[:8])"`

## Selectors Used in This Plugin

| Selector | Canonical Signature | Contract | Status |
|----------|--------------------|-----------|----|
| `0x04e45aaf` | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))` | SwapRouter02 | VERIFIED |
| `0xc6a5026a` | `quoteExactInputSingle((address,address,uint256,uint24,uint160))` | QuoterV2 | VERIFIED |
| `0x1698ee82` | `getPool(address,address,uint24)` | UniswapV3Factory | VERIFIED |
| `0x99fbab88` | `positions(uint256)` | NonfungiblePositionManager | VERIFIED |
| `0x88316456` | `mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` | NonfungiblePositionManager | VERIFIED |
| `0x0c49ccbe` | `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` | NonfungiblePositionManager | VERIFIED |
| `0xfc6f7865` | `collect((uint256,address,uint128,uint128))` | NonfungiblePositionManager | VERIFIED |
| `0x42966c68` | `burn(uint256)` | NonfungiblePositionManager | VERIFIED |
| `0x095ea7b3` | `approve(address,uint256)` | ERC-20 | VERIFIED |
| `0xdd62ed3e` | `allowance(address,address)` | ERC-20 | VERIFIED |
| `0x70a08231` | `balanceOf(address)` | ERC-20 / NFPM | VERIFIED |
| `0x6352211e` | `ownerOf(uint256)` | ERC-721 (NFPM) | VERIFIED |
| `0x313ce567` | `decimals()` | ERC-20 | VERIFIED |
| `0x95d89b41` | `symbol()` | ERC-20 | VERIFIED |
| `0x2f745c59` | `tokenOfOwnerByIndex(address,uint256)` | ERC-721 (NFPM) | VERIFIED |
| `0x3850c7bd` | `slot0()` | UniswapV3Pool | VERIFIED |
| `0x1a686502` | `liquidity()` | UniswapV3Pool | VERIFIED |

## Key Distinction: SwapRouter02 vs SwapRouter v1

- SwapRouter02 (used here): `exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))` → `0x04e45aaf` (7 fields, NO deadline)
- SwapRouter v1 (legacy, NOT used): `exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))` → `0x414bf389` (8 fields, WITH deadline)

Mixing these will cause silent wrong-function dispatch. This plugin always uses `0x04e45aaf`.

## Notes on alloy-sol-types Encoding

This plugin uses alloy-sol-types for ABI encoding. The `sol!` macro generates correct selectors from
the canonical function signatures. The EVM encodes tuples as (offset, data), so each `exactInputSingle`
call includes a 32-byte offset `0x...20` before the struct data.

The solidity `sol!` macro encoding for SwapRouter02:
```
04e45aaf                                                          ← selector
0000000000000000000000000000000000000000000000000000000000000020  ← tuple offset
000000000000000000000000<tokenIn>                                 ← address tokenIn
000000000000000000000000<tokenOut>                                ← address tokenOut
<fee, uint24 padded>                                              ← uint24 fee
000000000000000000000000<recipient>                               ← address recipient
<amountIn, uint256>                                               ← uint256 amountIn
<amountOutMinimum, uint256>                                       ← uint256 amountOutMinimum
0000...0000                                                       ← uint160 sqrtPriceLimitX96 = 0
```
