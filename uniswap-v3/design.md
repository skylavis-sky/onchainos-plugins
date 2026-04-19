# Uniswap V3 — Plugin Design Document

> Complete interface design for the `uniswap-v3` plugin. This document is the authoritative reference for the Developer Agent.
>
> **Scope:** Uniswap V3 — the concentrated liquidity market maker (CLMM). Positions are NFTs via NonfungiblePositionManager. Supports four fee tiers (100/500/3000/10000 bps). Swaps route through SwapRouter02. This document covers all five target chains: Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137).

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `uniswap-v3` |
| dapp_name | Uniswap V3 |
| dapp_repo | https://github.com/Uniswap/v3-periphery |
| dapp_alias | uniswap, uni v3, uniswap concentrated liquidity, univ3 |
| one_liner | Swap tokens and manage concentrated liquidity positions on Uniswap V3 — the leading CLMM across Ethereum and L2s |
| category | defi-protocol |
| tags | dex, swap, liquidity, clmm, evm, uniswap, v3, amm |
| target_chains | ethereum (1), arbitrum (42161), base (8453), optimism (10), polygon (137) |
| target_protocols | Uniswap V3 |
| version | 0.1.0 |
| integration_path | Direct on-chain (ABI calldata) via SwapRouter02 + NonfungiblePositionManager + eth_call reads |

---

## 1. Feasibility Research

### 1a. Feasibility Table

| Check Item | Result |
|------------|--------|
| Rust SDK? | **Community only.** No official Uniswap Rust SDK. Community crates exist: `uniswap-v3-sdk` (crates.io, `shuhuiluo/uniswap-v3-sdk-rs`) and `uniswap-rs` (`DaniPopes/uniswap-rs`). These provide tick math and price encoding helpers but are not production-hardened and have no onchainos integration. |
| SDK supports which stacks? | Official SDK is TypeScript only: `@uniswap/v3-sdk`. No official Rust, Python, or Go bindings. |
| REST API? | **No swap/LP API.** Uniswap provides a free REST API for market data (`https://api.uniswap.org/`) and subgraph queries, but no API for swap execution. All write operations require direct contract calls. |
| Official Skill? | **Yes (TypeScript, not applicable).** `uniswap-ai` SDK offers agent-oriented wrappers, but they are TypeScript-only and not compatible with onchainos Rust plugin format. |
| Open-source community Skill (onchainos)? | **No standalone V3 onchainos plugin found** (2026-04-17 search). No existing plugin in `okx/onchainos-skills` or community repos. |
| Supported chains? | Deployed on 30+ EVM chains. This plugin targets Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137). |
| Requires onchainos broadcast? | **Yes.** All swap, add-liquidity, remove-liquidity, and approve operations are on-chain writes via `onchainos wallet contract-call`. Read operations (get-quote, get-pools, get-positions) use off-chain `eth_call` over JSON-RPC — no broadcast required. |

### 1b. Integration Path Decision

**Path: Direct On-Chain (ABI calldata) — no SDK**

Rationale:
- No suitable Rust SDK. Community crates have incomplete coverage and are not maintained to production quality.
- Uniswap V3 contract ABIs are fully documented, stable, and immutable. All required functions (`exactInputSingle`, `mint`, `decreaseLiquidity`, etc.) have canonical ABI signatures.
- All read operations (`quoteExactInputSingle`, `positions`, `getPool`) are single `eth_call`s — no subgraph required for core functionality.
- The PancakeSwap V3 plugin (same architecture as Uniswap V3) already established the calldata-construction + onchainos pattern in this codebase.
- **Decision:** Construct ABI-encoded calldata directly in Rust, submit via `onchainos wallet contract-call --force`. Off-chain reads use JSON-RPC `eth_call`. Tick math for position bounds will be implemented in-plugin (tick spacing is deterministic from fee tier).

---

## 2. Interface Mapping

### 2a. Contract Addresses by Chain

> **Address sources:** Confirmed from official Uniswap V3 governance deployment list (gov.uniswap.org/t/official-uniswap-v3-deployments-list/24323) and `Uniswap/v3-periphery` deploys.md on GitHub. Base has different Factory and periphery addresses because it was deployed after the original cross-chain batch.

#### Ethereum (Chain ID: 1)

| Contract | Address | Source |
|----------|---------|--------|
| UniswapV3Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | v3-periphery deploys.md |
| SwapRouter02 | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | Governance deployment list |
| SwapRouter (v1, legacy) | `0xE592427A0AEce92De3Edee1F18E0157C05861564` | v3-periphery deploys.md |
| QuoterV2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | Governance deployment list |
| NonfungiblePositionManager | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | v3-periphery deploys.md |
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | Canonical Ethereum WETH |

#### Arbitrum (Chain ID: 42161)

| Contract | Address | Source |
|----------|---------|--------|
| UniswapV3Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | Governance deployment list |
| SwapRouter02 | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | Governance deployment list |
| QuoterV2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | Governance deployment list |
| NonfungiblePositionManager | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | Governance deployment list |
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | Arbitrum canonical WETH |

#### Base (Chain ID: 8453)

| Contract | Address | Source |
|----------|---------|--------|
| UniswapV3Factory | `0x33128a8fC17869897dcE68Ed026d694621f6FDfD` | Governance deployment list |
| SwapRouter02 | `0x2626664c2603336E57B271c5C0b26F421741e481` | Governance deployment list |
| QuoterV2 | `0x3d4e44Eb1374240CE5F1B871ab261CD16335B76a` | Governance deployment list |
| NonfungiblePositionManager | `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` | Governance deployment list |
| WETH | `0x4200000000000000000000000000000000000006` | Base canonical WETH |

#### Optimism (Chain ID: 10)

| Contract | Address | Source |
|----------|---------|--------|
| UniswapV3Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | Governance deployment list |
| SwapRouter02 | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | Governance deployment list |
| QuoterV2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | Governance deployment list |
| NonfungiblePositionManager | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | Governance deployment list |
| WETH | `0x4200000000000000000000000000000000000006` | Optimism canonical WETH |

