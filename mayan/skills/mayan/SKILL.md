---
name: mayan
version: 0.1.0
description: "Mayan cross-chain swap. Bridge and swap tokens between Solana (501), Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137), BSC (56), and Avalanche (43114) using Swift, MCTP, and Wormhole routes."
author: skylavis-sky
chains: [1, 10, 56, 137, 501, 8453, 42161, 43114]
---

# Mayan Cross-Chain Swap Plugin

## Overview

Mayan cross-chain swap. Move tokens between Solana, Ethereum, Arbitrum, Base,
Optimism, Polygon, BSC, and Avalanche using the Swift (fastest ~15s), MCTP
(stablecoin optimized), and Wormhole routes.

## Supported chains

| Chain     | onchainos chain ID |
|-----------|--------------------|
| Solana    | 501                |
| Ethereum  | 1                  |
| Arbitrum  | 42161              |
| Base      | 8453               |
| Optimism  | 10                 |
| Polygon   | 137                |
| BSC       | 56                 |
| Avalanche | 43114              |

## Native token addresses

- Native SOL: `11111111111111111111111111111111`
- Wrapped SOL: `So11111111111111111111111111111111111111112`
- Native ETH (all EVM chains): `0x0000000000000000000000000000000000000000`

## Common token addresses

| Token     | Chain    | Address                                      |
|-----------|----------|----------------------------------------------|
| USDC      | Solana   | EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v |
| USDT      | Solana   | Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB  |
| USDC      | Base     | 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913   |
| USDC      | Ethereum | 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48   |
| USDC      | Arbitrum | 0xaf88d065e77c8cC2239327C5EDb3A432268e5831   |
| WETH      | Ethereum | 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2   |
| WETH      | Arbitrum | 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1   |

## Commands

### get-quote — Fetch cross-chain swap quote

```
mayan get-quote \
  --from-chain <id> \
  --to-chain <id> \
  --from-token <address> \
  --to-token <address> \
  --amount <float> \
  [--slippage <bps>]
```

Returns all available routes (SWIFT, MCTP, WH) with expected output, fees, and ETA.
Does not execute any transaction.

Examples:
```bash
# Quote 100 USDC from Solana to Base
mayan get-quote \
  --from-chain 501 \
  --to-chain 8453 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 100

# Quote 0.01 ETH from Arbitrum to Solana SOL
mayan get-quote \
  --from-chain 42161 \
  --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token So11111111111111111111111111111111111111112 \
  --amount 0.01
```

---

### swap — Execute cross-chain swap

```
mayan swap \
  --from-chain <id> \
  --to-chain <id> \
  --from-token <address> \
  --to-token <address> \
  --amount <float> \
  [--slippage <bps>] \
  [--dry-run]
```

Full execution flow:
1. Resolve wallet addresses from onchainos
2. Fetch best route quote (prefers SWIFT > MCTP > WH)
3. Build transaction via Mayan API
4. For EVM ERC-20: approve Mayan Forwarder, wait 3s, then swap
5. For EVM native ETH: submit swap with --amt value
6. For Solana: convert the serialized tx (b64 encoding) to base58, broadcast via --unsigned-tx
7. Print source tx hash and status check command

Use --dry-run to test the flow without broadcasting transactions.

Examples:
```bash
# Bridge 100 USDC from Solana to Base (MCTP route)
mayan swap \
  --from-chain 501 \
  --to-chain 8453 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 100

# Swap 0.01 ETH from Arbitrum to Solana SOL (Swift route)
mayan swap \
  --from-chain 42161 \
  --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token So11111111111111111111111111111111111111112 \
  --amount 0.01

# Swap 50 USDC from Base to Solana USDC (dry run)
mayan swap \
  --from-chain 8453 \
  --to-chain 501 \
  --from-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 50 \
  --dry-run

# Swap 0.05 WETH from Ethereum to Base USDC (ERC-20, approves Forwarder first)
mayan swap \
  --from-chain 1 \
  --to-chain 8453 \
  --from-token 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 0.05
```

---

### get-status — Check swap status

```
mayan get-status --tx-hash <hash> [--chain <id>]
```

Polls the Mayan Explorer API for swap progress. Status values:
- INPROGRESS — swap in flight
- COMPLETED — tokens delivered to destination
- REFUNDED — swap failed, tokens returned to sender

Examples:
```bash
# Check EVM-sourced swap
mayan get-status \
  --tx-hash 0xabc123...def

# Check Solana-sourced swap
mayan get-status \
  --tx-hash 5VfydLe8...xKj2 \
  --chain 501
```

---

## Notes

- The plugin automatically selects the best route (SWIFT preferred for speed).
- ERC-20 swaps require an approve transaction sent to the Mayan Forwarder
  (0x337685fdaB40D39bd02028545a4FfA7D287cC3E2) before the swap. A 3-second
  delay is inserted between approve and swap to avoid nonce conflicts.
- Solana transactions returned by the API use b64 encoding and are converted
  to base58 before passing to onchainos --unsigned-tx.
- Do not use --output json with onchainos wallet balance --chain 501.
- Aptos is not supported.
