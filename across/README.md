# Across Protocol Bridge Plugin

Cross-chain token bridging via Across Protocol. Supports USDC, WETH, native ETH and other ERC-20 tokens across Ethereum, Arbitrum, Base, Optimism, and Polygon.

## Commands

- `get-quote` — Fetch a bridge quote (fees, output amount, estimated fill time)
- `bridge` — Execute a cross-chain bridge (ERC-20 approve + SpokePool.depositV3)
- `get-status` — Check fill status of a submitted deposit

## Chains

Ethereum (1), Optimism (10), Polygon (137), Base (8453), Arbitrum (42161)

## Usage

```
across get-quote --from-chain 8453 --to-chain 10 --token USDC --amount 1.0
across bridge --from-chain 8453 --to-chain 10 --token USDC --amount 1.0
across get-status --deposit-tx 0x...
```