#### Polygon (Chain ID: 137)

| Contract | Address | Source |
|----------|---------|--------|
| UniswapV3Factory | `0x1F98431c8aD98523631AE4a59f267346ea31F984` | Governance deployment list |
| SwapRouter02 | `0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45` | Governance deployment list |
| QuoterV2 | `0x61fFE014bA17989E743c5F6cB21bF9697530B21e` | Governance deployment list |
| NonfungiblePositionManager | `0xC36442b4a4522E871399CD717aBDD847Ab11FE88` | Governance deployment list |
| WMATIC | `0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270` | Polygon canonical WMATIC |

> **Important:** Base has different contract addresses for Factory, SwapRouter02, QuoterV2, and NonfungiblePositionManager compared to the other four chains. Do not use Ethereum/Arbitrum/Optimism/Polygon addresses on Base.

> **SwapRouter02 vs SwapRouter (v1):** This plugin targets **SwapRouter02** (`0x68b3465...`) as the primary swap router. SwapRouter02 has a slightly different `ExactInputSingleParams` struct — it omits the `deadline` field (relying on `msg.sender` as origin and block-level deadline via multicall). SwapRouter v1 (`0xE592427...`) includes `deadline` in the struct. See section 2d for exact struct layouts.

### 2b. Fee Tiers and Tick Spacing

Uniswap V3 supports four fee tiers. Each maps to a fixed tick spacing used for position bounds:

| Fee Tier | uint24 Value | Tick Spacing | Typical Use Case |
|----------|-------------|--------------|-----------------|
| 0.01% | 100 | 1 | Stable-stable pairs (USDC/USDT, USDC/DAI) |
| 0.05% | 500 | 10 | Major liquid pairs (WETH/USDC, WETH/DAI) |
| 0.30% | 3000 | 60 | Standard pairs (most ERC-20 tokens) |
| 1.00% | 10000 | 200 | Exotic / long-tail tokens |

> **Fee tier validation (critical):** Before accepting a QuoterV2 quote for a fee tier, always verify the pool exists via `Factory.getPool(tokenA, tokenB, fee)`. QuoterV2 may return a plausible-looking quote for pools with zero in-range liquidity. If `getPool` returns `address(0)`, skip that fee tier. See kb/protocols/dex.md#quoter-zero-liquidity.

> **Auto fee tier selection:** When the user does not specify a fee tier, iterate over all four tiers (100, 500, 3000, 10000), check which pools exist, query QuoterV2 for each, and select the tier with the best output amount.

### 2c. Operations Table

| # | Operation | Type | Description |
|---|-----------|------|-------------|
| 1 | `get-quote` | Chain-off (eth_call) | Get expected swap output via QuoterV2.quoteExactInputSingle |
| 2 | `swap` | Chain-on | Execute swap via SwapRouter02.exactInputSingle (single-hop) |
| 3 | `get-pools` | Chain-off (eth_call) | Look up pool address and basic info via Factory.getPool |
| 4 | `get-positions` | Chain-off (eth_call) | Query NonfungiblePositionManager.positions for a token ID |
| 5 | `add-liquidity` | Chain-on | Mint a new V3 position via NonfungiblePositionManager.mint |
| 6 | `remove-liquidity` | Chain-on | Decrease + collect + burn via NonfungiblePositionManager |
| 7 | `approve` | Chain-on | ERC-20 approve (router or NFPM as spender) |

---

### 2d. Off-Chain Read Operations

All read operations use JSON-RPC `eth_call` — no onchainos broadcast needed.

---

#### Operation 1: `get-quote` — QuoterV2.quoteExactInputSingle

**Contract:** QuoterV2  
**Function:** `quoteExactInputSingle(QuoteExactInputSingleParams memory params)`  
**Selector:** `0xc6a5026a` *(keccak256("quoteExactInputSingle((address,address,uint256,uint24,uint160))") → c6a5026a)*

**QuoteExactInputSingleParams struct (ABI tuple order):**
```
(address tokenIn, address tokenOut, uint256 amountIn, uint24 fee, uint160 sqrtPriceLimitX96)
```

**eth_call calldata construction:**
```
selector:           c6a5026a
offset to tuple:    0000...0020  (32 — tuple starts immediately at offset 32)
tokenIn:            000000000000000000000000<tokenIn_no_0x>    (address, padded)
tokenOut:           000000000000000000000000<tokenOut_no_0x>   (address, padded)
amountIn:           <amountIn, uint256, 32 bytes>
fee:                <fee_tier, uint24 right-aligned in uint256 slot>
sqrtPriceLimitX96:  0000...0000  (zero = no price limit)
```

**Returns:**
```
uint256 amountOut
uint160 sqrtPriceX96After
uint32  initializedTicksCrossed
uint256 gasEstimate
```
Parse `amountOut` from return bytes offset 0 (first 32 bytes).

**Workflow:**
1. For each candidate fee tier (100, 500, 3000, 10000):
   a. Call `Factory.getPool(tokenIn, tokenOut, fee)` — skip if returns `address(0)`.
   b. Call `QuoterV2.quoteExactInputSingle(...)` for validated pools only.
2. Select fee tier with highest `amountOut`.
3. Return best quote plus fee tier used.

**Display to user:** "For X tokenIn, you will receive approximately Y tokenOut at the 0.3% fee tier."

---

#### Operation 3: `get-pools` — Factory.getPool

**Contract:** UniswapV3Factory  
**Function:** `getPool(address tokenA, address tokenB, uint24 fee)`  
**Selector:** `0x1698ee82` *(keccak256("getPool(address,address,uint24)") → 1698ee82)*

**eth_call calldata:**
```
1698ee82
000000000000000000000000<tokenA_no_0x>
000000000000000000000000<tokenB_no_0x>
<fee, uint24 right-aligned in 32-byte slot>
```

**Returns:** `address` — pool contract. Returns `address(0)` if pool not deployed.

