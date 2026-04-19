# 1inch — Plugin Design Document

> Complete interface design for the `1inch` plugin. This document is the authoritative reference for the Developer Agent.
>
> **Scope:** 1inch Aggregation Protocol v6 — the leading DEX aggregator. Routes swaps across 200+ liquidity sources to find the best execution price. This document covers all five target chains: Ethereum (1), Arbitrum (42161), Base (8453), BSC (56), Polygon (137). All on-chain writes use onchainos `wallet contract-call`.

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `1inch` |
| dapp_name | 1inch |
| dapp_repo | https://github.com/1inch/1inchProtocol |
| dapp_alias | 1inch, oneinch, 1inch aggregator, 1inch swap |
| one_liner | Swap tokens at the best rates across 200+ DEXs via the 1inch aggregation protocol — supports Ethereum, Arbitrum, Base, BSC, and Polygon |
| category | defi-protocol |
| tags | dex, swap, aggregator, evm, 1inch, best-price, routing |
| target_chains | ethereum (1), arbitrum (42161), base (8453), bsc (56), polygon (137) |
| target_protocols | 1inch Aggregation Protocol V6 |
| version | 0.1.0 |
| integration_path | API (calldata from 1inch Aggregation API v6, broadcast via onchainos wallet contract-call) |

---

## 1. Feasibility Research

### 1a. Feasibility Table

| Check Item | Result |
|------------|--------|
| Rust SDK? | **None official.** No official Rust SDK from 1inch. Community Go SDK exists (`1inch/1inch-sdk-go`). Python wrapper `1inch.py` available on PyPI but not Rust-compatible. Integration approach: call the REST API directly via `reqwest`. |
| SDK supports which stacks? | Official SDKs: Go (`1inch/1inch-sdk-go`), TypeScript (via 1inch DevPortal). No Rust, Python official. |
| REST API? | **Yes — 1inch Aggregation API v6.** Base URL: `https://api.1inch.dev/swap/v6.0/{chainId}`. Full HTTPS REST API for quotes, swap calldata, allowances, and approve transactions. API key required (Bearer token). |
| Official Skill? | **No.** No onchainos-native skill exists for 1inch. |
| Open-source community Skill (onchainos)? | **Not found** (2026-04-19 search). No existing onchainos plugin for 1inch in known community repos. |
| Supported chains? | Ethereum (1), Arbitrum (42161), Base (8453), BSC (56), Polygon (137), Optimism (10), and 10+ others. This plugin targets the five chains listed above. |
| Requires onchainos broadcast? | **Yes for writes.** Swap and approve operations require broadcasting the calldata returned by the 1inch API via `onchainos wallet contract-call`. Read operations (get-quote, get-allowance) are pure REST API calls — no broadcast. |

### 1b. Integration Path Decision

**Path: REST API (calldata from 1inch API, broadcast via onchainos)**

Rationale:
- No official Rust SDK. The 1inch API is the canonical integration path for all non-TypeScript/non-Go clients.
- The API returns ready-to-broadcast calldata (`tx.data`, `tx.to`, `tx.value`) for swap operations — no ABI encoding required by the plugin.
- For approvals, the API also provides a `/approve/transaction` endpoint that returns ERC-20 approve calldata — the plugin broadcasts it directly.
- The pattern of "fetch calldata from API, broadcast via onchainos" is the same architecture used by Jupiter (Solana) in this pipeline and maps cleanly to the onchainos plugin model.
- **Decision:** Call 1inch API v6 for all data and calldata generation. Broadcast swap and approve calldata via `onchainos wallet contract-call --force`. All reads (quote, allowance) are HTTP GET only — no broadcast.

---

## 2. Interface Mapping

### 2a. API Base URL and Authentication

**Base URL pattern:** `https://api.1inch.dev/swap/v6.0/{chainId}`

| Chain | Chain ID | Base URL |
|-------|----------|----------|
| Ethereum | 1 | `https://api.1inch.dev/swap/v6.0/1` |
| Arbitrum | 42161 | `https://api.1inch.dev/swap/v6.0/42161` |
| Base | 8453 | `https://api.1inch.dev/swap/v6.0/8453` |
| BSC | 56 | `https://api.1inch.dev/swap/v6.0/56` |
| Polygon | 137 | `https://api.1inch.dev/swap/v6.0/137` |

