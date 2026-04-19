# Jupiter Plugin — L0 Routing Test Cases

## Positive Cases (should route correctly)

| ID | Input | Expected Command | Reasoning |
|----|-------|-----------------|-----------|
| R1 | "swap SOL for USDC on Jupiter" | `swap --input-mint SOL --output-mint USDC --amount <N>` | Primary trigger phrase |
| R2 | "get a Jupiter swap quote for 0.1 SOL to USDC" | `get-quote --input-mint SOL --output-mint USDC --amount 0.1` | Quote trigger phrase |
| R3 | "what is the price of SOL on Jupiter" | `get-price --token SOL` | Price inquiry trigger |
| R4 | "search for JUP token on Solana" | `get-tokens --search JUP` | Token search trigger |
| R5 | "jup swap 0.5 SOL for USDT" | `swap --input-mint SOL --output-mint USDT --amount 0.5` | Alias trigger "jup swap" |
| R6 | "在Jupiter上兑换代币" (CN) | `swap` | Chinese trigger phrase |
| R7 | "查询Jupiter报价" (CN) | `get-quote` | Chinese quote trigger |
| R8 | "swap with 1% slippage" | `swap ... --slippage-bps 100` | Slippage parameter mapping |
| R9 | "get-tokens --limit 5" | `get-tokens --limit 5` | Limit argument pass-through |
| R10 | "swap using raw mint So111... for USDC" | `swap --input-mint So11111111111111111111111111111111111111112 --output-mint USDC --amount <N>` | Raw mint address resolution |

## Negative Cases (should NOT route to Jupiter)

| ID | Input | Should Route To | Reasoning |
|----|-------|----------------|-----------|
| N1 | "swap ETH for USDC on Uniswap" | uniswap-v3 | EVM chain swap — SKILL.md explicitly says do NOT use for EVM |
| N2 | "swap BNB for CAKE on PancakeSwap" | pancakeswap | BSC chain — Jupiter is Solana only |
| N3 | "bridge SOL to ETH" | mayan or cross-chain bridge | Cross-chain bridge operation |
| N4 | "create a new Solana token" | pump-fun or other Solana launch | Not a DEX aggregator operation |
| N5 | "buy token on Raydium" | raydium-plugin | Explicit Raydium routing request |
| N6 | "stake SOL for yield" | lido/staking plugin | Staking, not swapping |

## Argument Resolution Tests

| ID | Symbol Input | Expected Mint Resolution |
|----|-------------|--------------------------|
| A1 | `SOL` | `So11111111111111111111111111111111111111112` |
| A2 | `USDC` | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| A3 | `USDT` | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| A4 | `JUP` | `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN` |
| A5 | Raw address (any 44-char base58) | Passed through unchanged |

## Safety / Confirmation Gate Tests

| ID | Scenario | Expected Behavior |
|----|----------|-------------------|
| S1 | `swap` without `--dry-run` | Should prompt user confirmation before executing |
| S2 | `swap --dry-run` | Returns JSON with `dry_run: true`, no onchainos call |
| S3 | `get-quote` | No confirmation needed — read-only |
| S4 | `get-price` | No confirmation needed — read-only |
| S5 | `get-tokens` | No confirmation needed — read-only |