**Usage:** Call for all four fee tiers to enumerate which pools exist for a pair.

---

#### Operation 4: `get-positions` — NonfungiblePositionManager.positions

**Contract:** NonfungiblePositionManager  
**Function:** `positions(uint256 tokenId)`  
**Selector:** `0x99fbab88` *(keccak256("positions(uint256)") → 99fbab88)*

**eth_call calldata:**
```
99fbab88
<tokenId, uint256, 32 bytes>
```

**Returns (12 values, 32 bytes each):**
```
uint96  nonce
address operator
address token0
address token1
uint24  fee
int24   tickLower       ← ABI int256, decode as i32 (extract last 8 hex chars)
int24   tickUpper       ← ABI int256, decode as i32 (extract last 8 hex chars)
uint128 liquidity
uint256 feeGrowthInside0LastX128
uint256 feeGrowthInside1LastX128
uint128 tokensOwed0
uint128 tokensOwed1
```

> **Tick decoding (critical):** `tickLower` and `tickUpper` are ABI-encoded as 32-byte int256 values. Extract the last 8 hex characters from each 32-byte word and cast as `u32 as i32` to get the correct signed tick value. See kb/protocols/dex.md#tick-decoding.
>
> ```rust
> fn decode_tick(hex_word: &str) -> i32 {
>     let clean = hex_word.trim_start_matches("0x");
>     let last8 = &clean[clean.len().saturating_sub(8)..];
>     u32::from_str_radix(last8, 16).unwrap_or(0) as i32
> }
> ```

**Display:** token0/token1 addresses, fee tier, tick range, liquidity amount, uncollected fees (tokensOwed0/tokensOwed1 in human-readable units).

---

### 2e. On-Chain Write Operations

All on-chain writes:
```bash
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <CONTRACT_ADDRESS> \
  --input-data <HEX_CALLDATA> \
  [--amt <WEI_VALUE>] \
  --force
```

The `--force` flag is **required** on all DEX write operations — without it, `txHash` stays `"pending"` and never broadcasts. See kb/onchainos/gotchas.md#exit-code-2.

---

#### Operation 7: `approve` — ERC-20 approve

**Contract:** tokenIn (ERC-20)  
**Function:** `approve(address spender, uint256 amount)`  
**Selector:** `0x095ea7b3` *(keccak256("approve(address,uint256)") → 095ea7b3)*

**Pre-check — allowance (skip approve if sufficient):**
**Selector:** `0xdd62ed3e` *(keccak256("allowance(address,address)") → dd62ed3e)*

```
dd62ed3e
000000000000000000000000<owner_no_0x>
000000000000000000000000<spender_no_0x>
```

If current allowance >= amountIn, skip the approve tx.

**Approve calldata (if needed):**
```
095ea7b3
000000000000000000000000<spender_no_0x>   # SwapRouter02 or NFPM address
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  # uint256.max
```

> For swap: spender = SwapRouter02  
> For add-liquidity: spender = NonfungiblePositionManager  
> For remove-liquidity: no token approve needed (NFPM already owns the position NFT)

**After approve → use `wait_for_tx` receipt polling before main op.** Do NOT use a fixed sleep. See kb/onchainos/gotchas.md#approve-race-condition.

---

#### Operation 2: `swap` — SwapRouter02.exactInputSingle

> **This is the SwapRouter02 struct** (no `deadline` field — differs from SwapRouter v1):

**ExactInputSingleParams struct (SwapRouter02 / IV3SwapRouter):**
```
struct ExactInputSingleParams {
    address tokenIn;       // slot 0
    address tokenOut;      // slot 1
    uint24  fee;           // slot 2
    address recipient;     // slot 3
    uint256 amountIn;      // slot 4
    uint256 amountOutMinimum; // slot 5
    uint160 sqrtPriceLimitX96; // slot 6
}
```

**Canonical ABI type string (for selector):**
`exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))`

**Selector:** `0x04e45aaf` *(keccak256("exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))") → 04e45aaf)*

**Full interface mapping table:**

