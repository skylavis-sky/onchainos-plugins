---
name: jupiter
description: >-
  Jupiter DEX aggregator plugin for Solana. Swap any SPL token at the best price via
  multi-router aggregation (Raydium, Orca, Meteora, and more). Supports swap quotes,
  live token prices, token search, and on-chain swaps via onchainos.
  Trigger phrases: swap on jupiter, jupiter swap, jup swap, swap sol for usdc,
  get jupiter quote, jupiter price, search solana tokens.
  Chinese triggers: 在Jupiter上兑换代币, 用Jupiter兑换SOL, 查询Jupiter报价,
  查询Solana代币价格, 搜索Solana代币.
  Do NOT use for EVM swaps — use uniswap-v3 or 1inch instead.
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Architecture

- Read ops (`get-quote`, `get-price`, `get-tokens`) — direct REST API calls to Jupiter endpoints; no wallet or confirmation needed
- Write ops (`swap`) — after user confirmation, calls Jupiter API to get quote + unsigned tx, converts base64 to base58, then broadcasts via `onchainos wallet contract-call --chain 501 --unsigned-tx <base58_tx> --force`
- Chain: Solana mainnet (chain ID 501)
- Program ID: `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4`
- APIs: Jupiter Swap API v2, Price API v3, Tokens API

## Do NOT use for

- EVM chain swaps — use `uniswap-v3` or `1inch` instead
- Non-Solana chains — Jupiter is Solana only

## Commands

### get-quote — Get swap quote

Returns expected output amount, price impact, and routing plan. No on-chain action.

```bash
# Quote 0.1 SOL -> USDC
jupiter get-quote \
  --input-mint SOL \
  --output-mint USDC \
  --amount 0.1 \
  --slippage-bps 50

# Quote using raw mint addresses
jupiter get-quote \
  --input-mint So11111111111111111111111111111111111111112 \
  --output-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.1
```

**Args:**
- `--input-mint` — token symbol (SOL, USDC, USDT) or raw Solana mint address
- `--output-mint` — token symbol or raw mint address
- `--amount` — input amount in UI units (e.g. `0.1` for 0.1 SOL)
- `--slippage-bps` — slippage tolerance in basis points (default: 50 = 0.5%)

---

### swap — Execute token swap

**Ask user to confirm before executing.** This is an on-chain write operation that moves funds.

Execution flow:
1. Run with `--dry-run` first to preview (no on-chain action)
2. **Ask user to confirm** the swap details, output estimate, and price impact
3. Execute only after explicit user approval

```bash
# Preview (dry run — no funds moved)
jupiter swap \
  --input-mint SOL \
  --output-mint USDC \
  --amount 0.1 \
  --dry-run

# Execute (after user confirmation — broadcasts on-chain)
jupiter swap \
  --input-mint SOL \
  --output-mint USDC \
  --amount 0.1 \
  --slippage-bps 50
```

**Args:**
- `--input-mint` — token symbol or raw mint address
- `--output-mint` — token symbol or raw mint address
- `--amount` — input amount in UI units
- `--slippage-bps` — slippage tolerance in bps (default: 50)
- `--dry-run` — simulate without broadcasting (early return, no onchainos call)

**Safety notes:**
- Always preview with `--dry-run` before executing a live swap
- Solana blockhash expires in ~60 seconds; swap is broadcast immediately after the API call
- The plugin converts the base64 transaction from Jupiter API to base58 before passing to onchainos

---

### get-price — Get token USD price

Returns the real-time USD price and 24h change for a token.

```bash
# Get SOL price in USDC
jupiter get-price --token SOL

# Get JUP price in USDC
jupiter get-price --token JUP --vs-token USDC

# Using raw mint address
jupiter get-price \
  --token So11111111111111111111111111111111111111112
```

**Args:**
- `--token` — token symbol (SOL, USDC, JUP) or raw mint address
- `--vs-token` — denominator token (default: USDC)

---

### get-tokens — Search tokens

Search for SPL tokens by symbol or name, or list verified tokens.

```bash
# Search for JUP token
jupiter get-tokens --search JUP --limit 5

# List verified tokens (default 20)
jupiter get-tokens

# Search by token name
jupiter get-tokens --search "Jupiter" --limit 10
```

**Args:**
- `--search` — search query: token symbol, name, or mint address (optional)
- `--limit` — maximum number of results to return (default: 20)

---

## Common Token Mint Addresses (Solana Mainnet)

| Token | Mint Address |
|-------|-------------|
| SOL (Wrapped) | `So11111111111111111111111111111111111111112` |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| JUP | `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN` |

## Notes

- Token symbol shorthand: SOL, USDC, USDT, JUP are resolved automatically; any other input is treated as a raw mint address
- `onchainos wallet balance --chain 501` — do NOT add `--output json` (Solana returns JSON natively; adding the flag causes EOF failure)
- Rate limit: Jupiter keyless tier is 0.5 RPS; for higher throughput register at the Jupiter developer portal (developers.jup.ag) for a free API key
