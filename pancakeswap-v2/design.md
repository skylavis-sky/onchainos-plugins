# PancakeSwap V2 AMM — Plugin Design Document

> Complete interface design for the `pancakeswap-v2` plugin. This document is the authoritative reference for the Developer Agent.
>
> **Scope:** PancakeSwap V2 — the xyk constant-product AMM. Distinct from V3 (CLMM / concentrated liquidity, covered by PR #82). V2 uses the UniswapV2Router02 pattern: full-range LP tokens (ERC-20, not NFT), Router02 for swaps, and the V2 Factory for pair lookup.

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `pancakeswap-v2` |
| dapp_name | PancakeSwap V2 |
| dapp_repo | https://github.com/pancakeswap/pancake-smart-contracts |
| dapp_alias | pancake v2, pcs v2, pancakeswap amm, pancake amm |
| one_liner | Swap tokens and provide full-range liquidity on PancakeSwap V2 — the xyk AMM on BSC |
| category | defi-protocol |
| tags | dex, swap, liquidity, amm, pancakeswap, bsc, v2, xyk, lp |
| target_chains | bsc (56) primary; base (8453) secondary |
| target_protocols | PancakeSwap V2 AMM (constant-product xyk) |
| version | 0.1.0 |
| integration_path | Direct on-chain (ABI calldata) via Router02 + eth_call reads |

---

## 1. Feasibility Research

### 1a. Feasibility Table

| Check Item | Result |
|------------|--------|
| Rust SDK? | **No.** PancakeSwap has no Rust SDK. Official SDKs are TypeScript-only: `@pancakeswap/sdk` (V2/V3 token math) and `@pancakeswap/smart-router`. No Rust bindings exist. |
| SDK supports which stacks? | TypeScript / JavaScript only (npm packages `@pancakeswap/sdk`, `@pancakeswap/smart-router`). No Go, Python, or Rust. |
| REST API? | **Partial / read-only.** `https://api.pancakeswap.info/api/v2/tokens` provides token price data (read-only). No API for swap execution or LP management. All write operations require direct contract calls. |
| Official Skill? | **No.** `pancakeswap-ai` (https://github.com/pancakeswap/pancakeswap-ai) generates UI deep links for the web app, not autonomous on-chain execution. Not suitable for onchainos. |
| Open-source community Skill (onchainos)? | **No standalone V2 plugin.** `okx/onchainos-skills` includes `okx-defi-invest` covering PancakeSwap farming/staking via OKX aggregation, but it does not expose raw V2 Router02 calldata construction. An existing onchainos PancakeSwap V3 plugin (PR #82) covers CLMM; no V2 plugin exists. |
| Supported chains? | V2 is deployed on BSC (56), Base (8453), Ethereum (1), Arbitrum (42161), zkSync Era (324), and others. **This plugin targets BSC (56) as primary, Base (8453) as secondary.** |
| Requires onchainos broadcast? | **Yes.** All swap, addLiquidity, and removeLiquidity operations are on-chain write operations requiring `onchainos wallet contract-call`. ERC-20 approvals are also on-chain via `contract-call`. Read operations (getAmountsOut quote, getReserves, pair lookup) use off-chain `eth_call` over JSON-RPC. |

### 1b. Integration Path Decision

**Path: Direct On-Chain (ABI calldata) — no SDK**

Rationale:
- No Rust SDK exists.
- PancakeSwap V2 is a minimal fork of UniswapV2. Router02 ABI is fully documented and stable.
- All read operations (`getAmountsOut`, `getReserves`, `getPair`) are single `eth_call`s — no subgraph required.
- LP tokens are standard ERC-20 tokens, so `approve` + `removeLiquidity` follows a well-known pattern.
- The existing V3 plugin (PR #82) established the calldata-construction pattern for this codebase; V2 is simpler (no fee tiers, no tick math).
- **Decision:** Construct ABI-encoded calldata directly in Rust, submit via `onchainos wallet contract-call`. Off-chain reads use JSON-RPC `eth_call`.

---

## 2. Interface Mapping

### 2a. Contract Addresses

#### BSC (Chain ID: 56) — Primary

| Contract | Address | Source |
|----------|---------|--------|
| PancakeSwap V2 Router02 | `0x10ED43C718714eb63d5aA57B78B54704E256024E` | Official docs + BscScan verified |
| PancakeSwap V2 Factory | `0xcA143Ce32Fe78f1f7019d7d551a6402fC5350c73` | Official docs + BscScan verified |
| WBNB (wrapped BNB) | `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` | BscScan verified |

#### Base (Chain ID: 8453) — Secondary

| Contract | Address | Source |
|----------|---------|--------|
| PancakeSwap V2 Router02 | `0x8cFe327CEc66d1C090Dd72bd0FF11d690C33a2Eb` | BaseScan verified / V2 factory config |
| PancakeSwap V2 Factory | `0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E` | BaseScan verified (referenced in V3 design.md) |
| WETH (native wrap on Base) | `0x4200000000000000000000000000000000000006` | Base canonical WETH |

> **Address safety note:** These addresses are confirmed against official PancakeSwap deployment docs and block explorer verified labels. The Router02 address is stable (immutable contract, no upgrades). Do NOT resolve Router02 from an on-chain registry — there is none. At plugin startup, validate the Router02 factory() return value matches the expected V2 Factory address to detect misconfiguration.

> **Runtime validation:** Call `Router02.factory()` (selector `0xc45a0155`) and verify it equals the expected Factory address. Call `Router02.WETH()` (selector `0xad5c4648`) and verify it equals WBNB/WETH. These are cheap `eth_call`s that confirm the address is correct.

### 2b. V2 AMM Model

PancakeSwap V2 uses the **constant-product (xyk) formula**: `x * y = k`. Key properties:
- Pairs are full-range: liquidity covers the entire price range (no tick math).
- LP tokens are standard ERC-20 tokens. One LP token per pair.
- Swap fee: **0.25%** on BSC V2 (0.17% to LPs, 0.08% to treasury). No fee tier selection needed.
- `getAmountsOut` on Router02 computes output amounts including the fee.
- No NFT positions; no `NonfungiblePositionManager`. LP is represented as ERC-20 balance.

### 2c. Operations Table

| # | Operation | Type | Description |
|---|-----------|------|-------------|
| 1 | `quote` | Chain-off (eth_call) | Get expected output amount for a swap path via `getAmountsOut` |
| 2 | `swap` | Chain-on | Swap exact tokens for tokens (or ETH/BNB) via Router02 |
| 3 | `add-liquidity` | Chain-on | Add liquidity to a V2 pair and receive LP tokens |
| 4 | `remove-liquidity` | Chain-on | Burn LP tokens and withdraw both tokens |
| 5 | `get-pair` | Chain-off (eth_call) | Look up the pair contract address for two tokens |
| 6 | `get-reserves` | Chain-off (eth_call) | Get current reserves of a pair (for price/ratio) |
| 7 | `lp-balance` | Chain-off (eth_call) | Get user's LP token balance for a pair |

---

### 2d. Off-Chain Read Operations

#### Operation 1: `quote` — getAmountsOut

**Contract:** Router02  
**Function:** `getAmountsOut(uint256 amountIn, address[] calldata path)`  
**Selector:** `0xd06ca61f` *(verified: keccak256("getAmountsOut(uint256,address[])") → d06ca61f)*

**eth_call calldata construction:**
```
selector:    d06ca61f
amountIn:    <32 bytes, uint256, raw amount in minimal units>
offset:      0000...0040  (offset to path array = 64 bytes)
path_len:    <32 bytes, uint256, number of addresses>
path[0]:     <32 bytes, address padded>
path[1]:     <32 bytes, address padded>
...
```

**Parameters:**
| Name | Type | Notes |
|------|------|-------|
| amountIn | uint256 | Amount in minimal token units (e.g. 1 USDT = 1_000_000 for 6 decimals) |
| path | address[] | Array of token addresses: [tokenIn, tokenOut] for direct swap, or [tokenIn, intermediate, tokenOut] for multi-hop |

**Returns:** `uint256[]` — amounts at each step. `amounts[path.length - 1]` is the final output amount.

**Example (1 BNB → CAKE on BSC):**
- `path = [WBNB, CAKE]`
- Returns `[1e18, <cake_amount>]`
- Display: `amounts[1] / 10^18` CAKE

---

#### Operation 5: `get-pair` — Factory.getPair

**Contract:** V2 Factory  
**Function:** `getPair(address tokenA, address tokenB)`  
**Selector:** `0xe6a43905` *(verified: keccak256("getPair(address,address)") → e6a43905)*

**Returns:** `address` — pair contract address. Returns `address(0)` if pair does not exist.

**eth_call calldata:**
```
e6a43905
000000000000000000000000<tokenA_no_0x>
000000000000000000000000<tokenB_no_0x>
```

---

#### Operation 6: `get-reserves` — Pair.getReserves

**Contract:** Pair contract (address from `getPair`)  
**Function:** `getReserves()`  
**Selector:** `0x0902f1ac` *(verified: keccak256("getReserves()") → 0902f1ac)*

**Returns:** `(uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast)`
- `reserve0` corresponds to `token0()` (lower address lexicographically)
- `reserve1` corresponds to `token1()`

Also call `token0()` (`0x0dfe1681`) and `token1()` (`0xd21220a7`) to map reserve0/reserve1 to the tokens.

**Derived price:** `price_of_token0_in_token1 = reserve1 / reserve0` (scaled by decimals).

---

#### Operation 7: `lp-balance` — LP ERC-20 balanceOf

**Contract:** Pair contract (ERC-20)  
**Function:** `balanceOf(address account)`  
**Selector:** `0x70a08231` *(verified: keccak256("balanceOf(address)") → 70a08231)*

Also call `totalSupply()` (`0x18160ddd`) to compute the user's share of the pool.

**User pool share:**
```
share = user_lp_balance / total_lp_supply
token0_owned = reserve0 * share
token1_owned = reserve1 * share
```

---

### 2e. On-Chain Write Operations

All on-chain writes use `onchainos wallet contract-call --chain <ID> --to <CONTRACT> --input-data <HEX_CALLDATA> --force`.

---

#### Operation 2: `swap` — swapExactTokensForTokens / swapExactETHForTokens

**Two variants depending on whether tokenIn is native BNB/ETH:**

##### Variant A: Token → Token (non-native input)

**Pre-step:** Approve Router02 to spend tokenIn (if allowance < amountIn).

**Step 1 — Approve (if needed):**
```
Contract: tokenIn (ERC-20)
Selector: 0x095ea7b3  (verified: keccak256("approve(address,uint256)") → 095ea7b3)
Calldata:
  095ea7b3
  000000000000000000000000<router02_no_0x>   # spender
  ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  # uint256.max
```

**Check allowance before approving:**
```
Selector: 0xdd62ed3e  (verified: keccak256("allowance(address,address)") → dd62ed3e)
Calldata:
  dd62ed3e
  000000000000000000000000<owner_no_0x>
  000000000000000000000000<router02_no_0x>
```
Skip approve if current allowance >= amountIn.

**Step 2 — Swap:**
**Function:** `swapExactTokensForTokens(uint256 amountIn, uint256 amountOutMin, address[] calldata path, address to, uint256 deadline)`  
**Selector:** `0x38ed1739` *(verified: keccak256("swapExactTokensForTokens(uint256,uint256,address[],address,uint256)") → 38ed1739)*

```
Calldata layout (all uint256 = 32 bytes, address padded to 32):
  38ed1739
  <amountIn, uint256>
  <amountOutMin, uint256>         # amountOut * (1 - slippage_bps/10000)
  00000000000000000000000000000000000000000000000000000000000000a0  # offset to path
  000000000000000000000000<to_no_0x>                                # recipient wallet
  <deadline, uint256>             # block.timestamp + 300 (5 minutes)
  <path_len, uint256>             # number of addresses in path
  000000000000000000000000<path[0]_no_0x>
  000000000000000000000000<path[1]_no_0x>
  ...
```

onchainos command:
```bash
onchainos wallet contract-call \
  --chain 56 \
  --to 0x10ED43C718714eb63d5aA57B78B54704E256024E \
  --input-data <hex_calldata> \
  --force
```

**Wait 3 seconds between approve and swap** (nonce sequencing, per kb/protocols/dex.md).

##### Variant B: Native BNB/ETH → Token

**Function:** `swapExactETHForTokens(uint256 amountOutMin, address[] calldata path, address to, uint256 deadline)`  
**Selector:** `0x7ff36ab5` *(verified: keccak256("swapExactETHForTokens(uint256,address[],address,uint256)") → 7ff36ab5)*

- No approve needed (native BNB/ETH sent as `--amt`).
- `path[0]` must be WBNB (on BSC) or WETH (on Base).

```
Calldata:
  7ff36ab5
  <amountOutMin, uint256>
  0000000000000000000000000000000000000000000000000000000000000080  # offset to path
  000000000000000000000000<to_no_0x>
  <deadline, uint256>
  <path_len, uint256>
  000000000000000000000000<WBNB_no_0x>
  000000000000000000000000<tokenOut_no_0x>
```

onchainos command (with ETH/BNB value):
```bash
onchainos wallet contract-call \
  --chain 56 \
  --to 0x10ED43C718714eb63d5aA57B78B54704E256024E \
  --input-data <hex_calldata> \
  --amt <amountIn_wei> \
  --force
```

##### Variant C: Token → Native BNB/ETH

**Function:** `swapExactTokensForETH(uint256 amountIn, uint256 amountOutMin, address[] calldata path, address to, uint256 deadline)`  
**Selector:** `0x18cbafe5` *(verified: keccak256("swapExactTokensForETH(uint256,uint256,address[],address,uint256)") → 18cbafe5)*

- Approve tokenIn first.
- `path[last]` must be WBNB/WETH.

---

#### Operation 3: `add-liquidity`

**Two variants: token+token or token+native BNB/ETH.**

##### Variant A: Token + Token

**Pre-step:** Approve Router02 to spend both tokenA and tokenB (if allowances insufficient).

**Function:** `addLiquidity(address tokenA, address tokenB, uint256 amountADesired, uint256 amountBDesired, uint256 amountAMin, uint256 amountBMin, address to, uint256 deadline)`  
**Selector:** `0xe8e33700` *(verified: keccak256("addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)") → e8e33700)*

```
Calldata:
  e8e33700
  000000000000000000000000<tokenA_no_0x>
  000000000000000000000000<tokenB_no_0x>
  <amountADesired, uint256>
  <amountBDesired, uint256>
  <amountAMin, uint256>           # amountADesired * (1 - slippage_bps/10000)
  <amountBMin, uint256>           # amountBDesired * (1 - slippage_bps/10000)
  000000000000000000000000<to_no_0x>
  <deadline, uint256>
```

**Workflow:**
1. Call `getReserves()` on the pair to determine current ratio.
2. If pair does not exist (new pair), caller sets the initial price via the ratio of amountADesired:amountBDesired.
3. Approve tokenA (if allowance < amountADesired). Wait 5 seconds.
4. Approve tokenB (if allowance < amountBDesired). Wait 5 seconds.
5. Call `addLiquidity`.

onchainos command:
```bash
onchainos wallet contract-call \
  --chain 56 \
  --to 0x10ED43C718714eb63d5aA57B78B54704E256024E \
  --input-data <hex_calldata> \
  --force
```

**Returns event:** `Mint(sender, amount0, amount1)` + LP tokens minted to `to`.

##### Variant B: Token + Native BNB/ETH

**Function:** `addLiquidityETH(address token, uint256 amountTokenDesired, uint256 amountTokenMin, uint256 amountETHMin, address to, uint256 deadline)`  
**Selector:** `0xf305d719` *(verified: keccak256("addLiquidityETH(address,uint256,uint256,uint256,address,uint256)") → f305d719)*

- BNB/ETH value sent via `--amt <amountETHDesired_wei>`.
- Approve `token` first (if allowance < amountTokenDesired). Wait 5 seconds.

```
Calldata:
  f305d719
  000000000000000000000000<token_no_0x>
  <amountTokenDesired, uint256>
  <amountTokenMin, uint256>
  <amountETHMin, uint256>
  000000000000000000000000<to_no_0x>
  <deadline, uint256>
```

onchainos command:
```bash
onchainos wallet contract-call \
  --chain 56 \
  --to 0x10ED43C718714eb63d5aA57B78B54704E256024E \
  --input-data <hex_calldata> \
  --amt <amountETHDesired_wei> \
  --force
```

---

#### Operation 4: `remove-liquidity`

**Two variants: receive tokens or receive native BNB/ETH.**

**Pre-step (always):** Approve Router02 to spend LP tokens.

LP token address = pair contract address (from `Factory.getPair(tokenA, tokenB)`).

**Approve LP tokens:**
```
Contract: pair_address
Selector: 0x095ea7b3
Calldata:
  095ea7b3
  000000000000000000000000<router02_no_0x>
  <liquidity_amount, uint256>   (or uint256.max for convenience)
```

##### Variant A: Receive Token + Token

**Function:** `removeLiquidity(address tokenA, address tokenB, uint256 liquidity, uint256 amountAMin, uint256 amountBMin, address to, uint256 deadline)`  
**Selector:** `0xbaa2abde` *(verified: keccak256("removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)") → baa2abde)*

```
Calldata:
  baa2abde
  000000000000000000000000<tokenA_no_0x>
  000000000000000000000000<tokenB_no_0x>
  <liquidity, uint256>            # LP token amount to burn
  <amountAMin, uint256>           # min tokenA to receive (slippage)
  <amountBMin, uint256>           # min tokenB to receive (slippage)
  000000000000000000000000<to_no_0x>
  <deadline, uint256>
```

**Workflow:**
1. Call `getPair(tokenA, tokenB)` to get pair address.
2. Call `balanceOf(wallet)` on pair to get LP balance.
3. Compute expected withdrawal: `tokenA_out = reserve0 * lp_amount / totalSupply` (adjust for token ordering).
4. Apply slippage: `amountAMin = tokenA_out * (1 - slippage_bps/10000)`.
5. Approve LP tokens to Router02. Wait 5 seconds.
6. Call `removeLiquidity`.

onchainos command:
```bash
onchainos wallet contract-call \
  --chain 56 \
  --to 0x10ED43C718714eb63d5aA57B78B54704E256024E \
  --input-data <hex_calldata> \
  --force
```

##### Variant B: Receive Token + Native BNB/ETH

**Function:** `removeLiquidityETH(address token, uint256 liquidity, uint256 amountTokenMin, uint256 amountETHMin, address to, uint256 deadline)`  
**Selector:** `0x02751cec` *(verified: keccak256("removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)") → 02751cec)*

```
Calldata:
  02751cec
  000000000000000000000000<token_no_0x>
  <liquidity, uint256>
  <amountTokenMin, uint256>
  <amountETHMin, uint256>
  000000000000000000000000<to_no_0x>
  <deadline, uint256>
```

Use this variant when one side of the pair is WBNB/WETH — the router unwraps and sends native BNB/ETH to the recipient.

---

### 2f. Function Selector Summary

| Selector | Canonical Signature | Verified |
|----------|--------------------|----|
| `0xd06ca61f` | `getAmountsOut(uint256,address[])` | keccak256 |
| `0xe6a43905` | `getPair(address,address)` | keccak256 |
| `0x0902f1ac` | `getReserves()` | keccak256 |
| `0x0dfe1681` | `token0()` | keccak256 |
| `0xd21220a7` | `token1()` | keccak256 |
| `0x18160ddd` | `totalSupply()` | keccak256 |
| `0x70a08231` | `balanceOf(address)` | keccak256 |
| `0xdd62ed3e` | `allowance(address,address)` | keccak256 |
| `0x095ea7b3` | `approve(address,uint256)` | keccak256 |
| `0xc45a0155` | `factory()` | keccak256 |
| `0xad5c4648` | `WETH()` | keccak256 |
| `0x38ed1739` | `swapExactTokensForTokens(uint256,uint256,address[],address,uint256)` | keccak256 |
| `0x7ff36ab5` | `swapExactETHForTokens(uint256,address[],address,uint256)` | keccak256 |
| `0x18cbafe5` | `swapExactTokensForETH(uint256,uint256,address[],address,uint256)` | keccak256 |
| `0xe8e33700` | `addLiquidity(address,address,uint256,uint256,uint256,uint256,address,uint256)` | keccak256 |
| `0xf305d719` | `addLiquidityETH(address,uint256,uint256,uint256,address,uint256)` | keccak256 |
| `0xbaa2abde` | `removeLiquidity(address,address,uint256,uint256,uint256,address,uint256)` | keccak256 |
| `0x02751cec` | `removeLiquidityETH(address,uint256,uint256,uint256,address,uint256)` | keccak256 |

> All selectors above were computed using `pycryptodome` Keccak-256 and cross-validated against known-good reference selectors (`approve → 0x095ea7b3`, `allowance → 0xdd62ed3e`, `balanceOf → 0x70a08231`, `deposit → 0xd0e30db0`). **Do NOT use Python `hashlib.sha3_256` to recompute** — it implements NIST SHA3, not Ethereum Keccak-256. See kb/protocols/dex.md#python-sha3-wrong-selector.

---

### 2g. Token Addresses (BSC)

Built-in token map for common BSC V2 tokens (resolve at startup):

| Symbol | Address (BSC) | Decimals |
|--------|--------------|----------|
| WBNB | `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` | 18 |
| CAKE | `0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82` | 18 |
| USDT | `0x55d398326f99059fF775485246999027B3197955` | 18 |
| USDC | `0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d` | 18 |
| BUSD | `0xe9e7CEA3DedcA5984780Bafc599bD69ADd087D56` | 18 |
| ETH (BSC) | `0x2170Ed0880ac9A755fd29B2688956BD959F933F8` | 18 |
| BTCB | `0x7130d2A12B9BCbFAe4f2634d864A1Ee1Ce3Ead9c` | 18 |

For Base (8453), use `onchainos token search` to resolve addresses dynamically.

---

## 3. User Scenarios

### Scenario 1: Swap Tokens (Happy Path — Token → Token on BSC)

**User says:** "Swap 100 USDT for CAKE on PancakeSwap V2 on BSC"

**Agent actions:**

1. **[Chain-off] Resolve token addresses**
   - `tokenIn = USDT = 0x55d398326f99059fF775485246999027B3197955` (18 dec, from built-in map)
   - `tokenOut = CAKE = 0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82` (18 dec, from built-in map)
   - `amountIn = 100 * 10^18 = 100000000000000000000`

2. **[Chain-off] Quote via getAmountsOut**
   - `eth_call` to Router02 `0xd06ca61f` with `amountIn=100e18`, `path=[USDT, WBNB, CAKE]` (multi-hop via WBNB) or `path=[USDT, CAKE]` if direct pair exists.
   - First check: `eth_call` to Factory `getPair(USDT, CAKE)` — if returns non-zero, use direct path. Otherwise use `[USDT, WBNB, CAKE]`.
   - Parse `amounts[last]` as expected CAKE output.

3. **Display quote** to user:
   - "You will receive approximately X.XX CAKE for 100 USDT. Price impact: ~Y%. Swap fee: 0.25%."

4. **[Chain-off] Security scan** via `onchainos security token-scan --address <CAKE> --chain 56`.

5. **[Chain-off] Check USDT allowance** — `allowance(wallet, Router02)` via `0xdd62ed3e`.

6. **If allowance < amountIn — [Chain-on] Approve Router02**:
   - Ask user to confirm approval before proceeding.
   - `onchainos wallet contract-call --chain 56 --to <USDT> --input-data 0x095ea7b3<router02_padded><uint256_max> --force`
   - Wait 3 seconds for nonce to clear.

7. **[Chain-on] Execute swap**:
   - Build calldata: `swapExactTokensForTokens` selector `0x38ed1739` with `amountIn`, `amountOutMin = amounts[last] * 0.995` (0.5% slippage), path, `to = wallet`, `deadline = now + 300s`.
   - Ask user to confirm the swap before proceeding.
   - `onchainos wallet contract-call --chain 56 --to 0x10ED43C718714eb63d5aA57B78B54704E256024E --input-data <calldata> --force`

8. **Display result**: Parse `txHash` from `.data.txHash`, show BscScan link `https://bscscan.com/tx/<hash>`.

---

### Scenario 2: Add Liquidity (Token + BNB on BSC)

**User says:** "Add liquidity to CAKE/BNB pool on PancakeSwap V2 with 10 CAKE and 0.05 BNB"

**Agent actions:**

1. **[Chain-off] Resolve addresses**
   - `CAKE = 0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82`
   - BNB is native; use `addLiquidityETH` variant.
   - `amountTokenDesired = 10 * 10^18`
   - `amountETHDesired = 0.05 * 10^18 = 50000000000000000`

2. **[Chain-off] Check current pair ratio**
   - `getPair(CAKE, WBNB)` → pair address.
   - `getReserves()` + `token0()` on pair → current reserve ratio.
   - Verify user's ratio is within 1% of current ratio; warn if it deviates significantly (user may receive less liquidity than expected).

3. **[Chain-off] Check CAKE balance and allowance**
   - `balanceOf(wallet)` on CAKE — verify user has ≥ 10 CAKE.
   - `allowance(wallet, Router02)` on CAKE — check if approval needed.

4. **[Chain-on] Approve Router02 for CAKE (if needed)**:
   - Ask user to confirm approval.
   - `onchainos wallet contract-call --chain 56 --to <CAKE> --input-data 0x095ea7b3<router02_padded><uint256_max> --force`
   - Wait 5 seconds.

5. **Display summary**: "Adding 10 CAKE + 0.05 BNB to CAKE/BNB pool. You will receive LP tokens representing your share."

6. **[Chain-on] Add liquidity**:
   - Build calldata: `addLiquidityETH` selector `0xf305d719` with `token=CAKE`, `amountTokenDesired`, `amountTokenMin = amountTokenDesired * 0.99`, `amountETHMin = amountETHDesired * 0.99`, `to = wallet`, `deadline = now + 300s`.
   - Ask user to confirm before proceeding.
   - `onchainos wallet contract-call --chain 56 --to 0x10ED43C718714eb63d5aA57B78B54704E256024E --input-data <calldata> --amt 50000000000000000 --force`

7. **Display result**: Show txHash + BscScan link. Inform user: "LP tokens have been sent to your wallet. Use `remove-liquidity` to withdraw later."

---

### Scenario 3: Remove Liquidity (Receive Tokens)

**User says:** "Remove my CAKE/USDT liquidity on PancakeSwap V2"

**Agent actions:**

1. **[Chain-off] Resolve pair**
   - `getPair(CAKE, USDT)` → `pair_address`.
   - If returns `address(0)`: "No CAKE/USDT V2 pair found on BSC."

2. **[Chain-off] Check LP balance**
   - `balanceOf(wallet)` on `pair_address` → `lp_balance`.
   - `totalSupply()` on `pair_address` → `total_supply`.
   - If `lp_balance == 0`: "You have no LP tokens for this pair."

3. **[Chain-off] Compute expected withdrawal**
   - `getReserves()` + `token0()` on pair → `reserve0`, `reserve1`.
   - `share = lp_balance / total_supply`
   - `tokenA_out = reserveA * share`, `tokenB_out = reserveB * share` (map reserve0/reserve1 to CAKE/USDT via token0()).
   - `amountAMin = tokenA_out * 0.99`, `amountBMin = tokenB_out * 0.99`.

4. **Display to user**: "Removing X LP tokens (~Y CAKE + Z USDT). Confirm?"

5. **[Chain-on] Approve LP tokens to Router02**:
   - Ask user to confirm approval.
   - `onchainos wallet contract-call --chain 56 --to <pair_address> --input-data 0x095ea7b3<router02_padded><lp_balance_hex> --force`
   - Wait 5 seconds.

6. **[Chain-on] Remove liquidity**:
   - Build calldata: `removeLiquidity` selector `0xbaa2abde` with `tokenA=CAKE`, `tokenB=USDT`, `liquidity=lp_balance`, `amountAMin`, `amountBMin`, `to=wallet`, `deadline=now+300s`.
   - Ask user to confirm before proceeding.
   - `onchainos wallet contract-call --chain 56 --to 0x10ED43C718714eb63d5aA57B78B54704E256024E --input-data <calldata> --force`

7. **Display result**: Show txHash + BscScan link. "CAKE and USDT have been returned to your wallet."

---

### Scenario 4: Get Quote Only (Read-Only)

**User says:** "How much CAKE would I get for 0.1 BNB on PancakeSwap V2?"

**Agent actions:**

1. **[Chain-off] Build getAmountsOut call**
   - `amountIn = 0.1 * 10^18`
   - `path = [WBNB, CAKE]`
   - `eth_call` Router02 `0xd06ca61f`.

2. **Parse result** — `amounts[1]` is CAKE output.

3. **Display**: "For 0.1 BNB, you would receive approximately X.XX CAKE on PancakeSwap V2 (BSC). Price: Y CAKE/BNB. Swap fee: 0.25%."
   - No confirmation needed — read-only operation.

---

### Scenario 5: Check Pool Info / Reserves (Risk-Aware)

**User says:** "What are the reserves in the CAKE/BNB pool on PancakeSwap V2?"

**Agent actions:**

1. **[Chain-off] Get pair address** — `getPair(CAKE, WBNB)` on Factory.
2. **[Chain-off] Get token ordering** — `token0()` and `token1()` on pair.
3. **[Chain-off] Get reserves** — `getReserves()` on pair → `reserve0`, `reserve1`.
4. **[Chain-off] Get decimals** — `decimals()` (`0x313ce567`) on each token.
5. **Display**: Pool address, reserve0 (formatted), reserve1 (formatted), current price ratio, implied price of CAKE in BNB.

---

## 4. External API Dependencies

| API | Endpoint | Purpose | Auth |
|-----|----------|---------|------|
| BSC JSON-RPC | `https://bsc-rpc.publicnode.com` | All `eth_call` reads on BSC | None (public) |
| Base JSON-RPC | `https://base-rpc.publicnode.com` | All `eth_call` reads on Base | None (public) |
| PancakeSwap Info API (optional) | `https://api.pancakeswap.info/api/v2/tokens` | Token metadata enrichment (read-only) | None (public) |

> **RPC selection rationale:**
> - BSC: `bsc-rpc.publicnode.com` — avoids `bsc-dataseed.binance.org` TLS handshake failures in sandbox (see kb/onchainos/gotchas.md#bsc-rpc-tls).
> - Base: `base-rpc.publicnode.com` — avoids `mainnet.base.org` rate limits under multi-call DEX load (see kb/onchainos/gotchas.md#base-rpc-rate-limit).
> - Do NOT use `cloudflare-eth.com` (see kb/onchainos/gotchas.md#cloudflare-eth-bad).

---

## 5. Configuration Parameters

| Parameter | Type | Default | Notes |
|-----------|------|---------|-------|
| `chain_id` | u64 | `56` | Chain to operate on. Supported: 56 (BSC), 8453 (Base) |
| `slippage_bps` | u64 | `50` | Slippage tolerance in basis points (50 = 0.5%) |
| `deadline_secs` | u64 | `300` | Swap/LP deadline: seconds from now (5 minutes) |
| `dry_run` | bool | `false` | Skip broadcast; print calldata only. Handle in wrapper layer — do NOT pass `--dry-run` to onchainos CLI (unsupported flag). Use zero address `0x000...000` as recipient placeholder in dry-run ABI encoding. |
| `rpc_url` | String | per-chain default | Override default RPC endpoint |
| `gas_limit` | Option\<u64\> | None | If set, pass `--gas-limit` to `wallet contract-call` |

---

## 6. Key Implementation Notes for Developer Agent

### 6a. V2 vs V3 Distinction

This plugin is **strictly V2** (xyk AMM, Router02, ERC-20 LP tokens). It must NOT overlap with the V3 plugin (PR #82). Key differences:
- No fee tier selection (V2 has fixed 0.25% fee).
- No NFT positions, no `NonfungiblePositionManager`.
- LP tokens are ERC-20 (not EIP-721 NFTs).
- `removeLiquidity` burns LP tokens; no `decreaseLiquidity` + `collect` two-step.
- No `QuoterV2` — use Router02's `getAmountsOut` view function directly.
- No tick math.

### 6b. Path Routing

V2 does not have an on-chain smart router. The plugin must determine the swap path:
- For common pairs (e.g. CAKE/BNB, USDT/BNB), use direct two-hop path.
- For token→token where no direct pair exists, route via WBNB: `[tokenIn, WBNB, tokenOut]`.
- Before accepting a path, call `getPair` for each hop to verify the pair exists. If `getPair` returns `address(0)`, the hop does not exist.
- If no valid path found, return an error: "No V2 liquidity path found for this pair."

### 6c. Token Ordering in Pairs

V2 pairs always store tokens as `token0` (lower address) and `token1` (higher address). When parsing reserves, always call `token0()` first to map reserves correctly:
```rust
let t0 = pair_token0(pair_addr, rpc_url).await?; // 0x0dfe1681
let (r0, r1, _) = pair_get_reserves(pair_addr, rpc_url).await?; // 0x0902f1ac
let (reserve_in, reserve_out) = if token_in.to_lowercase() == t0.to_lowercase() {
    (r0, r1)
} else {
    (r1, r0)
};
```

### 6d. LP Token Amount for Full Removal

When the user wants to remove all liquidity, use their exact LP balance (from `balanceOf`), not `uint256::MAX`. The V2 pair does not accept `type(uint256).max` for `liquidity` — it must be the exact amount to burn.

### 6e. Approve-Then-Swap Delay

Add 3-second sleep between approve and swap, 5-second sleep between approve and addLiquidity/removeLiquidity (per kb/protocols/dex.md#lp-nonce-delay).

### 6f. Recipient Address

Always fetch the real wallet address via `onchainos wallet balance --chain 56 --output json` → `.data.address`. For dry-run, use `0x0000000000000000000000000000000000000000` as placeholder (per kb/protocols/dex.md#dry-run-placeholder).

### 6g. `--force` Flag Required

All `wallet contract-call` invocations for DEX operations must include `--force`. Without it, transactions return `txHash: "pending"` and never broadcast (per KNOWLEDGE_HUB.md gotcha: "txHash: pending on DEX swap, no broadcast").

### 6h. monorepo source_dir

In `plugin.yaml`:
```yaml
build:
  source_repo: skylavis-sky/onchainos-plugins
  source_dir: pancakeswap-v2
```

---

## 7. Submission Metadata

| Field | Value |
|-------|-------|
| plugin_store_name | `pancakeswap-v2` |
| binary_name | `pancakeswap-v2` |
| source_repo | `skylavis-sky/onchainos-plugins` |
| source_dir | `pancakeswap-v2` |
| category | `defi-protocol` |
| license | MIT |
| priority_rank | 17 |
| pr_reference | PR #82 (PancakeSwap V3, for contrast — do not duplicate) |