| Operation | Contract | Function Signature (canonical) | Selector | ABI Param Order |
|-----------|----------|-------------------------------|----------|-----------------|
| swap (single-hop) | SwapRouter02 | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))` | `0x04e45aaf` | tokenIn, tokenOut, fee, recipient, amountIn, amountOutMinimum, sqrtPriceLimitX96 |

**Calldata layout (single-hop, Token → Token):**
```
04e45aaf
<offset to tuple = 0x20, 32 bytes>
000000000000000000000000<tokenIn_no_0x>      # address tokenIn
000000000000000000000000<tokenOut_no_0x>     # address tokenOut
<fee, uint24 right-aligned in 32 bytes>      # uint24 fee
000000000000000000000000<recipient_no_0x>    # address recipient (wallet address — NEVER zero unless dry-run)
<amountIn, uint256, 32 bytes>                # uint256 amountIn
<amountOutMinimum, uint256, 32 bytes>        # uint256 amountOutMinimum = quote * (1 - slippage_bps/10000)
0000000000000000000000000000000000000000000000000000000000000000  # uint160 sqrtPriceLimitX96 = 0 (no limit)
```

**Workflow:**
1. Resolve wallet address: `onchainos wallet balance --chain <ID> --output json` → `.data.address`.
   - For dry-run: use `0x0000000000000000000000000000000000000000` as recipient placeholder.
2. Call `get-quote` to find best fee tier and `amountOut`.
3. Verify pool exists via `Factory.getPool(tokenIn, tokenOut, fee)` — revert if `address(0)`.
4. Check allowance: `allowance(wallet, SwapRouter02)`. Skip approve if sufficient.
5. If approve needed: send approve tx, poll receipt via `wait_for_tx`, then proceed.
6. Build `exactInputSingle` calldata with `amountOutMinimum = amountOut * (1 - slippage_bps/10000)`.
7. Execute: `onchainos wallet contract-call --chain <ID> --to <SwapRouter02> --input-data <calldata> --force`.

**Multi-hop (exactInput):**  
Use `exactInput((bytes,address,uint256,uint256))` (selector `0xb858183f`) for multi-hop swaps where path = ABI-packed `[tokenIn, fee, hop1, fee, tokenOut]`.

---

#### Operation 5: `add-liquidity` — NonfungiblePositionManager.mint

**MintParams struct:**
```
struct MintParams {
    address token0;        // must be the lexicographically lower address
    address token1;        // must be the lexicographically higher address
    uint24  fee;
    int24   tickLower;
    int24   tickUpper;
    uint256 amount0Desired;
    uint256 amount1Desired;
    uint256 amount0Min;
    uint256 amount1Min;
    address recipient;
    uint256 deadline;
}
```

**Canonical ABI type string:**
`mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))`

**Full interface mapping table:**

| Operation | Contract | Function Signature (canonical) | Selector | ABI Param Order |
|-----------|----------|-------------------------------|----------|-----------------|
| add-liquidity (mint) | NonfungiblePositionManager | `mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` | `0x88316456` | token0, token1, fee, tickLower, tickUpper, amount0Desired, amount1Desired, amount0Min, amount1Min, recipient, deadline |

**Calldata layout:**
```
88316456
<offset to tuple = 0x20, 32 bytes>
000000000000000000000000<token0_no_0x>       # address token0 (lower address)
000000000000000000000000<token1_no_0x>       # address token1 (higher address)
<fee, uint24 right-aligned>                  # e.g. 0x...0BB8 for 3000
<tickLower, int24 — ABI int256 encoding>     # e.g. 0xFFFF...FFC4E (negative tick in two's complement)
<tickUpper, int24 — ABI int256 encoding>     # e.g. 0x000...3B9ACA00 (positive tick)
<amount0Desired, uint256>
<amount1Desired, uint256>
<amount0Min, uint256>                        # amount0Desired * (1 - slippage_bps/10000)
<amount1Min, uint256>                        # amount1Desired * (1 - slippage_bps/10000)
000000000000000000000000<recipient_no_0x>    # wallet address (or zero for dry-run)
<deadline, uint256>                          # block.timestamp + 300 (5 minutes)
```

**Tick encoding (int24 → ABI int256):**
```rust
fn encode_tick(tick: i32) -> String {
    if tick >= 0 {
        format!("{:064x}", tick as u64)
    } else {
        // Two's complement: cast i32 → i64 → u64, take lower 64 hex chars
        format!("{:064x}", (tick as i64) as u64)
    }
}
```

**Token ordering (critical):** V3 pools always use `token0 < token1` lexicographically. Sort token addresses before building calldata:
```rust
let (token0, token1) = if token_a.to_lowercase() < token_b.to_lowercase() {
    (token_a, token_b)
} else {
    (token_b, token_a)
};
```

**Default tick range (full range for fee tier):**
- Fee 100 (tick spacing 1): tickLower = -887272, tickUpper = 887272
- Fee 500 (tick spacing 10): tickLower = -887270, tickUpper = 887270
- Fee 3000 (tick spacing 60): tickLower = -887220, tickUpper = 887220
- Fee 10000 (tick spacing 200): tickLower = -887200, tickUpper = 887200

> Note: For concentrated positions, ticks must be multiples of the fee tier's tick spacing. Round user-supplied price bounds to nearest valid tick.

**Workflow:**
1. Resolve wallet address (or use zero address for dry-run).
2. Sort token addresses → token0 (lower), token1 (higher).
3. Verify pool exists via `Factory.getPool(token0, token1, fee)`.
4. Determine tick range (default: full range per tick spacing above; or user-supplied).
5. Check allowance for token0 → approve NFPM if needed → `wait_for_tx`.
6. Check allowance for token1 → approve NFPM if needed → `wait_for_tx`.
   - Wait 5 seconds between sequential on-chain calls (per kb/protocols/dex.md#lp-nonce-delay).
7. Build `mint` calldata.
8. Execute: `onchainos wallet contract-call --chain <ID> --to <NFPM> --input-data <calldata> --force`.
9. Parse emitted `tokenId` from the tx receipt's `Transfer` event (or display txHash for user to look up).

---

#### Operation 6: `remove-liquidity` — decreaseLiquidity + collect + burn

Remove liquidity is a three-step process:
1. `decreaseLiquidity` — withdraw liquidity from position, tokens become owed.
2. `collect` — transfer owed tokens + fees to recipient wallet.
3. `burn` — (optional) destroy the NFT if all liquidity removed.

**Step 1 — decreaseLiquidity:**

**DecreaseLiquidityParams struct:**
```
struct DecreaseLiquidityParams {
    uint256 tokenId;
    uint128 liquidity;
    uint256 amount0Min;
    uint256 amount1Min;
    uint256 deadline;
}
```

**Canonical signature:** `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))`

**Full interface mapping table (all three steps):**

| Operation | Contract | Function Signature (canonical) | Selector | ABI Param Order |
|-----------|----------|-------------------------------|----------|-----------------|
| remove-liquidity step 1 | NonfungiblePositionManager | `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` | `0x0c49ccbe` | tokenId, liquidity, amount0Min, amount1Min, deadline |
| remove-liquidity step 2 | NonfungiblePositionManager | `collect((uint256,address,uint128,uint128))` | `0xfc6f7865` | tokenId, recipient, amount0Max, amount1Max |
| remove-liquidity step 3 | NonfungiblePositionManager | `burn(uint256)` | `0x42966c68` | tokenId |

**Step 1 calldata (decreaseLiquidity):**
```
0c49ccbe
<offset to tuple = 0x20>
<tokenId, uint256>
<liquidity, uint128 right-aligned in 32 bytes>
<amount0Min, uint256>                           # 0 for max slippage, or quote * (1 - slippage)
<amount1Min, uint256>
<deadline, uint256>                             # block.timestamp + 300
```

**Step 2 calldata (collect):**
```
fc6f7865
<offset to tuple = 0x20>
<tokenId, uint256>
000000000000000000000000<recipient_no_0x>       # wallet address (or zero for dry-run)
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  # uint128 max = collect all
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  # uint128 max = collect all
```

> Wait 5 seconds between `decreaseLiquidity` and `collect` (per kb/protocols/dex.md#lp-nonce-delay).

**Step 3 calldata (burn):**
```
42966c68
<tokenId, uint256>
```

> Only call `burn` if `liquidity == 0` after `decreaseLiquidity`. If user is partially removing, skip burn.

**Workflow:**
1. Call `positions(tokenId)` to fetch current liquidity, token0, token1, fee, tokensOwed0, tokensOwed1.
2. Verify position belongs to wallet (check ownership via `ownerOf(tokenId)` — selector `0x6352211e`).
3. Calculate `amount0Min`, `amount1Min` from estimated output with slippage applied.
4. Execute `decreaseLiquidity`. `wait_for_tx` to confirm.
5. Wait 5 seconds, then execute `collect`.
6. If all liquidity removed: execute `burn`.
7. Return txHash and estimated amounts received.

---

### 2f. Complete Function Selector Summary

| Selector | Canonical Signature | Contract | Verified Method |
|----------|--------------------|----|------|
| `0x04e45aaf` | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))` | SwapRouter02 | keccak256 (eth_hash) |
| `0xb858183f` | `exactInput((bytes,address,uint256,uint256))` | SwapRouter02 | keccak256 (eth_hash) |
| `0x414bf389` | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))` | SwapRouter v1 (legacy, has deadline) | keccak256 (eth_hash) |
| `0xc04b8d59` | `exactInput((bytes,address,uint256,uint256,uint256))` | SwapRouter v1 (legacy, has deadline) | keccak256 (eth_hash) |
| `0xc6a5026a` | `quoteExactInputSingle((address,address,uint256,uint24,uint160))` | QuoterV2 | keccak256 (eth_hash) |
| `0xcdca1753` | `quoteExactInput(bytes,uint256)` | QuoterV2 | keccak256 (eth_hash) |
| `0x1698ee82` | `getPool(address,address,uint24)` | UniswapV3Factory | keccak256 (eth_hash) |
| `0x88316456` | `mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` | NonfungiblePositionManager | keccak256 (eth_hash) |
| `0x219f5d17` | `increaseLiquidity((uint256,uint256,uint256,uint256,uint256,uint256))` | NonfungiblePositionManager | keccak256 (eth_hash) |
| `0x0c49ccbe` | `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` | NonfungiblePositionManager | keccak256 (eth_hash) |
| `0xfc6f7865` | `collect((uint256,address,uint128,uint128))` | NonfungiblePositionManager | keccak256 (eth_hash) |
| `0x42966c68` | `burn(uint256)` | NonfungiblePositionManager | keccak256 (eth_hash) |
| `0x99fbab88` | `positions(uint256)` | NonfungiblePositionManager | keccak256 (eth_hash) |
| `0x095ea7b3` | `approve(address,uint256)` | ERC-20 | keccak256 (eth_hash) |
| `0xdd62ed3e` | `allowance(address,address)` | ERC-20 | keccak256 (eth_hash) |
| `0x70a08231` | `balanceOf(address)` | ERC-20 / NFPM | keccak256 (eth_hash) |
| `0x6352211e` | `ownerOf(uint256)` | ERC-721 (NFPM) | keccak256 (eth_hash) |

> All selectors computed using Python `eth_hash.auto.keccak` (correct Keccak-256). **Do NOT use Python `hashlib.sha3_256`** — it is NIST SHA3, not Ethereum Keccak-256. See kb/protocols/dex.md#python-sha3-wrong-selector.

---

### 2g. Token Addresses (Built-in Map)

#### Ethereum (1)
| Symbol | Address | Decimals |
|--------|---------|----------|
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | 18 |
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | 6 |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | 6 |
| DAI | `0x6B175474E89094C44Da98b954EedeAC495271d0F` | 18 |
| WBTC | `0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599` | 8 |
| UNI | `0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984` | 18 |

#### Arbitrum (42161)
| Symbol | Address | Decimals |
|--------|---------|----------|
| WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | 18 |
| USDC | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` | 6 |
| USDC.e | `0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8` | 6 |
| USDT | `0xFd086bC7CD5C481DCC9C85ebE478A1C0b69FCbb9` | 6 |
| ARB | `0x912CE59144191C1204E64559FE8253a0e49E6548` | 18 |

