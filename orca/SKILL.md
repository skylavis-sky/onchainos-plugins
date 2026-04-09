---
name: orca
description: "Concentrated liquidity AMM on Solana — swap tokens and query pools via the Whirlpools CLMM program. Trigger phrases: swap on orca, orca swap, swap tokens on solana, orca pools, get swap quote, whirlpool swap, orca dex. Chinese: Orca兑换, 在Orca上交换代币, 查询Orca流动性池, 获取兑换报价"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Architecture

- Read ops (`get-pools`, `get-quote`) → direct Orca REST API calls (`https://api.orca.so/v1`); no wallet needed, no confirmation required
- Write ops (`swap`) → after user confirmation, submits via `onchainos dex swap execute --chain 501`
- Chain: Solana mainnet (chain ID 501)
- Program: Orca Whirlpools (`whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`)

## Commands

### get-pools — Query Whirlpool Pools

List all Orca Whirlpool pools for a token pair, sorted by TVL.

```bash
orca get-pools \
  --token-a <MINT_A> \
  --token-b <MINT_B> \
  [--min-tvl <USD>] \
  [--include-low-liquidity]
```

**Parameters:**
- `--token-a`: First token mint address (use `11111111111111111111111111111111` for native SOL)
- `--token-b`: Second token mint address
- `--min-tvl`: Minimum pool TVL in USD (default: 10000)
- `--include-low-liquidity`: Include pools below min-tvl threshold

**Example:**
```bash
# Find SOL/USDC pools
orca get-pools \
  --token-a So11111111111111111111111111111111111111112 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```

**Output fields:** `address`, `token_a_symbol`, `token_b_symbol`, `fee_rate_pct`, `price`, `tvl_usd`, `volume_24h_usd`, `fee_apr_24h_pct`, `total_apr_24h_pct`

---

### get-quote — Get Swap Quote

Calculate an estimated swap output for a given input amount on Orca.

```bash
orca get-quote \
  --from-token <MINT> \
  --to-token <MINT> \
  --amount <AMOUNT> \
  [--slippage-bps <BPS>] \
  [--pool <POOL_ADDRESS>]
```

**Parameters:**
- `--from-token`: Input token mint address
- `--to-token`: Output token mint address
- `--amount`: Input amount in human-readable units (e.g. `0.5` for 0.5 SOL)
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%)
- `--pool`: Specific pool address (optional; uses highest-TVL pool if omitted)

**Example:**
```bash
# Quote: how much USDC for 0.5 SOL?
orca get-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 50
```

**Output fields:** `estimated_amount_out`, `minimum_amount_out`, `slippage_bps`, `fee_rate_pct`, `price`, `pool_address`, `pool_tvl_usd`, `estimated_price_impact_pct`

---

### swap — Execute Token Swap

Execute a token swap on Orca via `onchainos dex swap execute`.

**Pre-swap safety checks:**
1. Security scan of output token via `onchainos security token-scan`
2. Price impact check: warns at >2%, blocks at >10%
3. **Ask user to confirm** before executing on-chain

```bash
orca swap \
  --from-token <MINT> \
  --to-token <MINT> \
  --amount <AMOUNT> \
  [--slippage-bps <BPS>] \
  [--dry-run] \
  [--skip-security-check]
```

**Parameters:**
- `--from-token`: Input token mint address
- `--to-token`: Output token mint address
- `--amount`: Amount in human-readable units
- `--slippage-bps`: Slippage tolerance in basis points (default: 50 = 0.5%)
- `--dry-run`: Simulate only; do not broadcast transaction
- `--skip-security-check`: Bypass token security scan (not recommended)

**Execution Flow:**
1. Run with `--dry-run` first to preview
2. **Ask user to confirm** the swap details (amount, tokens, slippage) before proceeding
3. Execute only after explicit user approval
4. Report transaction hash and Solscan link

**Example:**
```bash
# Step 1: Preview
orca --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5

# Step 2: After user confirms, execute for real
orca swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5 \
  --slippage-bps 100
```

**Output fields:** `ok`, `tx_hash`, `solscan_url`, `from_token`, `to_token`, `amount`, `slippage_bps`, `estimated_price_impact_pct`

---

## Known Token Addresses (Solana Mainnet)

| Token | Mint Address |
|-------|-------------|
| Native SOL | `11111111111111111111111111111111` |
| Wrapped SOL (wSOL) | `So11111111111111111111111111111111111111112` |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| ORCA | `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` |

## Safety Rules

- Never swap into a token flagged as `block` by security scan
- Swaps with estimated price impact > 10% are automatically rejected
- Always run `--dry-run` first and show the quote to the user before asking for confirmation
- If pool TVL < $10,000, warn user about high slippage risk
