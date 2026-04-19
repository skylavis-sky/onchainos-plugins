---
name: uniswap-v3
version: "0.1.0"
description: >-
  Swap tokens and manage concentrated liquidity positions on Uniswap V3 - the
  leading CLMM DEX across Ethereum, Arbitrum, Base, Optimism, and Polygon.
  Do NOT use for: BSC swaps (use pancakeswap), Uniswap V2 pools, or Solana
  token swaps.
---

# Uniswap V3 Skill

Swap tokens and manage concentrated liquidity (CLMM) positions on Uniswap V3 across Ethereum, Arbitrum, Base, Optimism, and Polygon.

**Trigger phrases:** "uniswap v3", "swap on uniswap", "uniswap concentrated liquidity", "univ3", "add liquidity uniswap", "remove liquidity uniswap", "uniswap v3 pool", "uniswap position", "uni v3"

---

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Ethereum | 1 |
| Arbitrum | 42161 |
| Base | 8453 |
| Optimism | 10 |
| Polygon | 137 |

---

## Commands

### `get-quote` — Get swap quote (read-only)

Get the expected output amount for a token swap without submitting any transaction.

**Trigger phrases:** "get quote uniswap", "how much will I get on uniswap", "uniswap price", "uniswap quote"

```
uniswap-v3 get-quote \
  --token-in <address_or_symbol> \
  --token-out <address_or_symbol> \
  --amount <human_amount> \
  [--chain 1|10|137|8453|42161] \
  [--fee 100|500|3000|10000]
```

**Examples:**
```
# Quote 100 USDC -> WETH on Ethereum
uniswap-v3 get-quote --token-in USDC --token-out WETH --amount 100 --chain 1

# Quote 1 WETH -> USDC on Base
uniswap-v3 get-quote --token-in WETH --token-out USDC --amount 1 --chain 8453

# Quote with specific fee tier
uniswap-v3 get-quote --token-in USDC --token-out WETH --amount 500 --chain 1 --fee 500
```

Queries QuoterV2 via `eth_call` (no transaction, no gas). Iterates all fee tiers (0.01%, 0.05%, 0.3%, 1%) and returns the best output. Read-only — no confirmation required.

---

### `swap` — Swap tokens via SwapRouter02

Swap an exact input amount of one token for the maximum available output via Uniswap V3 SwapRouter02.

**Trigger phrases:** "swap on uniswap", "trade on uniswap v3", "uniswap swap", "exchange tokens uniswap"

```
uniswap-v3 swap \
  --token-in <address_or_symbol> \
  --token-out <address_or_symbol> \
  --amount <human_amount> \
  [--slippage-bps 50] \
  [--chain 1|10|137|8453|42161] \
  [--fee 100|500|3000|10000] \
  [--dry-run]
```

**Execution flow:**

1. Fetch token metadata (decimals, symbol) via `eth_call`.
2. Get best quote across all fee tiers via QuoterV2 `eth_call`. Validates pool exists via Factory.getPool before each quote.
3. Compute `amountOutMinimum` using slippage tolerance.
4. Present the full swap plan and **ask user to confirm** before proceeding.
5. Check IERC-20 allowance. If insufficient, submit Step 1 — ERC-20 approve via `onchainos wallet contract-call --force` (tokenIn -> SwapRouter02). Wait for confirmation via receipt polling.
6. Submit Step 2 — `exactInputSingle` via `onchainos wallet contract-call --force` to SwapRouter02.
7. Report transaction hash and explorer link.

**Flags:**
- `--slippage-bps` — tolerance in basis points (default: 50 = 0.5%)
- `--chain` — chain ID (default: 1 = Ethereum)
- `--fee` — override fee tier; if omitted, auto-selects best
- `--dry-run` — print calldata without submitting (no wallet required)

**Notes:**
- SwapRouter02 `exactInputSingle` uses 7 struct fields (NO deadline field — different from SwapRouter v1).
- Selector: `0x04e45aaf`. Do NOT use SwapRouter v1 selector `0x414bf389`.
- Approval goes to SwapRouter02 address (not NFPM).
- `--force` is required on all write calls — without it, the tx never broadcasts.

---

### `get-pools` — List pools for a token pair

Query UniswapV3Factory for all pools across all fee tiers for a given token pair.

**Trigger phrases:** "uniswap v3 pool", "show uniswap pools", "find uniswap pool", "uniswap pool info"

```
uniswap-v3 get-pools \
  --token-a <address_or_symbol> \
  --token-b <address_or_symbol> \
  [--chain 1|10|137|8453|42161]
```

**Example:**
```
uniswap-v3 get-pools --token-a USDC --token-b WETH --chain 1
```

Returns pool addresses, current liquidity, and sqrtPriceX96 for each fee tier. Read-only — no transactions or confirmation required.

---

### `get-positions` — View LP positions

View Uniswap V3 LP positions for a wallet address.

**Trigger phrases:** "my uniswap positions", "show uniswap v3 LP", "view uniswap liquidity", "uniswap position details"

```
uniswap-v3 get-positions \
  [--owner <wallet_address>] \
  [--token-id <nft_id>] \
  [--chain 1|10|137|8453|42161]
```

**Examples:**
```
# List all positions for your connected wallet on Ethereum
uniswap-v3 get-positions --chain 1

# List all positions for a specific wallet
uniswap-v3 get-positions --owner 0xYourWallet --chain 1

# View a specific position by token ID
uniswap-v3 get-positions --token-id 12345 --chain 42161
```

Queries NonfungiblePositionManager on-chain via `eth_call`. Read-only — no confirmation required.

---

### `add-liquidity` — Add concentrated liquidity