#### Base (8453)
| Symbol | Address | Decimals |
|--------|---------|----------|
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` | 6 |
| cbETH | `0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22` | 18 |

#### Optimism (10)
| Symbol | Address | Decimals |
|--------|---------|----------|
| WETH | `0x4200000000000000000000000000000000000006` | 18 |
| USDC | `0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85` | 6 |
| OP | `0x4200000000000000000000000000000000000042` | 18 |

#### Polygon (137)
| Symbol | Address | Decimals |
|--------|---------|----------|
| WMATIC | `0x0d500B1d8E8eF31E21C99d1Db9A6444d3ADf1270` | 18 |
| USDC | `0x2791Bca1f2de4661ED88A30C99A7a9449Aa84174` | 6 |
| USDC.e | `0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359` | 6 |
| WETH | `0x7ceB23fD6bC0adD59E62ac25578270cFf1b9f619` | 18 |

For unlisted tokens: use `onchainos token search --keyword <SYMBOL> --chain <CHAIN_ID>` to resolve addresses dynamically.

---

## 3. User Scenarios

### Scenario 1: Swap Tokens — Single Hop (Happy Path, Base)

**User says:** "Swap 100 USDC for WETH on Uniswap V3 on Base"

**Agent actions:**

1. **[Chain-off] Resolve addresses**
   - `tokenIn = USDC = 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` (6 decimals, built-in map)
   - `tokenOut = WETH = 0x4200000000000000000000000000000000000006` (18 decimals)
   - `amountIn = 100 * 10^6 = 100_000_000`
   - `chain_id = 8453`, `SwapRouter02 = 0x2626664c2603336E57B271c5C0b26F421741e481`

2. **[Chain-off] Find best pool and get quote**
   - Iterate fee tiers [100, 500, 3000, 10000]:
     - `Factory.getPool(USDC, WETH, fee)` on each (selector `0x1698ee82`)
     - Skip fee tiers where `getPool` returns `address(0)`
     - Call `QuoterV2.quoteExactInputSingle(...)` (selector `0xc6a5026a`) for each valid pool
   - Select fee tier with best `amountOut`

3. **Display quote:**
   "For 100 USDC, you will receive approximately 0.0XXXX WETH at the 0.05% fee tier on Uniswap V3 (Base)."

4. **[Chain-off] Resolve wallet address**
   - `onchainos wallet balance --chain 8453 --output json` → `.data.address`

5. **[Chain-off] Check USDC allowance**
   - `allowance(wallet, SwapRouter02)` via `0xdd62ed3e`
   - If allowance < amountIn, proceed with approve

6. **[Chain-on] Approve SwapRouter02 (if needed):**
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
     --input-data 0x095ea7b3\
   0000000000000000000000002626664c2603336E57B271c5C0b26F421741e481\
   ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
     --force
   ```
   - Poll `eth_getTransactionReceipt` until confirmed (`wait_for_tx`).

