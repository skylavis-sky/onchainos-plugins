# Compound V2 Plugin Design

## Overview

Compound V2 is a classic cToken-based lending protocol on Ethereum mainnet. Users supply assets to receive cTokens (representing their share plus accrued interest), and can borrow other assets against their collateral. The protocol issues COMP governance rewards.

## Architecture

```
User
 ├── markets     → Comptroller.getAllMarkets() + cToken rates (read)
 ├── positions   → cToken.balanceOf + borrowBalanceCurrent per market (read)
 ├── supply      → ERC20.approve + cToken.mint(amount) | cETH.mint() payable (write)
 ├── redeem      → cToken.redeem(cTokenAmount) (write)
 ├── borrow      → cToken.borrow(amount) (dry-run only)
 ├── repay       → ERC20.approve + cToken.repayBorrow(amount) | cETH.repayBorrow() payable (dry-run only)
 └── claim-comp  → Comptroller.claimComp(address) (write)
```

## Key Contracts (Ethereum Mainnet)

| Contract | Address |
|----------|---------|
| Comptroller (Unitroller) | `0x3d9819210A31b4961b30EF54bE2aeD79B9c9Cd3b` |
| cETH | `0x4Ddc2D193948926D02f9B1fE9e1daa0718270ED5` |
| cUSDT | `0xf650C3d88D12dB855b8bf7D11Be6C55A4e07dCC9` |
| cUSDC | `0x39AA39c021dfbaE8faC545936693aC917d5E7563` |
| cDAI | `0x5d3a536E4D6DbD6114cc1Ead35777bAB948E3643` |
| COMP Token | `0xc00e94Cb662C3520282E6f5717214004A7f26888` |

## Supported Assets (by symbol)

- ETH → cETH (special: payable mint, no ERC20 approve)
- USDT → cUSDT (ERC20 flow)
- USDC → cUSDC (ERC20 flow)
- DAI → cDAI (ERC20 flow)

## Function Selectors (verified via keccak256)

| Function | Selector |
|----------|---------|
| `mint(uint256)` | `0xa0712d68` |
| `mint()` payable | `0x1249c58b` |
| `redeem(uint256)` | `0xdb006a75` |
| `redeemUnderlying(uint256)` | `0x852a12e3` |
| `borrow(uint256)` | `0xc5ebeaec` |
| `repayBorrow(uint256)` | `0x0e752702` |
| `repayBorrow()` payable (ETH) | `0x4e4d9fea` |
| `claimComp(address)` | `0xe9af0292` |
| `getAllMarkets()` | `0xb0772d0b` |
| `supplyRatePerBlock()` | `0xae9d70b0` |
| `borrowRatePerBlock()` | `0xf8f9da28` |
| `exchangeRateCurrent()` | `0xbd6d894d` |
| `balanceOf(address)` | `0x70a08231` |
| `borrowBalanceCurrent(address)` | `0x17bfdfbc` |
| `approve(address,uint256)` | `0x095ea7b3` |
| `underlying()` | `0x6f307dc3` |
| `decimals()` | `0x313ce567` |
| `getAccountLiquidity(address)` | `0x5ec88c79` |

## Rate Calculation

- `supplyRatePerBlock()` returns rate per Ethereum block (scaled by 1e18)
- Blocks per year ≈ 2,102,400 (avg 15 seconds/block)
- APY = ((rate_per_block * blocks_per_year / 1e18) + 1)^1 - 1  (simplified to linear APR for display)
- APR% = rate_per_block * blocks_per_year / 1e18 * 100

## cToken Exchange Rate

- `exchangeRateCurrent()` returns underlying per cToken scaled by 1e18
- Underlying balance = cToken balance * exchangeRate / 1e18
- For cETH: 18 decimals underlying, 8 decimals cToken
- For cUSDT/cUSDC: 6 decimals underlying, 8 decimals cToken
- For cDAI: 18 decimals underlying, 8 decimals cToken

## Operations Detail

### markets
- Call `Comptroller.getAllMarkets()` → array of cToken addresses
- For each: call `supplyRatePerBlock()`, `borrowRatePerBlock()`, `exchangeRateCurrent()`
- Filter to known markets (cETH, cUSDT, cUSDC, cDAI)
- Display APR, exchange rate

### positions
- For each known market, call:
  - `cToken.balanceOf(wallet)` → cToken balance
  - `cToken.borrowBalanceCurrent(wallet)` → borrow debt
  - `cToken.exchangeRateCurrent()` → to compute underlying supplied
- Show supplied and borrowed per market

### supply
- ETH path: `cETH.mint()` payable with ETH value = amount in wei
- ERC20 path:
  1. `ERC20.approve(cToken, amount)` → tx1
  2. Wait 3s for nonce safety
  3. `cToken.mint(amount)` → tx2
- Ask user to confirm before executing

### redeem
- `cToken.redeem(cTokenAmount)` → burns cTokens, receives underlying
- Ask user to confirm

### borrow (dry-run only)
- `cToken.borrow(amount)` — requires sufficient collateral
- Only executes in dry-run mode for safety

### repay (dry-run only)
- ETH path: `cETH.repayBorrow()` payable with ETH
- ERC20 path: approve + `cToken.repayBorrow(amount)`
- Only executes in dry-run mode for safety

### claim-comp
- `Comptroller.claimComp(wallet)` — claims all accrued COMP

## RPC Endpoint
- Primary: `https://ethereum.publicnode.com`

## Constraints
- Chain: Ethereum mainnet (1) only
- borrow and repay: dry-run only (safety)
- Max test tx: 0.01 USDT or 0.00005 ETH
- Reserve ≥ 0.001 ETH for gas
