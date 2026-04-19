# drift-protocol

Drift Protocol plugin for onchainos — perpetual futures DEX and lending on Solana.

> **Protocol Status (as of 2026-04-19):** Drift Protocol is in recovery mode following a $285M security incident on April 1, 2026. Trading, deposits, and withdrawals are paused while independent security audits (Ottersec + Asymmetric) complete. Track relaunch status at https://drift.trade

## Features

### Available now

- **get-balance** — Check your Solana wallet SOL, USDC, and USDT balances via onchainos
- **get-markets** — Fetch L2 orderbook for any Drift perpetual (returns graceful error while API is down)
- **get-funding-rates** — Fetch funding rates for perpetual markets (returns graceful error while API is down)

### Pending protocol relaunch

- **place-order** — Place market or limit orders on Drift perps (stub — returns pause message)
- **deposit** — Deposit USDT/SOL into Drift vault (stub — returns pause message)
- **cancel-order** — Cancel an open order (stub — returns pause message)

Write operations are blocked by two separate constraints:
1. Protocol is in recovery mode — no trading activity is possible
2. Drift does not provide a public unsigned-transaction API compatible with the onchainos trust model

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Solana Mainnet | 501 |

## Installation

```bash
plugin-store install drift-protocol
```

## Usage

```bash
# Check wallet balances
drift-protocol get-balance

# Query SOL-PERP orderbook (returns paused error until relaunch)
drift-protocol get-markets --market SOL-PERP --depth 5

# Query funding rates (returns paused error until relaunch)
drift-protocol get-funding-rates --market SOL-PERP
```

## Key Constants

| Constant | Value |
|----------|-------|
| Drift Program ID | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` |
| USDT Mint (Solana) | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| USDC Mint (Solana, legacy) | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| onchainos chain ID | `501` |