7. **[Chain-on] Execute swap:**
   - Build `exactInputSingle` calldata with selector `0x04e45aaf`
   - `amountOutMinimum = amountOut * (1 - slippage_bps / 10000)` (default: 50 bps = 0.5%)
   - `sqrtPriceLimitX96 = 0` (no price limit)
   - `recipient = wallet address`
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x2626664c2603336E57B271c5C0b26F421741e481 \
     --input-data <encoded_exactInputSingle_calldata> \
     --force
   ```

8. **Display result:**
   - Parse `txHash` from `.data.txHash`
   - Show: "Swap submitted! txHash: 0x... View on BaseScan: https://basescan.org/tx/<hash>"

---

### Scenario 2: Add Liquidity — New Concentrated Position (Ethereum)

**User says:** "Add liquidity to the USDC/WETH 0.05% pool on Uniswap V3 with 1000 USDC and 0.5 WETH, full range"

**Agent actions:**

1. **[Chain-off] Resolve addresses and sort tokens**
   - `USDC = 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` (6 dec)
   - `WETH = 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` (18 dec)
   - USDC < WETH lexicographically → `token0 = USDC`, `token1 = WETH`
   - `fee = 500` (0.05% tier), tick spacing = 10
   - Full range ticks: `tickLower = -887270`, `tickUpper = 887270` (multiples of tick spacing 10)

2. **[Chain-off] Verify pool exists**
   - `Factory.getPool(USDC, WETH, 500)` — must return non-zero address
   - If pool not deployed: "No USDC/WETH 0.05% pool exists on Ethereum. Try a different fee tier."

3. **[Chain-off] Resolve wallet address**
   - `onchainos wallet balance --chain 1 --output json` → `.data.address`

4. **[Chain-off] Check balances and allowances**
   - `USDC.balanceOf(wallet)` — verify ≥ 1000 USDC (1_000_000_000 raw)
   - `WETH.balanceOf(wallet)` — verify ≥ 0.5 WETH (500_000_000_000_000_000 raw)
   - `USDC.allowance(wallet, NFPM)` — check approval
   - `WETH.allowance(wallet, NFPM)` — check approval

5. **[Chain-on] Approve NFPM for USDC (if needed):**
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
     --input-data 0x095ea7b3\
   000000000000000000000000C36442b4a4522E871399CD717aBDD847Ab11FE88\
   ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
     --force
   ```
   - `wait_for_tx` until confirmed.

6. **[Chain-on] Approve NFPM for WETH (if needed):**
   - Similar calldata with WETH token address
   - `wait_for_tx` until confirmed.
   - Wait 5 seconds between USDC and WETH approve calls.

7. **Display pre-mint summary to user:**
   "Minting USDC/WETH 0.05% position on Uniswap V3:
   - token0: 1000 USDC (full amount)
   - token1: 0.5 WETH (full amount)
   - Tick range: [-887270, 887270] (full range)
   - Price range: full (0 → ∞)
   Confirm? (y/n)"

8. **[Chain-on] Execute mint:**
   - Build `mint` calldata with selector `0x88316456`
   - `amount0Desired = 1_000_000_000`, `amount1Desired = 500_000_000_000_000_000`
   - `amount0Min = amount0Desired * 0.995`, `amount1Min = amount1Desired * 0.995` (0.5% slippage)
   - `deadline = current_unix_timestamp + 300`
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xC36442b4a4522E871399CD717aBDD847Ab11FE88 \
     --input-data <mint_calldata> \
     --force
   ```

9. **Display result:**
   - txHash + Etherscan link: `https://etherscan.io/tx/<hash>`
   - "Your V3 position NFT has been minted. Use `get-positions --token-id <ID>` to view it."

---

### Scenario 3: Remove Liquidity — Close Position (Arbitrum)

**User says:** "Remove all liquidity from my Uniswap V3 position #12345 on Arbitrum"

**Agent actions:**

1. **[Chain-off] Fetch position details**
   - `positions(12345)` (selector `0x99fbab88`) on NFPM (`0xC36442b4a4522E871399CD717aBDD847Ab11FE88`, chain 42161)
   - Parse: token0, token1, fee, tickLower, tickUpper, liquidity, tokensOwed0, tokensOwed1
   - Verify `liquidity > 0` — if zero, position is already closed.

2. **[Chain-off] Verify ownership**
   - `ownerOf(12345)` (selector `0x6352211e`) — verify returned address equals wallet address.
   - If not owner: "You do not own position #12345."

3. **[Chain-off] Resolve wallet address**
   - `onchainos wallet balance --chain 42161 --output json` → `.data.address`