Mint a new V3 LP position via NonfungiblePositionManager.mint.

**Trigger phrases:** "add liquidity uniswap v3", "provide liquidity uniswap", "mint uniswap position", "deposit to uniswap pool"

```
uniswap-v3 add-liquidity \
  --token-a <address_or_symbol> \
  --token-b <address_or_symbol> \
  --fee <100|500|3000|10000> \
  --amount-a <human_amount> \
  --amount-b <human_amount> \
  [--tick-lower <int>] \
  [--tick-upper <int>] \
  [--slippage-bps 50] \
  [--chain 1|10|137|8453|42161] \
  [--dry-run]
```

**Execution flow:**

1. Sort tokens so that token0 < token1 lexicographically (required by protocol).
2. Validate tick values are multiples of the fee tier's tickSpacing. Default to full-range if not specified.
3. Verify pool exists via Factory.getPool.
4. Present the full plan (amounts, tick range, NFPM address) and **ask user to confirm** before proceeding.
5. Check token0 allowance for NFPM. If insufficient, submit approve tx, wait for confirmation.
6. Check token1 allowance for NFPM. If insufficient, submit approve tx, wait for confirmation.
7. Submit `mint(MintParams)` via `onchainos wallet contract-call --force` to NFPM.
8. Report tokenId and transaction hash.

**tickSpacing by fee tier:**
| Fee | tickSpacing | Full-range ticks |
|-----|-------------|-----------------|
| 100 (0.01%) | 1 | -887272, 887272 |
| 500 (0.05%) | 10 | -887270, 887270 |
| 3000 (0.3%) | 60 | -887220, 887220 |
| 10000 (1.0%) | 200 | -887200, 887200 |

**Notes:**
- Ticks must be multiples of tickSpacing or mint will revert.
- If --tick-lower/--tick-upper are omitted, uses full-range for the fee tier.
- Approvals go to NonfungiblePositionManager (not SwapRouter02).
- `--force` is required on all write calls.

---

### `remove-liquidity` — Remove liquidity and collect tokens

Remove liquidity from an existing V3 position. Performs up to three steps: `decreaseLiquidity`, `collect`, and optionally `burn`.

**Trigger phrases:** "remove liquidity uniswap v3", "withdraw uniswap position", "close uniswap LP", "collect uniswap fees", "exit uniswap position"

```
uniswap-v3 remove-liquidity \
  --token-id <nft_id> \
  [--liquidity-pct 100] \
  [--chain 1|10|137|8453|42161] \
  [--dry-run]
```

**Examples:**
```
# Remove all liquidity from position #12345 on Arbitrum
uniswap-v3 remove-liquidity --token-id 12345 --chain 42161

# Remove 50% liquidity from position #12345 on Ethereum
uniswap-v3 remove-liquidity --token-id 12345 --liquidity-pct 50 --chain 1
```

**Execution flow:**

1. Fetch position data via `positions(tokenId)` to verify existence and get liquidity.
2. Verify ownership via `ownerOf(tokenId)`.
3. Present current position details and **ask user to confirm** before proceeding.
4. Submit Step 1 — `decreaseLiquidity` via `onchainos wallet contract-call --force`. Tokens become "owed" but are NOT transferred yet. Wait for confirmation.
5. Wait 5 seconds, then submit Step 2 — `collect` via `onchainos wallet contract-call --force`. Transfers owed tokens to your wallet. Wait for confirmation.
6. If full removal (100%): submit Step 3 — `burn` via `onchainos wallet contract-call --force` to destroy the NFT.
7. Report tokens received and transaction hashes.

**Important notes:**
- `decreaseLiquidity` alone does NOT transfer tokens. The `collect` step is always required.
- `burn` only executes when 100% liquidity is removed (position NFT has zero liquidity remaining).
- `--force` is required on all write calls.
- No token approve needed for remove-liquidity (NFPM already owns the position NFT).

---

## Contract Addresses

| Contract | Ethereum (1) | Arbitrum (42161) | Base (8453) | Optimism (10) | Polygon (137) |
|----------|-------------|-----------------|-------------|---------------|---------------|
| UniswapV3Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | `0x33128a8fC17869897dcE68Ed026d694621f6FDfD` | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | `0x1F98431c8aD98523631AE4a59f267346ea31F984` |
| SwapRouter02 | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | `0x2626664c2603336E57B271c5C0b26F421741e481` | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` |
| QuoterV2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | `0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a` | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` |
| NonfungiblePositionManager | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` |

**Note:** Base has DIFFERENT contract addresses compared to all other chains.

---

## Common Token Addresses

### Ethereum (1)
| Token | Address | Decimals |
|-------|---------|----------|
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | 18 |
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | 6 |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | 6 |
| DAI | `0x6B175474E89094C44Da98b954EedeAC495271d0F` | 18 |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` | 8 |
| UNI | `0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984` | 18 |

### Arbitrum (42161)
| Token | Address | Decimals |
|-------|---------|----------|
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | 18 |
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` | 6 |
| USDT | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` | 6 |
| ARB | `0x912CE59144191C1204E64559FE8253a0e49E6548` | 18 |

### Base (8453)
| Token | Address | Decimals |
|-------|---------|----------|
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | 6 |
| cbETH | `0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22` | 18 |

### Optimism (10)
| Token | Address | Decimals |
|-------|---------|----------|
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| USDC | `0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85` | 6 |
| OP | `0x4200000000000000000000000000000000000042` | 18 |

### Polygon (137)
| Token | Address | Decimals |
|-------|---------|----------|
| WMATIC | `0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270` | 18 |
| USDC | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` | 6 |
| WETH | `0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619` | 18 |
