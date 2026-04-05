# Mayan Cross-Chain Swap Plugin

Cross-chain token swaps via Mayan Finance. Supports SWIFT, MCTP, and Wormhole routes between Solana, Ethereum, Arbitrum, Base, Optimism, Polygon, BSC, and Avalanche.

## Commands

- `get-quote` — Fetch swap quote(s) across SWIFT/MCTP/WH routes
- `swap` — Execute cross-chain swap (ERC-20 approve + Mayan Forwarder on EVM; SWIFT/MCTP program on Solana)
- `get-status` — Check swap status on Mayan Explorer

## Chains

Solana (501), Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137), BSC (56), Avalanche (43114)

## Usage

```
mayan get-quote --from-chain 501 --to-chain 8453 --from-token <USDC_SOL> --to-token <USDC_BASE> --amount 1.0
mayan swap --from-chain 8453 --to-chain 501 --from-token 0x0000000000000000000000000000000000000000 --to-token <WSOL> --amount 0.001
mayan get-status --tx-hash 0x...
```