**Authentication:** Bearer token in HTTP header.
```
Authorization: Bearer {API_KEY}
```
API keys are obtained from the 1inch Developer Portal (https://portal.1inch.dev). For demo/testing purposes, use API key `demo` (rate-limited). Set the real key via `ONEINCH_API_KEY` environment variable.

### 2b. 1inch Router V6 Contract Addresses

The 1inch AggregationRouterV6 is deployed at the **same address across all five target chains**:

| Chain | Chain ID | Router V6 Address | Source |
|-------|----------|-------------------|--------|
| Ethereum | 1 | `0x111111125421cA6dc452d289314280a0f8842A65` | Etherscan verified |
| Arbitrum | 42161 | `0x111111125421cA6dc452d289314280a0f8842A65` | Arbiscan verified |
| Base | 8453 | `0x111111125421cA6dc452d289314280a0f8842A65` | BaseScan verified |
| BSC | 56 | `0x111111125421cA6dc452d289314280a0f8842A65` | BscScan verified |
| Polygon | 137 | `0x111111125421cA6dc452d289314280a0f8842A65` | PolygonScan verified |

> **Note:** The router address is uniform across all supported chains for RouterV6. Do not confuse with RouterV4 (`0x1111111254fb6c44bac0bed2854e76f90643097d`) or RouterV5 (`0x1111111254EEB25477B68fb85Ed929f73A960582`) — those are legacy versions.

> **Spender for ERC-20 approvals:** Always use the router address above as the `spender` in `approve(address,uint256)` calls. The `/approve/spender` endpoint confirms this address at runtime.

### 2c. Operations Table

| # | Operation | Type | Description |
|---|-----------|------|-------------|
| 1 | `get-quote` | API (read-only) | Get best swap rate and expected output via GET /quote |
| 2 | `swap` | API → broadcast | Get swap calldata via GET /swap, broadcast via onchainos |
| 3 | `get-allowance` | API (read-only) | Check current ERC-20 allowance for 1inch router via GET /approve/allowance |
| 4 | `approve` | API → broadcast | Get ERC-20 approve calldata via GET /approve/transaction, broadcast via onchainos |

### 2d. ETH Address Convention

1inch uses a sentinel address for native ETH (not WETH):
- **ETH (native):** `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE`
- When swapping FROM native ETH, set `value` in the broadcast to the ETH amount in wei.
- When swapping TO native ETH, no approve needed for the input side.
- WETH addresses are chain-specific (see §2h Token Map).

---

### 2e. Operation 1: `get-quote` — Read-Only

**Endpoint:** `GET {base_url}/quote`

**Purpose:** Returns the expected output amount for a swap without broadcasting a transaction.

**Required parameters:**

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `src` | string | Source token address | `0xEeee...EEeE` (ETH) |
| `dst` | string | Destination token address | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` (USDC on Base) |
| `amount` | string | Input amount in smallest unit (wei) | `1000000000000000` (0.001 ETH) |

**Optional parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `fee` | string | — | Protocol fee percentage (e.g. `"1"` = 1%) |
| `protocols` | string | all | Comma-separated protocol list to restrict routing |
| `gasPrice` | string | fast | `"fast"` or explicit wei value |
| `complexityLevel` | int | 2 | Routing complexity: 0 (fastest) to 3 (best rate) |
| `parts` | int | 20 | Number of split route parts |
| `mainRouteParts` | int | 10 | Main route parts for routing algorithm |

**Response fields to extract:**

| Field | Type | Description |
|-------|------|-------------|
| `dstAmount` | string | Expected output amount in smallest token unit |
| `protocols` | array | Routing path used (array of protocol hops) |
| `srcToken.decimals` | int | Source token decimals (for display) |
| `dstToken.decimals` | int | Destination token decimals (for display) |
| `srcToken.symbol` | string | Source token symbol |
| `dstToken.symbol` | string | Destination token symbol |

**Display to user:** "For X {srcSymbol}, you will receive approximately Y {dstSymbol} (routed via 1inch on {chain})."

**This is a read-only operation — no transaction is broadcast.**

---

### 2f. Operation 2: `swap` — Fetch Calldata + Broadcast

**Endpoint:** `GET {base_url}/swap`

**Purpose:** Returns ready-to-broadcast calldata for the swap. The plugin extracts `tx.data`, `tx.to`, `tx.value` and broadcasts via onchainos.

**Required parameters:**

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `src` | string | Source token address | `0xEeee...EEeE` |
| `dst` | string | Destination token address | `0x833589...02913` |
| `amount` | string | Input amount in wei | `"1000000000000000"` |
| `from` | string | Sender wallet address | `0xYourWallet` |
| `slippage` | float | Slippage tolerance **in percent** (e.g. `0.5` = 0.5%) | `0.5` |

> **Critical:** 1inch `slippage` is expressed in **percent** (0.5 = 0.5%), NOT basis points. This differs from many other protocols. Convert: `slippage_bps / 100 = slippage_percent`. Default config: `slippage_bps = 50` → `slippage = 0.5`.

**Optional parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `disableEstimate` | bool | false | Skip gas estimation (set `true` to avoid errors on dry-run) |
| `allowPartialFill` | bool | false | Allow partial fill if full amount cannot be routed |
| `fee` | string | — | Protocol fee |
| `protocols` | string | all | Restrict to specific protocols |
| `gasPrice` | string | fast | Gas price preference |

**Response fields to extract:**

| Field | Path in JSON | Type | Description |
|-------|-------------|------|-------------|
| calldata | `tx.data` | string | Encoded swap calldata — pass to `--input-data` |
| router address | `tx.to` | string | 1inch router address — pass to `--to` |
| ETH value | `tx.value` | string | Native ETH to send in wei — pass to `--amt` if > 0 |
| gas | `tx.gas` | int | Estimated gas (informational) |
| gasPrice | `tx.gasPrice` | string | Recommended gas price in wei |
| expected output | `dstAmount` | string | Expected output amount (for display) |

**Broadcast command (token → token, no ETH value):**
```bash
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <tx.to> \
  --input-data <tx.data> \
  --force
```

**Broadcast command (ETH → token, with value):**
```bash
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <tx.to> \
  --input-data <tx.data> \
  --amt <tx.value> \
  --force
```

**Full swap workflow:**

1. Resolve wallet address: `onchainos wallet balance --chain <ID>` → parse address from output.
2. Call `GET /quote` to display expected output to user before sending tx.
3. If `src` is NOT native ETH: check allowance via `GET /approve/allowance`. If `allowance < amount`, call `GET /approve/transaction` and broadcast approve tx, then poll `wait_for_tx` before proceeding.
4. Call `GET /swap` with `from`, `slippage`, and all required params.
5. Extract `tx.data`, `tx.to`, `tx.value` from response.
6. Broadcast via `onchainos wallet contract-call --force`. Parse `txHash` from output.
7. Display: "Swap submitted! txHash: `{hash}`. View on {explorer}: `{url}`"

---

### 2g. Operation 3: `get-allowance` — Read-Only

**Endpoint:** `GET {base_url}/approve/allowance`

**Purpose:** Returns the current ERC-20 allowance granted by `walletAddress` to the 1inch router for a given token.

**Required parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `tokenAddress` | string | ERC-20 token contract address |
| `walletAddress` | string | Wallet address to check allowance for |

**Response:**
```json
{
  "allowance": "115792089237316195423570985008687907853269984665640564039457584007913129639935"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `allowance` | string | Current allowance in smallest token unit. `"0"` means no approval. Max uint256 means unlimited approval. |

**Logic:** If `allowance >= amount_to_swap`, no approve tx is needed. Otherwise, broadcast approve.

**This is a read-only operation — no transaction is broadcast.**

---

### 2h. Operation 4: `approve` — Fetch Calldata + Broadcast

**Two sub-operations used internally:**

#### 2h-i: Get Spender Address

**Endpoint:** `GET {base_url}/approve/spender`

**Purpose:** Returns the canonical 1inch router address that must be approved as spender.

**No parameters required.**

**Response:**
```json
{
  "address": "0x111111125421cA6dc452d289314280a0f8842A65"
}
```

> This should match the Router V6 address in §2b. Always confirm via API at runtime rather than hardcoding, in case 1inch deploys a new router version.

#### 2h-ii: Get Approve Transaction Calldata

**Endpoint:** `GET {base_url}/approve/transaction`

**Purpose:** Returns ERC-20 approve calldata for the plugin to broadcast.

**Required parameters:**

| Parameter | Type | Description |
|-----------|------|-------------|
| `tokenAddress` | string | ERC-20 token to approve |

**Optional parameters:**

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `amount` | string | `uint256.max` | Approval amount in smallest token unit. Omit for unlimited (recommended). |

**Response fields:**

| Field | Type | Description |
|-------|------|-------------|
| `to` | string | Token contract address (the ERC-20 to be approved) |
| `data` | string | Encoded `approve(spender, amount)` calldata (selector `0x095ea7b3`) |
| `value` | string | Always `"0"` for ERC-20 approve |
| `gasPrice` | string | Recommended gas price in wei |

**Broadcast command:**
```bash
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <response.to> \
  --input-data <response.data> \
  --force
```

> `response.to` is the **token contract** address (not the router). The calldata in `response.data` already encodes the router as the spender with selector `0x095ea7b3`.

**ERC-20 approve selector confirmation:**
- `cast sig "approve(address,uint256)"` = `0x095ea7b3` ✓ (standard ERC-20)
- `cast sig "allowance(address,address)"` = `0xdd62ed3e` ✓ (for direct eth_call fallback)

**After approve broadcast:** poll `wait_for_tx` receipt before executing swap. Do NOT use a fixed sleep — use receipt polling. See §6c.

---

### 2i. Token Addresses (Built-in Map)

For unlisted tokens: use `onchainos token search --keyword <SYMBOL> --chain <CHAIN_ID>`.

#### Ethereum (1)
| Symbol | Address | Decimals |
|--------|---------|----------|
| ETH (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | 18 |
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | 6 |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | 6 |
| DAI | `0x6B175474E89094C44Da98b954EedeAC495271d0F` | 18 |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` | 8 |
| 1INCH | `0x111111111117dC0aa78b770fA6A738034120C302` | 18 |

#### Arbitrum (42161)
| Symbol | Address | Decimals |
|--------|---------|----------|
| ETH (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | 18 |
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` | 6 |
| USDC.e | `0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8` | 6 |
| USDT | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` | 6 |
| ARB | `0x912CE59144191C1204E64559FE8253a0e49E6548` | 18 |

#### Base (8453)
| Symbol | Address | Decimals |
|--------|---------|----------|
| ETH (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | 6 |
| cbETH | `0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22` | 18 |

#### BSC (56)
| Symbol | Address | Decimals |
|--------|---------|----------|
| BNB (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WBNB | `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` | 18 |
| USDC | `0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d` | 18 |
| USDT | `0x55d398326f99059fF775485246999027B3197955` | 18 |
| BUSD | `0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56` | 18 |

#### Polygon (137)
| Symbol | Address | Decimals |
|--------|---------|----------|
| MATIC (native) | `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` | 18 |
| WMATIC | `0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270` | 18 |
| USDC | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` | 6 |
| USDC.e | `0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359` | 6 |
| USDT | `0xc2132D05D31c914a87C6611C10748AEb04B58e8F` | 6 |
| WETH | `0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619` | 18 |

---

## 3. User Scenarios

### Scenario 1: Get a Quote — ETH → USDC on Base (Read-Only)

**User says:** "How much USDC would I get for 0.001 ETH on 1inch on Base?"

**Agent actions:**

1. **[Off-chain] Resolve params**
   - `src = 0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` (native ETH)
   - `dst = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` (USDC on Base, 6 decimals)
   - `amount = 1000000000000000` (0.001 ETH in wei)
   - `chain_id = 8453`

2. **[API Call — read-only]**
   ```
   GET https://api.1inch.dev/swap/v6.0/8453/quote
     ?src=0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
     &dst=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
     &amount=1000000000000000
   Authorization: Bearer {API_KEY}
   ```

3. **[Parse response]**
   - Extract `dstAmount` (e.g. `"2450000"` = 2.45 USDC with 6 decimals)
   - Extract `protocols` for routing info

4. **Display to user:**
   "For 0.001 ETH, you will receive approximately 2.45 USDC on 1inch (Base). Route: Uniswap V3 → USDC. No transaction submitted."

No confirmation needed — read-only operation.

---

### Scenario 2: Swap 0.001 ETH → USDC on Base (Native ETH Input)

**User says:** "Swap 0.001 ETH for USDC on 1inch on Base"

**Agent actions:**

1. **[Off-chain] Resolve wallet address**
   - `onchainos wallet balance --chain 8453` → parse wallet address from output

2. **[API Call] Get quote for display**
   ```
   GET https://api.1inch.dev/swap/v6.0/8453/quote
     ?src=0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
     &dst=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
     &amount=1000000000000000
   ```
   - Display expected output: "You will receive ~2.45 USDC"

3. **[Skip approve]** — Source token is native ETH. No ERC-20 approval needed.

4. **[API Call] Get swap calldata**
   ```
   GET https://api.1inch.dev/swap/v6.0/8453/swap
     ?src=0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
     &dst=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
     &amount=1000000000000000
     &from=<wallet_address>
     &slippage=0.5
   ```
   - Extract `tx.data`, `tx.to`, `tx.value` (will equal `"1000000000000000"` for ETH swaps)

5. **[Chain-on] Broadcast swap**
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x111111125421cA6dc452d289314280a0f8842A65 \
     --input-data <tx.data> \
     --amt 1000000000000000 \
     --force
   ```

6. **Display result:**
   - Parse `txHash` from output
   - "Swap submitted! Sent 0.001 ETH → ~2.45 USDC. txHash: `0x...`. View on BaseScan: https://basescan.org/tx/{hash}"

---

### Scenario 3: Swap 100 USDC → ETH on Ethereum (ERC-20 Input, Approval Required)

**User says:** "Swap 100 USDC to ETH on 1inch on Ethereum"

**Agent actions:**

1. **[Off-chain] Resolve params**
   - `src = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` (USDC, 6 decimals)
   - `dst = 0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` (native ETH)
   - `amount = 100000000` (100 USDC in smallest unit)
   - `chain_id = 1`

2. **[Off-chain] Resolve wallet address**
   - `onchainos wallet balance --chain 1` → parse wallet address

3. **[API Call] Check USDC allowance**
   ```
   GET https://api.1inch.dev/swap/v6.0/1/approve/allowance
     ?tokenAddress=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
     &walletAddress=<wallet_address>
   ```
   - Response: `{"allowance": "0"}` → approval required

4. **[API Call] Get approve calldata**
   ```
   GET https://api.1inch.dev/swap/v6.0/1/approve/transaction
     ?tokenAddress=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
   ```
   - Extract `response.to` (USDC contract) and `response.data` (approve calldata)

5. **[Chain-on] Broadcast approve**
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
     --input-data <response.data> \
     --force
   ```
   - `wait_for_tx` until confirmed — do NOT proceed until approved.

6. **[API Call] Get quote for display**
   ```
   GET https://api.1inch.dev/swap/v6.0/1/quote
     ?src=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
     &dst=0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
     &amount=100000000
   ```
   - Display: "You will receive approximately 0.0XXX ETH"

7. **[API Call] Get swap calldata**
   ```
   GET https://api.1inch.dev/swap/v6.0/1/swap
     ?src=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
     &dst=0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
     &amount=100000000
     &from=<wallet_address>
     &slippage=0.5
   ```
   - Extract `tx.data`, `tx.to` (router), `tx.value` (should be `"0"` for token input)

8. **[Chain-on] Broadcast swap**
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x111111125421cA6dc452d289314280a0f8842A65 \
     --input-data <tx.data> \
     --force
   ```

9. **Display result:**
   - "Swap submitted! 100 USDC → ~0.0XXX ETH. txHash: `0x...`. View on Etherscan: https://etherscan.io/tx/{hash}"

---

### Scenario 4: Check Current Token Allowance

**User says:** "Check my USDC allowance for 1inch on Base"

**Agent actions:**

1. **[Off-chain] Resolve wallet address**
   - `onchainos wallet balance --chain 8453` → parse wallet address

2. **[API Call] Check allowance**
   ```
   GET https://api.1inch.dev/swap/v6.0/8453/approve/allowance
     ?tokenAddress=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
     &walletAddress=<wallet_address>
   ```

3. **[Parse and display]**
   - If `allowance == "0"`: "No USDC approval granted to the 1inch router on Base. Run `approve` before swapping."
   - If `allowance == "115792089237316195423570985008687907853269984665640564039457584007913129639935"` (uint256 max): "Unlimited USDC approval granted to 1inch router on Base."
   - Otherwise: "Current USDC allowance: {human-readable amount} USDC on Base."

No transaction broadcast — read-only.

---

### Scenario 5: Swap Tokens on Polygon (MATIC → USDC)

**User says:** "Swap 10 MATIC to USDC on 1inch on Polygon"

**Agent actions:**

1. **[Off-chain]** `src = 0xEeee...` (native MATIC), `dst = 0x2791...` (USDC.e on Polygon, 6 dec), `amount = 10000000000000000000` (10 MATIC in wei), `chain_id = 137`

2. **[Skip approve]** — Native MATIC input, no ERC-20 approval needed.

3. **[API Call] Get swap calldata**
   ```
   GET https://api.1inch.dev/swap/v6.0/137/swap
     ?src=0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE
     &dst=0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174
     &amount=10000000000000000000
     &from=<wallet_address>
     &slippage=0.5
   ```

4. **[Chain-on] Broadcast swap**
   ```bash
   onchainos wallet contract-call \
     --chain 137 \
     --to 0x111111125421cA6dc452d289314280a0f8842A65 \
     --input-data <tx.data> \
     --amt 10000000000000000000 \
     --force
   ```

5. **Display result with PolygonScan link.**

---

## 4. External API Dependencies

| API | Endpoint | Purpose | Auth | Rate Limit |
|-----|----------|---------|------|------------|
| 1inch Aggregation API v6 | `https://api.1inch.dev/swap/v6.0/{chainId}/quote` | Get swap quote | Bearer token | Tiered (see portal) |
| 1inch Aggregation API v6 | `https://api.1inch.dev/swap/v6.0/{chainId}/swap` | Get swap calldata | Bearer token | Tiered |
| 1inch Aggregation API v6 | `https://api.1inch.dev/swap/v6.0/{chainId}/approve/allowance` | Check ERC-20 allowance | Bearer token | Tiered |
| 1inch Aggregation API v6 | `https://api.1inch.dev/swap/v6.0/{chainId}/approve/transaction` | Get approve calldata | Bearer token | Tiered |
| 1inch Aggregation API v6 | `https://api.1inch.dev/swap/v6.0/{chainId}/approve/spender` | Get router spender address | Bearer token | Tiered |

> **API key requirement:** All 1inch API v6 endpoints require a Bearer token (`Authorization: Bearer {API_KEY}`). Keys are obtained at https://portal.1inch.dev. For this demo plugin, set `ONEINCH_API_KEY=demo` — the `demo` key is rate-limited but functional for testing.

> **Error handling:** The API returns HTTP 400 with a JSON body containing `"description"` on error (e.g., insufficient liquidity, bad slippage). Always check HTTP status before parsing response. On 429 (rate limit), retry after 1 second with exponential backoff (max 3 retries).

> **RPC — not needed for reads:** All read operations (quote, allowance) go through the 1inch API directly. No JSON-RPC `eth_call` is required. Only broadcast uses onchainos (which handles its own RPC internally).

---

## 5. Configuration Parameters

| Parameter | Type | Default | Notes |
|-----------|------|---------|-------|
| `chain_id` | u64 | `1` | Chain to operate on. Supported: 1, 56, 137, 8453, 42161 |
| `slippage_bps` | u64 | `50` | Slippage tolerance in **basis points** (50 = 0.5%). Internally converted to percent for API: `slippage = slippage_bps / 100`. |
| `api_key` | String | `"demo"` | 1inch API key. Set via `ONEINCH_API_KEY` env var or config. The `demo` key is rate-limited — replace with a real key for production use. Obtain at https://portal.1inch.dev. |
| `dry_run` | bool | `false` | When `true`: skip broadcast; print calldata only. Call `/swap` with `disableEstimate=true` to avoid gas estimation errors. Do NOT pass `--force` to onchainos in dry-run mode. |
| `allow_partial_fill` | bool | `false` | Allow partial fill if full liquidity is unavailable. |
| `complexity_level` | u8 | `2` | Routing complexity: 0 = fastest, 3 = best rate. Higher values may be slower but find better prices for large trades. |

---

## 6. Key Implementation Notes for Developer Agent

### 6a. Slippage Unit Conversion (Critical)

1inch API takes `slippage` as a **percentage** (e.g. `0.5` = 0.5%), not basis points. The plugin's user-facing config uses basis points (consistent with other plugins in this codebase). Always convert before calling the API:

```rust
let slippage_percent = slippage_bps as f64 / 100.0;  // 50 bps → 0.5
let slippage_str = format!("{}", slippage_percent);    // "0.5"
```

Do NOT pass basis points directly (e.g. `slippage=50` would request 50% slippage — catastrophic).

### 6b. Native Token Sentinel Address

1inch uses `0xEeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` for the native chain token (ETH, BNB, MATIC) on all chains. When the user specifies "ETH", "BNB", or "MATIC" as the source or destination, map to this address. Key implications:

- If `src == sentinel address`: no ERC-20 approval needed; set `--amt <amount_in_wei>` in the broadcast.
- If `dst == sentinel address`: no ERC-20 approval needed for destination.
- If `src` is a real ERC-20: check allowance and approve before swap.

### 6c. Approve-Before-Swap Race Condition

After broadcasting the approve tx, do NOT immediately proceed to the swap with a fixed sleep. Use `wait_for_tx` receipt polling:

```rust
let approve_hash = broadcast_approve_tx(chain_id, token, calldata).await?;
wait_for_tx(&approve_hash, chain_id).await?;
// Only now call /swap and broadcast
```

Fixed sleeps (3s, 5s) are unreliable under network congestion. This is a known issue from the Pendle and Morpho plugins.

### 6d. `--force` Flag on All Write Calls

Every `onchainos wallet contract-call` for a write operation (swap, approve) must include `--force`. Without it, the first call returns with `"confirming": true` and the tx never broadcasts. This is the onchainos safety confirmation gate — `--force` bypasses it for automated plugin use.

### 6e. reqwest Proxy in Sandbox

All HTTP calls (to the 1inch API) must use the proxy-aware reqwest client:

```rust
pub fn build_http_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::https(&url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}
```

### 6f. API Key Header

The `Authorization: Bearer {key}` header must be sent on every request to `api.1inch.dev`. Requests without the header will receive HTTP 401. Set via environment variable `ONEINCH_API_KEY`:

```rust
let api_key = std::env::var("ONEINCH_API_KEY").unwrap_or_else(|_| "demo".to_string());
let response = client
    .get(&url)
    .header("Authorization", format!("Bearer {}", api_key))
    .send()
    .await?;
```

### 6g. Wallet Address Resolution

Resolve the wallet address using onchainos before calling `/swap` (which requires `from`). Do NOT hardcode or assume.

For the `onchainos wallet balance` output: the address is in the JSON output under `.data.address` when using `--output json`, OR can be parsed from the plain-text output. Use `--output json` when available.

> Note from kb: on chain 501 (Solana), `--output json` causes EOF. On EVM chains, `--output json` works correctly.

### 6h. Dry-Run Mode

In dry-run mode:
1. Call `/quote` and `/swap` with `disableEstimate=true` to avoid gas estimation errors when a real wallet is not present.
2. Print the calldata, `tx.to`, and `tx.value` to stdout without broadcasting.
3. Do NOT call `wallet contract-call` (no `--force`, no broadcast).
4. Display: "Dry-run mode: swap calldata generated. Broadcasting skipped."

### 6i. Block Explorer URLs

| Chain | Explorer | URL Pattern |
|-------|----------|-------------|
| Ethereum (1) | Etherscan | `https://etherscan.io/tx/{hash}` |
| Arbitrum (42161) | Arbiscan | `https://arbiscan.io/tx/{hash}` |
| Base (8453) | BaseScan | `https://basescan.org/tx/{hash}` |
| BSC (56) | BscScan | `https://bscscan.com/tx/{hash}` |
| Polygon (137) | PolygonScan | `https://polygonscan.com/tx/{hash}` |

### 6j. API Error Handling

| HTTP Status | Meaning | Action |
|------------|---------|--------|
| 200 | OK | Parse JSON response normally |
| 400 | Bad request / no route found | Extract `"description"` field from JSON and display to user |
| 401 | Invalid or missing API key | "Invalid API key. Set ONEINCH_API_KEY environment variable." |
| 429 | Rate limit exceeded | Retry after 1s with exponential backoff (max 3 retries) |
| 500 | 1inch server error | "1inch API temporarily unavailable. Please try again." |

Common 400 errors:
- `"insufficient liquidity"` — not enough liquidity for the requested amount/pair
- `"cannot estimate"` — gas estimation failed (use `disableEstimate=true` for dry-run)

---

## 7. Submission Metadata

| Field | Value |
|-------|-------|
| plugin_store_name | `1inch` |
| binary_name | `1inch` |
| source_repo | `skylavis-sky/onchainos-plugins` |
| source_dir | `1inch` |
| category | `defi-protocol` |
| license | MIT |
| reference_plugin | `jupiter` (same API→calldata→broadcast pattern for Solana) |
