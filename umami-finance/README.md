# Umami Finance Plugin

Umami Finance GM Vault integration for onchainos. Deposit USDC, WETH, or WBTC into auto-compounding yield vaults on Arbitrum.

## Supported Vaults

- GM USDC ETH Vault (`gmUSDC-eth`) — USDC yield from ETH/XRP/DOGE/LTC GMX V2 markets
- GM USDC BTC Vault (`gmUSDC-btc`) — USDC yield from BTC GMX V2 markets
- GM WETH Vault (`gmWETH`) — WETH yield
- GM WBTC Vault (`gmWBTC`) — WBTC yield

## Chain

Arbitrum (chain ID: 42161)

## Commands

```bash
umami-finance list-vaults
umami-finance vault-info --vault gmUSDC-eth
umami-finance positions
umami-finance deposit --vault gmUSDC-eth --amount 10.0 --dry-run
umami-finance redeem --vault gmUSDC-eth --dry-run
```