4. **Display current position to user:**
   "Position #12345 (USDC/WETH 0.3%, ticks [-69000, 69000]):
   - Liquidity: 1234567890
   - Uncollected fees: ~X USDC, ~Y WETH
   Removing all liquidity. Confirm? (y/n)"

5. **[Chain-on] Step 1 — decreaseLiquidity:**
   - Build calldata with selector `0x0c49ccbe`
   - `liquidity = full position liquidity`, `amount0Min = 0`, `amount1Min = 0` (accept any amount — user wants full removal)
   - `deadline = now + 300`
   ```bash
   onchainos wallet contract-call \
     --chain 42161 \
     --to 0xC36442b4a4522E871399CD717aBDD847Ab11FE88 \
     --input-data <decreaseLiquidity_calldata> \
     --force
   ```
   - `wait_for_tx` to confirm step 1.

6. **[Chain-on] Step 2 — collect (wait 5 seconds after step 1):**
   - Build calldata with selector `0xfc6f7865`
   - `recipient = wallet address`, `amount0Max = uint128::MAX`, `amount1Max = uint128::MAX`
   ```bash
   onchainos wallet contract-call \
     --chain 42161 \
     --to 0xC36442b4a4522E871399CD717aBDD847Ab11FE88 \
     --input-data <collect_calldata> \
     --force
   ```
   - `wait_for_tx` to confirm step 2.

7. **[Chain-on] Step 3 — burn (since all liquidity removed):**
   - Build calldata with selector `0x42966c68`
   - Param: `tokenId = 12345`
   ```bash
   onchainos wallet contract-call \
     --chain 42161 \
     --to 0xC36442b4a4522E871399CD717aBDD847Ab11FE88 \
     --input-data 0x42966c68\
   000000000000000000000000000000000000000000000000000000000000000 \
     --force
   ```

8. **Display result:**
   - "Position #12345 fully closed. Tokens returned to your wallet. View on Arbiscan: https://arbiscan.io/tx/<hash>"

---

### Scenario 4: Get Quote Only (Read-Only)

**User says:** "How much WETH would I get for 500 USDC on Uniswap V3 on Ethereum?"

**Agent actions:**

1. Resolve token addresses (USDC `0xA0b86991...`, WETH `0xC02aaA39...`). `amountIn = 500_000_000`.
2. Iterate fee tiers [100, 500, 3000, 10000]:
   - `Factory.getPool(USDC, WETH, fee)` — skip `address(0)` pools.
   - `QuoterV2.quoteExactInputSingle(...)` for valid pools.
3. Display best quote:
   "For 500 USDC → WETH on Uniswap V3 (Ethereum):
   - Best rate: 0.XXXXX WETH (0.05% fee tier)
   - Effective price: $XXXX.XX per ETH
   This is a read-only quote. No transaction was submitted."

No confirmation needed — read-only operation.

---

### Scenario 5: View Positions (Read-Only)

**User says:** "Show me my Uniswap V3 positions on Ethereum"

**Agent actions:**

1. **[Chain-off] Get wallet address** — `onchainos wallet balance --chain 1 --output json`.
2. **[Chain-off] Get NFT balance** — `NFPM.balanceOf(wallet)` (selector `0x70a08231`).
3. For each position token ID (enumerate via `tokenOfOwnerByIndex` — selector `0x2f745c59`):
   - Call `positions(tokenId)` to fetch details.
   - Decode ticks using `decode_tick` (last 8 hex chars as `u32 as i32`).
4. Display tabular summary: token ID, token pair, fee tier, tick range, liquidity, uncollected fees.

---

## 4. External API Dependencies

| API | Endpoint | Purpose | Auth |
|-----|----------|---------|------|
| Ethereum JSON-RPC | `https://ethereum.publicnode.com` | `eth_call` reads on Ethereum | None (public) |
| Arbitrum JSON-RPC | `https://arbitrum-one-rpc.publicnode.com` | `eth_call` reads on Arbitrum | None (public) |
| Base JSON-RPC | `https://base-rpc.publicnode.com` | `eth_call` reads on Base | None (public) |
| Optimism JSON-RPC | `https://optimism.publicnode.com` | `eth_call` reads on Optimism | None (public) |
| Polygon JSON-RPC | `https://polygon-bor-rpc.publicnode.com` | `eth_call` reads on Polygon | None (public) |

> **RPC selection rationale (from kb/onchainos/gotchas.md):**
> - Do NOT use `cloudflare-eth.com` — blocks sandbox IPs and causes `Internal error` on all eth_call.
> - Do NOT use `eth.llamarpc.com` or `polygon-rpc.com` — rate-limited and unreliable.
> - Do NOT use `mainnet.base.org` — rate-limits under multi-call DEX load (triggers `-32016` errors).
> - `publicnode.com` endpoints are preferred for all chains.

---

## 5. Configuration Parameters

| Parameter | Type | Default | Notes |
|-----------|------|---------|-------|
| `chain_id` | u64 | `1` | Chain to operate on. Supported: 1, 10, 137, 8453, 42161 |
| `slippage_bps` | u64 | `50` | Slippage tolerance in basis points (50 = 0.5%) |
| `deadline_secs` | u64 | `300` | Deadline for swap/LP operations: seconds from now (5 minutes) |
| `dry_run` | bool | `false` | Skip broadcast; print calldata only. Intercept in wrapper layer — do NOT pass to onchainos CLI (unsupported). Use `0x0000000000000000000000000000000000000000` as recipient placeholder. |
| `fee_tier` | Option\<u64\> | `None` | Fee tier override (100/500/3000/10000). If None, auto-select best. |
| `tick_lower` | Option\<i32\> | `None` | Lower tick for add-liquidity. If None, use full-range default. |
| `tick_upper` | Option\<i32\> | `None` | Upper tick for add-liquidity. If None, use full-range default. |
| `rpc_url` | Option\<String\> | per-chain default | Override default RPC endpoint |

---

