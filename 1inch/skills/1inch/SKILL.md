---
name: 1inch
version: "0.1.0"
description: >-
  Swap tokens at the best rates across 200+ DEXs via the 1inch aggregation
  protocol -- supports Ethereum, Arbitrum, Base, BSC, and Polygon.
  Do NOT use for non-EVM chains; use jupiter for Solana swaps.
---

# 1inch Skill

Swap tokens at the best rates across 200+ DEXs using the 1inch aggregation protocol on Ethereum, Arbitrum, Base, BSC, and Polygon.

**Trigger phrases (English):** "1inch", "swap on 1inch", "1inch quote", "1inch aggregator", "best swap rate", "swap via 1inch", "oneinch swap", "check allowance 1inch", "approve token 1inch"

**Trigger phrases (Chinese):** "1inch 换币", "1inch 报价", "1inch 聚合", "1inch 最优价格", "用1inch兑换", "1inch 代币授权", "1inch 查询额度"

**Do NOT use for:** non-EVM chains (use jupiter for Solana swaps); Uniswap-specific features; direct AMM pool interactions.

---

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Ethereum | 1 |
| Arbitrum | 42161 |
| Base | 8453 |
| BSC | 56 |
| Polygon | 137 |

---

## Setup

Set your 1inch API key (obtain at https://portal.1inch.dev):

```bash
export ONEINCH_API_KEY=your_api_key_here
```

The plugin defaults to the `demo` key if unset (rate-limited, for testing only).

---

## Commands

### `get-quote` -- Get swap quote (read-only)

Get the expected output amount for a token swap without submitting any transaction.

**Trigger phrases (English):** "get 1inch quote", "how much will I get on 1inch", "1inch price", "1inch quote", "best rate on 1inch"

**Trigger phrases (Chinese):** "1inch 报价", "1inch 查询价格", "在1inch能换多少", "1inch 最优汇率"

```
1inch get-quote \
  --src <token_or_address> \
  --dst <token_or_address> \
  --amount <human_amount> \
  [--chain 1|42161|8453|56|137]
```

**Examples:**
```bash
# Quote 0.001 ETH -> USDC on Base
1inch get-quote --src ETH --dst USDC --amount 0.001 --chain 8453

# Quote 100 USDC -> ETH on Ethereum
1inch get-quote --src USDC --dst ETH --amount 100 --chain 1

# Quote 10 MATIC -> USDC on Polygon
1inch get-quote --src MATIC --dst USDC --amount 10 --chain 137
```

Read-only -- calls 1inch API. No transaction, no wallet required. No confirmation needed.

---

### `swap` -- Swap tokens via 1inch

Swap an exact input amount of one token for the maximum available output via 1inch AggregationRouterV6.

**Trigger phrases (English):** "swap on 1inch", "trade via 1inch", "1inch swap", "exchange tokens via 1inch"

**Trigger phrases (Chinese):** "用1inch换币", "1inch兑换", "1inch swap代币", "通过1inch交易"

```
1inch swap \
  --src <token_or_address> \
  --dst <token_or_address> \
  --amount <human_amount> \
  [--slippage-bps 50] \
  [--chain 1|42161|8453|56|137] \
  [--dry-run]
```

**Execution flow:**

1. Fetch expected output via 1inch /quote API for display.
2. If src is an ERC-20 (not native ETH/BNB/MATIC): check allowance via /approve/allowance.
3. If allowance insufficient: fetch approve calldata from /approve/transaction.
   Ask user to confirm before broadcasting the approve transaction via `onchainos wallet contract-call --force`.
   Wait for approval confirmation before proceeding.
4. Fetch swap calldata from 1inch /swap API.
   Ask user to confirm before broadcasting the swap transaction via `onchainos wallet contract-call --force`.
5. Report txHash and block explorer link.

**Flags:**
- `--slippage-bps` -- slippage tolerance in basis points (default: 50 = 0.5%)
- `--chain` -- chain ID (default: 8453 = Base)
- `--dry-run` -- print calldata without submitting (no wallet required)

**Examples:**
```bash
# Swap 0.001 ETH -> USDC on Base (0.5% slippage)
1inch swap --src ETH --dst USDC --amount 0.001 --chain 8453

# Swap 100 USDC -> ETH on Ethereum (1% slippage)
1inch swap --src USDC --dst ETH --amount 100 --slippage-bps 100 --chain 1

# Dry-run: preview calldata without broadcasting
1inch swap --src ETH --dst USDC --amount 0.001 --chain 8453 --dry-run

# Swap 10 MATIC -> USDC on Polygon
1inch swap --src MATIC --dst USDC --amount 10 --chain 137
```

**Notes:**
- Slippage is passed to 1inch as a percentage: 50 bps -> 0.5%.
- For ETH/BNB/MATIC inputs, no ERC-20 approval step is needed.
- `--force` is required on all write calls via onchainos -- without it, the tx never broadcasts.
- Ask user to confirm each `wallet contract-call` before executing (approve and swap steps).

---

### `get-allowance` -- Check ERC-20 allowance (read-only)

Check the current ERC-20 allowance granted by the connected wallet to the 1inch router.

**Trigger phrases (English):** "check 1inch allowance", "my 1inch approval", "how much is approved for 1inch", "1inch token allowance"

**Trigger phrases (Chinese):** "查询1inch授权额度", "1inch代币授权", "我的1inch批准额度", "1inch allowance查询"

```
1inch get-allowance \
  --token <token_or_address> \
  [--chain 1|42161|8453|56|137]
```

**Examples:**
```bash
# Check USDC allowance on Base
1inch get-allowance --token USDC --chain 8453

# Check USDT allowance on Ethereum
1inch get-allowance --token USDT --chain 1
```

Read-only -- calls 1inch API. No transaction. No confirmation required.

---

### `approve` -- Approve ERC-20 for 1inch router

Approve an ERC-20 token for use by the 1inch AggregationRouterV6. Required before swapping ERC-20 tokens.

**Trigger phrases (English):** "approve token for 1inch", "1inch approval", "allow 1inch to spend", "approve USDC 1inch"

**Trigger phrases (Chinese):** "授权1inch使用代币", "1inch代币授权", "批准1inch花费", "授权1inch交换USDC"

```
1inch approve \
  --token <token_or_address> \
  [--amount <human_amount>] \
  [--chain 1|42161|8453|56|137] \
  [--dry-run]
```

**Execution flow:**

1. Fetch approve calldata from 1inch /approve/transaction API.
2. Ask user to confirm before broadcasting the approve transaction via `onchainos wallet contract-call --force`.
3. Report txHash and block explorer link.

**Flags:**
- `--amount` -- approval amount in human-readable units; omit for unlimited (uint256 max, recommended)
- `--chain` -- chain ID (default: 8453 = Base)
- `--dry-run` -- print calldata without submitting (no wallet required)

**Examples:**
```bash
# Approve unlimited USDC on Base
1inch approve --token USDC --chain 8453

# Approve exactly 100 USDC on Ethereum
1inch approve --token USDC --amount 100 --chain 1

# Dry-run approve (no broadcast)
1inch approve --token USDC --chain 8453 --dry-run
```

**Notes:**
- `response.to` is the token contract address (not the router). The calldata encodes the 1inch router as spender.
- ERC-20 approve selector: `0x095ea7b3` (approve(address,uint256)).
- Ask user to confirm the `wallet contract-call` before executing.
- Omitting `--amount` approves uint256 max (unlimited), which avoids re-approval on future swaps.

---

## Contract Addresses

| Contract | Address (all chains) |
|----------|---------------------|
| 1inch AggregationRouterV6 | `0x111111125421cA6dc452d289314280a0f8842A65` |

The router address is identical on Ethereum, Arbitrum, Base, BSC, and Polygon.

---

## Common Token Addresses

### Base (8453)
| Token | Address | Decimals |
|-------|---------|----------|
| ETH (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | 6 |
| cbETH | `0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22` | 18 |

### Ethereum (1)
| Token | Address | Decimals |
|-------|---------|----------|
| ETH (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | 18 |
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | 6 |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | 6 |
| DAI | `0x6B175474E89094C44Da98b954EedeAC495271d0F` | 18 |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` | 8 |
| 1INCH | `0x111111111117dC0aa78b770fA6A738034120C302` | 18 |

### Arbitrum (42161)
| Token | Address | Decimals |
|-------|---------|----------|
| ETH (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | 18 |
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` | 6 |
| USDT | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` | 6 |
| ARB | `0x912CE59144191C1204E64559FE8253a0e49E6548` | 18 |

### BSC (56)
| Token | Address | Decimals |
|-------|---------|----------|
| BNB (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WBNB | `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` | 18 |
| USDC | `0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d` | 18 |
| USDT | `0x55d398326f99059fF775485246999027B3197955` | 18 |
| BUSD | `0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56` | 18 |

### Polygon (137)
| Token | Address | Decimals |
|-------|---------|----------|
| MATIC (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WMATIC | `0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270` | 18 |
| USDC | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` | 6 |
| USDT | `0xc2132D05D31c914a87C6611C10748AEb04B58e8F` | 6 |
| WETH | `0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619` | 18 |
