# deBridge DLN Bridge Plugin

Cross-chain token swaps via deBridge Decentralized Liquidity Network (DLN). Supports EVM chains (Ethereum, Arbitrum, Base, Optimism, BSC, Polygon) and Solana.

## Commands

- `get-quote` — Fetch a swap quote without building a transaction
- `bridge` — Execute a cross-chain swap (ERC-20 approve + DlnSource.createOrder)
- `get-status` — Check fill status of a submitted order
- `get-chains` — List all supported chains and their IDs

## Chains

Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), BSC (56), Polygon (137), Solana (501)

## Usage

```
debridge get-quote --src-chain-id 42161 --dst-chain-id 8453 --src-token <USDC_ARB> --dst-token <USDC_BASE> --amount 1000000
debridge bridge --src-chain-id 8453 --dst-chain-id 42161 --src-token <USDC_BASE> --dst-token <USDC_ARB> --amount 1000000
debridge get-status --order-id 0x...
debridge get-chains
```
