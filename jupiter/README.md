# Jupiter Plugin

Solana DEX aggregator plugin for onchainos. Routes token swaps across all major Solana DEXes (Raydium, Orca, Meteora, etc.) via Jupiter's multi-router aggregation to find the best price for any SPL token pair.

## Commands

- `get-quote` — Get a swap quote (output amount, price impact, route plan)
- `swap` — Execute a token swap on Jupiter via onchainos
- `get-price` — Get real-time USD price for a token
- `get-tokens` — Search for SPL tokens by symbol or name

## Usage

```bash
# Get a quote for 0.1 SOL -> USDC
jupiter get-quote --input-mint SOL --output-mint USDC --amount 0.1

# Swap 0.1 SOL for USDC (dry run)
jupiter swap --input-mint SOL --output-mint USDC --amount 0.1 --dry-run

# Swap 0.1 SOL for USDC (live)
jupiter swap --input-mint SOL --output-mint USDC --amount 0.1

# Get SOL price in USDC
jupiter get-price --token SOL

# Search for tokens
jupiter get-tokens --search JUP --limit 5
```

## Chain

Solana mainnet (chain ID 501)

## Program ID

`JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`
