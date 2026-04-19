# uniswap-v3

Uniswap V3 plugin for the OKX onchainos plugin store. Swap tokens and manage concentrated liquidity positions on Uniswap V3 across Ethereum, Arbitrum, Base, Optimism, and Polygon.

## Commands

- `get-quote` — Get swap quote (read-only, no gas)
- `swap` — Swap tokens via SwapRouter02.exactInputSingle
- `get-pools` — List pools for a token pair
- `get-positions` — View LP positions for a wallet
- `add-liquidity` — Mint a new concentrated liquidity position
- `remove-liquidity` — Remove liquidity (decreaseLiquidity + collect + burn)

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Ethereum | 1 |
| Arbitrum | 42161 |
| Base | 8453 |
| Optimism | 10 |
| Polygon | 137 |

## License

MIT