## 6. Key Implementation Notes for Developer Agent

### 6a. SwapRouter02 vs SwapRouter v1

This plugin uses **SwapRouter02** (`0x68b3465...`). The key difference: SwapRouter02's `ExactInputSingleParams` struct does **not** include a `deadline` field (7 fields vs SwapRouter v1's 8 fields). The struct ordering is:
- SwapRouter02: `(address,address,uint24,address,uint256,uint256,uint160)` → selector `0x04e45aaf`
- SwapRouter v1: `(address,address,uint24,address,uint256,uint256,uint256,uint160)` → selector `0x414bf389`

Mixing these selectors will cause silent wrong-function dispatch. Always use `0x04e45aaf` for SwapRouter02.

### 6b. Recipient Must Never Be Zero Address in Production

Per kb/protocols/dex.md#zero-address-recipient: passing `address(0)` as `recipient` in `exactInputSingle` causes the router to send output tokens to the zero address, which reverts with `TF` (Too Few output tokens). Always resolve the real wallet address before building calldata. Only use the zero address in dry-run mode.

```rust
let recipient = if dry_run {
    "0x0000000000000000000000000000000000000000".to_string()
} else {
    get_wallet_address(chain_id)?
};
```

Resolve wallet address only *after* the dry-run early-return guard — wallet resolution will fail if the wallet is not logged in, and that should not block dry-run.

### 6c. QuoterV2 Pool Validation

Always call `Factory.getPool(tokenIn, tokenOut, fee)` before `QuoterV2.quoteExactInputSingle(...)`. QuoterV2 may return a non-zero quote even for pools with zero in-range liquidity. A pool with zero liquidity will revert on-chain with `TF`. See kb/protocols/dex.md#quoter-zero-liquidity.

### 6d. Approve-Before-Swap Race Condition

After the approve tx is submitted, do NOT proceed to the swap immediately with a fixed sleep. Use `wait_for_tx` receipt polling:
```rust
let approve_hash = wallet_contract_call(/* approve */, chain_id, false).await?;
wait_for_tx(&approve_hash, rpc_url).await?;
// Only now submit the swap
```
Fixed sleeps (3s, 5s) are unreliable under network congestion. See kb/onchainos/gotchas.md#approve-race-condition.

### 6e. Multi-Step LP Nonce Sequencing

For add-liquidity and remove-liquidity with multiple on-chain calls:
- Between approve calls for token0 and token1: `wait_for_tx`
- Between `decreaseLiquidity` and `collect`: `wait_for_tx` + 5-second sleep
- Between approve and mint: `wait_for_tx`

See kb/protocols/dex.md#lp-nonce-delay.

### 6f. Token Ordering in mint

V3 pairs use `token0 < token1` (lexicographic comparison of lowercase hex addresses). If the user specifies tokens in the wrong order, silently swap them before building calldata. The `mint` function will revert if token0 >= token1.

```rust
let (token0, token1) = if tokenA.to_lowercase() < tokenB.to_lowercase() {
    (tokenA, tokenB)
} else {
    (tokenB, tokenA)
};
```

### 6g. Tick Encoding for ABI

Ticks are `int24` in Solidity but ABI-encoded as 32-byte `int256` (sign-extended). Positive ticks encode straightforwardly. Negative ticks use two's complement sign extension:

```rust
fn encode_tick_to_abi_hex(tick: i32) -> String {
    if tick >= 0 {
        format!("{:064x}", tick as u64)
    } else {
        // Sign extend: cast i32 → i64 → reinterpret as u64 for hex formatting
        let extended = (tick as i64) as u64;
        format!("{:064x}", extended)
    }
}
```

Example: `tickLower = -887270` → `0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffeff7f16`.

### 6h. Tick Decoding from ABI Response

ABI returns `int256` (64 hex chars). To decode into `i32`:
```rust
fn decode_tick(hex_str: &str) -> i32 {
    let clean = hex_str.trim_start_matches("0x");
    let last8 = &clean[clean.len().saturating_sub(8)..];
    u32::from_str_radix(last8, 16).unwrap_or(0) as i32
}
```
See kb/protocols/dex.md#tick-decoding.

### 6i. Base Chain Address Difference

Base has different contract addresses than all other four chains. Always use a per-chain lookup map in `config.rs`:
```rust
pub fn get_swap_router02(chain_id: u64) -> &'static str {
    match chain_id {
        8453 => "0x2626664c2603336E57B271c5C0b26F421741e481",
        _    => "0x68b3465833fb72A70ecDF485E0e4C7bD8665Fc45",  // Ethereum, Arbitrum, Optimism, Polygon
    }
}
```
Same pattern for QuoterV2, Factory, and NFPM.

### 6j. reqwest Proxy in Sandbox

All HTTP calls via reqwest must use the proxy-aware client builder. See kb/onchainos/gotchas.md#reqwest-proxy:
```rust
pub fn build_client() -> reqwest::Client {
    let mut builder = reqwest::Client::builder();
    if let Ok(url) = std::env::var("HTTPS_PROXY").or_else(|_| std::env::var("https_proxy")) {
        if let Ok(proxy) = reqwest::Proxy::https(&url) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build().unwrap_or_default()
}
```

### 6k. `--force` Flag on All Write Calls

Every `onchainos wallet contract-call` for a write operation must include `--force`. Without it, the first call returns exit code 2 with `"confirming": true`, and the txHash never broadcasts. This is a safety confirmation gate — the `--force` flag bypasses it for automated plugin use. See kb/onchainos/gotchas.md#exit-code-2.

---

## 7. Submission Metadata

| Field | Value |
|-------|-------|
| plugin_store_name | `uniswap-v3` |
| binary_name | `uniswap-v3` |
| source_repo | `skylavis-sky/onchainos-plugins` |
| source_dir | `uniswap-v3` |
| category | `defi-protocol` |
| license | MIT |
| reference_plugin | `pancakeswap` (PR #82 — same CLMM architecture, different addresses/fee tiers) |
