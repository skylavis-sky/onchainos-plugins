# PancakeSwap AMM — Plugin Design Document

> Complete interface design for the `pancakeswap` plugin. This document is the authoritative reference for the Developer Agent.

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `pancakeswap` |
| dapp_name | PancakeSwap AMM |
| dapp_repo | https://github.com/pancakeswap/pancake-smart-contracts |
| dapp_alias | pancake, pcs, pancakeswap v3 |
| one_liner | Swap tokens and manage liquidity on PancakeSwap — the leading DEX on BSC and Base |
| category | defi-protocol |
| tags | dex, swap, liquidity, amm, pancakeswap, bsc, base, v3 |
| target_chains | bsc (56), base (8453) |
| target_protocols | PancakeSwap V3 AMM (concentrated liquidity) |
| version | 1.0.0 |
| integration_path | Direct on-chain (ABI calldata) + subgraph reads |

---

## 1. Feasibility Research

### 1a. Feasibility Table

| Check Item | Result |
|------------|--------|
| Rust SDK? | **No.** PancakeSwap has no Rust SDK. Official SDKs are TypeScript only: `@pancakeswap/sdk`, `@pancakeswap/smart-router`, `@pancakeswap/v3-sdk`. No Rust bindings exist. |
| SDK supports which stacks? | TypeScript/JavaScript only. `@pancakeswap/smart-router` (npm) handles routing; `@pancakeswap/sdk` handles token/pool math. No Go, Python, or Rust. |
| REST API? | **Partial.** PancakeSwap has no official REST swap/liquidity API. They expose a price/info API (`https://api.pancakeswap.info/api/v2/tokens`) but it is read-only market data. All swap/LP actions require direct contract calls. Bitquery provides a paid analytics API. |
| Official Skill? | **Indirect.** `pancakeswap-ai` (https://github.com/pancakeswap/pancakeswap-ai) provides TypeScript-based planning plugins (`pancakeswap-driver`, `pancakeswap-farming`). These generate deep links for UI confirmation, not autonomous on-chain execution. Not suitable for onchainos. |
| Open-source community Skill (onchainos)? | **Partial overlap.** `okx/onchainos-skills` (https://github.com/okx/onchainos-skills) includes `okx-defi-invest` which covers PancakeSwap farming/staking via the OKX API. However, it routes through OKX infrastructure, not direct contract calls, and does not expose raw calldata construction. No standalone PancakeSwap onchainos plugin found. |
| Supported chains? | BSC (56), Base (8453), Ethereum (1), Arbitrum (42161), zkSync Era (324), Linea (59144), opBNB (204), Scroll (534352). **This plugin targets BSC and Base only.** |
| Requires onchainos broadcast? | **Yes.** All swaps, LP minting, and liquidity changes are on-chain write operations. They require wallet signing and transaction broadcast via `onchainos wallet contract-call`. ERC-20 approvals are also on-chain via `contract-call`. Read operations (quotes, pool info, positions) are off-chain eth_call or subgraph queries. |

### 1b. Integration Path Decision

**Path: Direct On-Chain (ABI calldata)**

Rationale:
- No Rust SDK exists; TypeScript SDK cannot be used in Rust plugin.
- The official pancakeswap-ai skill only generates UI deep links, not autonomous on-chain execution.
- The onchainos-skills/okx-defi-invest covers PancakeSwap via OKX aggregation — useful for yield farming, but adds a dependency on OKX infrastructure and does not expose raw V3 contract interactions needed for arbitrary swaps and custom LP positions.
- PancakeSwap V3 is a close fork of Uniswap V3. All function signatures are well-documented and stable.
- **Decision:** Construct ABI-encoded calldata directly in Rust, submit via `onchainos wallet contract-call`. Off-chain reads use JSON-RPC `eth_call` to QuoterV2 and TheGraph subgraph for position data.

---

## 2. Interface Mapping

### 2a. Contract Addresses

#### BSC (Chain ID: 56)

| Contract | Address |
|----------|---------|
| SmartRouter (primary swap entry) | `0x13f4EA83D0bd40E75C8222255bc855a974568Dd4` |
| SwapRouter (V3 only, legacy) | `0x1b81D678ffb9C0263b24A97847620C99d213eB14` |
| PancakeV3Factory | `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` |
| PancakeV3PoolDeployer | `0x41ff9AA7e16B8B1a8a8dc4f0eFacd93D02d071c9` |
| NonfungiblePositionManager | `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364` |
| QuoterV2 | `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` |
| MixedRouteQuoterV1 | `0x678Aa4bF4E210cf2166753e054d5b7c31cc7fa86` |
| TickLens | `0x9a489505a00cE272eAa5e07Dba6491314CaE3796` |
| MasterChefV3 | `0x556B9306565093C855AEA9AE92A594704c2Cd59e` |
| WBNB | `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` |

#### Base (Chain ID: 8453)

| Contract | Address |
|----------|---------|
| SmartRouter (primary swap entry) | `0x678Aa4bF4E210cf2166753e054d5b7c31cc7fa86` |
| SwapRouter (V3 only, legacy) | `0x1b81D678ffb9C0263b24A97847620C99d213eB14` |
| PancakeV3Factory | `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` |
| NonfungiblePositionManager | `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364` |
| QuoterV2 | `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` |
| MasterChefV3 | `0xC6a2Db661D5a5690172d8eB0a7DEA2d3008665A3` |
| WETH (native wrap) | `0x4200000000000000000000000000000000000006` |
| V2 Factory (Base only) | `0x02a84c1b3BBD7401a5f7fa98a384EBC70bB5749E` |

> **Address source:** Confirmed from `pancakeswap/exchange-v3-subgraphs` config files (BSC: bsc.js, Base: base.js) and cross-referenced against BscScan / BaseScan verified contract labels. The Factory and NonfungiblePositionManager share the same address on BSC and Base (deterministic deployment via CREATE2). The SmartRouter differs per chain.

> **Runtime address resolution:** The SmartRouter and QuoterV2 addresses above are stable hardcoded deployments. However, the plugin SHOULD verify them at startup by calling `PancakeV3Factory.owner()` or confirming a known pool exists. Do not resolve SmartRouter from a registry — there is no on-chain registry for it; use the hardcoded addresses above.

### 2b. Fee Tiers

PancakeSwap V3 supports four fee tiers (same as Uniswap V3):

| Fee Tier | uint24 Value | Use Case |
|----------|-------------|----------|
| 0.01% | 100 | Stable-stable pairs |
| 0.05% | 500 | Major pairs (WBNB/USDT) |
| 0.25% | 2500 | Mid-tier pairs |
| 1% | 10000 | Exotic / long-tail tokens |

### 2c. Operations Table

| # | Operation | Type | Priority | Contract | Method |
|---|-----------|------|----------|----------|--------|
| 1 | ERC-20 approve for SmartRouter | On-chain | P0 | Token contract | `approve(address,uint256)` |
| 2 | Swap tokens — exact input single | On-chain | P0 | SmartRouter | `exactInputSingle(ExactInputSingleParams)` |
| 3 | Swap tokens — exact input multi-hop | On-chain | P0 | SmartRouter | `exactInput(ExactInputParams)` |
| 4 | Get swap quote (single pool) | Off-chain | P0 | QuoterV2 | `quoteExactInputSingle(QuoteExactInputSingleParams)` |
| 5 | Get swap quote (multi-hop) | Off-chain | P0 | QuoterV2 | `quoteExactInput(bytes,uint256)` |
| 6 | Get pool info / price | Off-chain | P0 | Subgraph / eth_call | Pool slot0, liquidity |
| 7 | Add liquidity (mint V3 position) | On-chain | P1 | NonfungiblePositionManager | `mint(MintParams)` |
| 8 | Increase liquidity | On-chain | P1 | NonfungiblePositionManager | `increaseLiquidity(IncreaseLiquidityParams)` |
| 9 | Remove liquidity (decrease) | On-chain | P1 | NonfungiblePositionManager | `decreaseLiquidity(DecreaseLiquidityParams)` |
| 10 | Collect fees / tokens | On-chain | P1 | NonfungiblePositionManager | `collect(CollectParams)` |
| 11 | View LP positions | Off-chain | P1 | NonfungiblePositionManager / Subgraph | `positions(tokenId)` / subgraph |
| 12 | Get token info | Off-chain | P0 | ERC-20 contract | `symbol()`, `decimals()`, `balanceOf()` |

---

### 2d. Off-Chain Read Operations

#### Op 4: QuoterV2 — quoteExactInputSingle

Used to get an expected output amount for a single-pool swap without executing it. Called via `eth_call` (read-only, no gas).

**Function signature:**
```solidity
function quoteExactInputSingle(QuoteExactInputSingleParams memory params)
    public
    returns (
        uint256 amountOut,
        uint160 sqrtPriceX96After,
        uint32 initializedTicksCrossed,
        uint256 gasEstimate
    )
```

**QuoteExactInputSingleParams struct:**
```solidity
struct QuoteExactInputSingleParams {
    address tokenIn;
    address tokenOut;
    uint256 amountIn;
    uint24 fee;
    uint160 sqrtPriceLimitX96;  // 0 = no limit
}
```

**ABI selector:** `0xc6a5026a` (keccak256("quoteExactInputSingle((address,address,uint256,uint24,uint160))"))

**eth_call encoding (Rust):**
```
calldata = selector + abi_encode_tuple(tokenIn, tokenOut, amountIn, fee, sqrtPriceLimitX96)
// All packed as a single tuple: (address, address, uint256, uint24, uint160)
// address = 32 bytes (left-padded)
// uint256 = 32 bytes
// uint24 = 32 bytes (left-padded, fits in uint256 slot)
// uint160 = 32 bytes (left-padded)
```

**Key return value:** `amountOut` (first 32 bytes of return data, as uint256)

---

#### Op 5: QuoterV2 — quoteExactInput (multi-hop)

**Function signature:**
```solidity
function quoteExactInput(bytes memory path, uint256 amountIn)
    public
    returns (
        uint256 amountOut,
        uint160[] memory sqrtPriceX96AfterList,
        uint32[] memory initializedTicksCrossedList,
        uint256 gasEstimate
    )
```

**ABI selector:** `0xcdca1753`

**Path encoding:**
```
path = abi.encodePacked(tokenIn, fee0, tokenMid, fee1, tokenOut)
// tokenIn: 20 bytes (address, no padding in path)
// fee0:    3 bytes (uint24, big-endian)
// tokenMid: 20 bytes
// fee1:    3 bytes
// tokenOut: 20 bytes
// Total for 2-hop: 20 + 3 + 20 + 3 + 20 = 66 bytes
```

---

#### Op 6: Pool Info via eth_call

Get pool address first, then query slot0:

**Get pool address:**
```solidity
// On PancakeV3Factory
function getPool(address tokenA, address tokenB, uint24 fee) external view returns (address pool)
```
ABI selector: `0x1698ee82`

**Query slot0 on the pool:**
```solidity
function slot0() external view returns (
    uint160 sqrtPriceX96,
    int24 tick,
    uint16 observationIndex,
    uint16 observationCardinality,
    uint16 observationCardinalityNext,
    uint32 feeProtocol,
    bool unlocked
)
```
ABI selector: `0x3850c7bd`

**Compute price from sqrtPriceX96:**
```
price = (sqrtPriceX96 / 2^96)^2
// Adjust for token decimals: price_adjusted = price * 10^(decimals0 - decimals1)
```

**Query liquidity on the pool:**
```solidity
function liquidity() external view returns (uint128)
```
ABI selector: `0x1a686502`

---

#### Op 11: View LP Positions via NonfungiblePositionManager

**Function signature:**
```solidity
function positions(uint256 tokenId) external view returns (
    uint96 nonce,
    address operator,
    address token0,
    address token1,
    uint24 fee,
    int24 tickLower,
    int24 tickUpper,
    uint128 liquidity,
    uint256 feeGrowthInside0LastX128,
    uint256 feeGrowthInside1LastX128,
    uint128 tokensOwed0,
    uint128 tokensOwed1
)
```
ABI selector: `0x99fbab88`

**Getting all tokenIds for an address:** Use the subgraph (see §4) or iterate via:
```solidity
function balanceOf(address owner) external view returns (uint256)
function tokenOfOwnerByIndex(address owner, uint256 index) external view returns (uint256)
```

---

### 2e. On-Chain Write Operations (onchainos calldata)

All on-chain writes use:
```
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <CONTRACT_ADDRESS> \
  --input-data <HEX_CALLDATA> \
  [--value <WEI_AMOUNT>]
```

There is NO `onchainos dex swap` or `onchainos dex approve` command.

---

#### Op 1: ERC-20 approve

Must be called before the first swap (or when allowance is insufficient). Approve SmartRouter to spend `tokenIn`.

**Function signature:**
```solidity
function approve(address spender, uint256 amount) external returns (bool)
```
ABI selector: `0x095ea7b3`

**Calldata construction:**
```
calldata = 0x095ea7b3
         + abi_encode(spender: address)   // 32 bytes, left-padded
         + abi_encode(amount: uint256)    // 32 bytes
// Total: 4 + 32 + 32 = 68 bytes
```

**Example (approve SmartRouter on BSC for max amount):**
```
onchainos wallet contract-call \
  --chain 56 \
  --to <TOKEN_ADDRESS> \
  --input-data 0x095ea7b3\
               00000000000000000000000013f4ea83d0bd40e75c8222255bc855a974568dd4\
               ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
```

**Recommended amount:** `type(uint256).max` = `0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`

---

#### Op 2: exactInputSingle — Single-pool exact input swap

**Function signature (SmartRouter V3SwapRouter):**
```solidity
function exactInputSingle(ExactInputSingleParams calldata params)
    external
    payable
    returns (uint256 amountOut)
```

**ExactInputSingleParams struct:**
```solidity
struct ExactInputSingleParams {
    address tokenIn;         // 32 bytes
    address tokenOut;        // 32 bytes
    uint24  fee;             // 32 bytes (padded)
    address recipient;       // 32 bytes
    uint256 amountIn;        // 32 bytes
    uint256 amountOutMinimum;// 32 bytes
    uint160 sqrtPriceLimitX96; // 32 bytes
}
```

ABI selector: `0x04e45aaf`

> **Note:** The SmartRouter's `exactInputSingle` does NOT include a `deadline` field (unlike the legacy SwapRouter `0x1b81D678`). Use the SmartRouter for all new code. The legacy SwapRouter at `0x1b81D678` has `deadline` in the struct (selector `0x414bf389`).

**Calldata construction (Rust pseudocode):**
```rust
let selector = hex!("04e45aaf");
let params = abi_encode_tuple((
    token_in:            Address,  // pad to 32 bytes
    token_out:           Address,  // pad to 32 bytes
    fee:                 U256::from(500u32),  // uint24 padded to 32
    recipient:           Address,  // pad to 32 bytes
    amount_in:           U256,
    amount_out_minimum:  U256,     // = quote * (1 - slippage)
    sqrt_price_limit:    U256::zero(), // 0 = no limit
));
let calldata = [selector, params].concat();
```

**onchainos command:**
```
onchainos wallet contract-call \
  --chain 56 \
  --to 0x13f4EA83D0bd40E75C8222255bc855a974568Dd4 \
  --input-data <HEX_CALLDATA>
```

**For native BNB/ETH input (wrapping via value):** Set `tokenIn = WBNB/WETH`, pass `--value <WEI_AMOUNT>`. The SmartRouter handles unwrapping automatically.

---

#### Op 3: exactInput — Multi-hop exact input swap

**Function signature:**
```solidity
function exactInput(ExactInputParams calldata params)
    external
    payable
    returns (uint256 amountOut)
```

**ExactInputParams struct:**
```solidity
struct ExactInputParams {
    bytes   path;           // ABI dynamic type: offset + length + data
    address recipient;      // 32 bytes
    uint256 amountIn;       // 32 bytes
    uint256 amountOutMinimum; // 32 bytes
}
```

ABI selector: `0xb858183f`

**Path encoding (tight packing, NOT ABI-padded):**
```
path = tokenIn (20 bytes) ++ fee0 (3 bytes big-endian) ++ tokenMid (20 bytes) ++ fee1 (3 bytes) ++ tokenOut (20 bytes)
```

**Full calldata layout** (dynamic type, struct with bytes):
```
selector (4 bytes)
offset to path data = 0x80 (128, pointing past 4 static words)  [32 bytes]
recipient                                                         [32 bytes]
amountIn                                                         [32 bytes]
amountOutMinimum                                                 [32 bytes]
path.length (bytes)                                              [32 bytes]
path data (padded to 32-byte boundary)                          [N bytes]
```

---

#### Op 7: NonfungiblePositionManager — mint (add liquidity to new position)

**Function signature:**
```solidity
function mint(MintParams calldata params)
    external
    payable
    returns (uint256 tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)
```

**MintParams struct:**
```solidity
struct MintParams {
    address token0;          // must be token0 < token1 (lower address first)
    address token1;
    uint24  fee;
    int24   tickLower;       // must be multiple of tickSpacing
    int24   tickUpper;
    uint256 amount0Desired;
    uint256 amount1Desired;
    uint256 amount0Min;
    uint256 amount1Min;
    address recipient;
    uint256 deadline;
}
```

ABI selector: `0x88316456`

**tickSpacing by fee tier:**
| fee | tickSpacing |
|-----|-------------|
| 100 | 1 |
| 500 | 10 |
| 2500 | 50 |
| 10000 | 200 |

**onchainos command:**
```
onchainos wallet contract-call \
  --chain 56 \
  --to 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364 \
  --input-data <HEX_CALLDATA>
```

**Pre-conditions:**
1. Both token0 and token1 must be approved to the NonfungiblePositionManager (Op 1, separate call per token).
2. token0 address MUST be numerically less than token1 address. If not, swap them and swap amounts.
3. Compute tickLower/tickUpper from price range; round to nearest valid tick multiple.

---

#### Op 8: NonfungiblePositionManager — increaseLiquidity

**Function signature:**
```solidity
function increaseLiquidity(IncreaseLiquidityParams calldata params)
    external
    payable
    returns (uint128 liquidity, uint256 amount0, uint256 amount1)
```

**IncreaseLiquidityParams struct:**
```solidity
struct IncreaseLiquidityParams {
    uint256 tokenId;
    uint256 amount0Desired;
    uint256 amount1Desired;
    uint256 amount0Min;
    uint256 amount1Min;
    uint256 deadline;
}
```

ABI selector: `0x219f5d17`

---

#### Op 9: NonfungiblePositionManager — decreaseLiquidity

**Function signature:**
```solidity
function decreaseLiquidity(DecreaseLiquidityParams calldata params)
    external
    payable
    returns (uint256 amount0, uint256 amount1)
```

**DecreaseLiquidityParams struct:**
```solidity
struct DecreaseLiquidityParams {
    uint256 tokenId;
    uint128 liquidity;   // padded to 32 bytes in ABI encoding
    uint256 amount0Min;
    uint256 amount1Min;
    uint256 deadline;
}
```

ABI selector: `0x0c49ccbe`

**Note:** After decreaseLiquidity, tokens are credited to the position but NOT transferred. Must call `collect` (Op 10) to actually receive them.

---

#### Op 10: NonfungiblePositionManager — collect

**Function signature:**
```solidity
function collect(CollectParams calldata params)
    external
    payable
    returns (uint256 amount0, uint256 amount1)
```

**CollectParams struct:**
```solidity
struct CollectParams {
    uint256 tokenId;
    address recipient;
    uint128 amount0Max;   // use type(uint128).max = 0xffffffffffffffffffffffffffffffff
    uint128 amount1Max;   // use type(uint128).max
}
```

ABI selector: `0xfc6f7865`

**Calldata layout:**
```
0xfc6f7865
tokenId          [32 bytes]
recipient        [32 bytes, address padded]
amount0Max       [32 bytes, uint128 left-padded to 32]
amount1Max       [32 bytes, uint128 left-padded to 32]
```

---

### 2f. ABI Encoding Reference (Rust)

The plugin should implement a minimal ABI encoder. Key rules:

- **Static types** (address, uint256, uint24, int24, uint128, uint160, bool): each encodes as exactly 32 bytes, right-value left-padded with zeros.
- **address**: 20 bytes value, 12 bytes of leading zeros. Total 32 bytes.
- **int24/int256 (signed)**: sign-extended to 32 bytes (negative values have leading `0xff` bytes).
- **Dynamic types** (bytes, string): encoded as an offset (pointing to data location) in the static area, then length + data (padded to 32-byte multiple) in the dynamic area.
- **Structs**: encoded as a tuple of their fields (static or dynamic rules apply per field).

Recommended Rust crate: `ethabi` or `alloy-core` (preferred, modern). Both support tuple encoding.

---

## 3. User Scenarios

### Scenario 1: Simple Token Swap (Happy Path)

**User says:** "Swap 0.5 BNB for USDT on BSC using PancakeSwap"

**Agent action sequence:**

1. **[Off-chain] Resolve token addresses**
   - `tokenIn` = WBNB = `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c` (chain 56)
   - `tokenOut` = USDT = `0x55d398326f99059ff775485246999027b3197955`
   - Swap is BNB→USDT, so `tokenIn` is WBNB but user sends native BNB via `--value`

2. **[Off-chain] Check fee tier** — Use default 500 (0.05%) for WBNB/USDT, the most liquid pool.

3. **[Off-chain] Get quote via eth_call to QuoterV2**
   - Contract: `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` (BSC)
   - Method: `quoteExactInputSingle`
   - Params: `{tokenIn: WBNB, tokenOut: USDT, amountIn: 0.5e18, fee: 500, sqrtPriceLimitX96: 0}`
   - Returns: `amountOut` (e.g., ~310 USDT = 310_000_000_000_000_000_000 wei with 18 decimals)

4. **[Off-chain] Apply slippage**
   - `amountOutMinimum = amountOut * (1 - 0.005)` = ~308.4 USDT

5. **[Off-chain] Check BNB balance**
   - Confirm wallet has >= 0.5 BNB + gas

6. **[Off-chain] Compute deadline**
   - `deadline = current_block_timestamp + 1200` (20 minutes)
   - Note: SmartRouter `exactInputSingle` does NOT have a deadline param; deadline is N/A for this method. Skip.

7. **[On-chain] Call SmartRouter.exactInputSingle with native BNB value**
   - Construct calldata for `exactInputSingle`:
     ```
     tokenIn:            0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c
     tokenOut:           0x55d398326f99059ff775485246999027b3197955
     fee:                500
     recipient:          <wallet_address>
     amountIn:           500000000000000000 (0.5 × 10^18)
     amountOutMinimum:   308400000000000000000
     sqrtPriceLimitX96:  0
     ```
   - Command:
     ```
     onchainos wallet contract-call \
       --chain 56 \
       --to 0x13f4EA83D0bd40E75C8222255bc855a974568Dd4 \
       --value 500000000000000000 \
       --input-data <encoded_calldata>
     ```

8. **[Off-chain] Confirm transaction and report output amount to user.**

---

### Scenario 2: Get Price / Pool Information (Query)

**User says:** "What's the current WETH/USDC price on PancakeSwap Base?"

**Agent action sequence:**

1. **[Off-chain] Resolve token addresses on Base (8453)**
   - `tokenA` = WETH = `0x4200000000000000000000000000000000000006`
   - `tokenB` = USDC native = `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`
   - Most liquid fee tier for WETH/USDC = 500

2. **[Off-chain] Get pool address via eth_call to PancakeV3Factory**
   - Contract: `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` (Base)
   - Method: `getPool(tokenA, tokenB, 500)`
   - Returns: pool address (e.g., `0xe58b73ff901325b8b2056b29712c50237242f520`)

3. **[Off-chain] Get slot0 from pool**
   - eth_call to pool address, method `slot0()`
   - Returns: `sqrtPriceX96`, `tick`, etc.

4. **[Off-chain] Compute human-readable price**
   - `price = (sqrtPriceX96 / 2^96)^2`
   - WETH has 18 decimals, USDC has 6 decimals
   - `adjusted_price = price * 10^(18 - 6)` = price in USDC per WETH
   - Example output: "Current WETH price: ~3,245 USDC on PancakeSwap Base (fee tier 0.05%)"

5. **[Off-chain] Also query liquidity:**
   - eth_call to pool address, method `liquidity()`
   - Report TVL indication to user.

6. **[Off-chain] Optionally query subgraph for 24h volume:**
   ```graphql
   {
     pool(id: "0xe58b73ff901325b8b2056b29712c50237242f520") {
       token0Price
       token1Price
       volumeUSD
       tvlUSD
       feeTier
     }
   }
   ```
   Endpoint: `https://api.studio.thegraph.com/query/45376/exchange-v3-base/version/latest`

---

### Scenario 3: Add Liquidity to V3 Pool

**User says:** "Add liquidity to the WBNB/USDT 0.05% pool on BSC, with 0.1 BNB and corresponding USDT, in the ±10% price range"

**Agent action sequence:**

1. **[Off-chain] Resolve addresses**
   - token0 = WBNB = `0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c`
   - token1 = USDT = `0x55d398326f99059ff775485246999027b3197955`
   - Verify token0 < token1 numerically: `0xbb4c... > 0x55d3...`, so sort: token0 = USDT, token1 = WBNB
   - fee = 500

2. **[Off-chain] Get current tick from slot0**
   - Get pool via `getPool(USDT, WBNB, 500)`
   - Call `slot0()`, extract `tick` (e.g., tick = 68000)
   - tickSpacing for fee=500 is 10

3. **[Off-chain] Compute tick range for ±10%**
   - Current price from tick: `price = 1.0001^tick`
   - Lower price = current_price * 0.9 → `tickLower = floor(log(0.9*price) / log(1.0001) / 10) * 10`
   - Upper price = current_price * 1.1 → `tickUpper = ceil(log(1.1*price) / log(1.0001) / 10) * 10`

4. **[Off-chain] Compute amount1 (WBNB) for 0.1 BNB = 0.1e18 wei**
   - Using current sqrtPriceX96 and tick range, compute USDT amount needed
   - (Standard Uniswap V3 liquidity math: `L = amount / (sqrt_upper - sqrt_current)`)

5. **[On-chain] Approve USDT for NonfungiblePositionManager (if needed)**
   - Check allowance: `USDT.allowance(wallet, NPM_ADDRESS)`
   - If insufficient:
     ```
     onchainos wallet contract-call \
       --chain 56 \
       --to 0x55d398326f99059ff775485246999027b3197955 \
       --input-data 0x095ea7b3\
                    00000000000000000000000046a15b0b27311cedf172ab29e4f4766fbe7f4364\
                    ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
     ```

6. **[On-chain] Wrap BNB to WBNB (if needed) or use value parameter**
   - If user holds native BNB: must wrap first via WBNB.deposit() OR
   - NonfungiblePositionManager accepts native ETH/BNB with `--value` and handles wrapping internally when `token1 = WBNB` (the contract has a `refundETH` fallback)
   - Approve WBNB to NPM address if using wrapped

7. **[On-chain] Call NonfungiblePositionManager.mint**
   - Construct MintParams:
     ```
     token0:         0x55d398326f99059ff775485246999027b3197955 (USDT, lower addr)
     token1:         0xbb4CdB9CBd36B01bD1cBaEBF2De08d9173bc095c (WBNB, higher addr)
     fee:            500
     tickLower:      <computed_lower>
     tickUpper:      <computed_upper>
     amount0Desired: <usdt_amount_in_wei>
     amount1Desired: 100000000000000000 (0.1 × 10^18 WBNB)
     amount0Min:     <usdt_amount * 0.99> (1% slippage)
     amount1Min:     <wbnb_amount * 0.99>
     recipient:      <wallet_address>
     deadline:       <block_timestamp + 1200>
     ```
   - Command:
     ```
     onchainos wallet contract-call \
       --chain 56 \
       --to 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364 \
       --input-data <encoded_mint_calldata>
     ```

8. **[Off-chain] Parse receipt for tokenId (from Transfer event log)**
   - Report tokenId and actual amounts provided to user.

---

### Scenario 4: Remove Liquidity and Collect (Risk-Aware)

**User says:** "Remove all my liquidity from my PancakeSwap V3 position #1234 on BSC"

**Agent action sequence:**

1. **[Off-chain] Verify position ownership**
   - eth_call: `NonfungiblePositionManager.positions(1234)` on BSC
   - Confirm `liquidity > 0` and position exists
   - Check current tick vs tickLower/tickUpper to inform user if position is out-of-range (single-sided)

2. **[Off-chain] Warn user if out-of-range**
   - If `tick < tickLower` or `tick > tickUpper`, only one token will be returned
   - Confirm user still wants to proceed

3. **[Off-chain] Estimate output amounts**
   - Use liquidity math: `amount0 = liquidity * (1/sqrt_lower - 1/sqrt_current)` etc.

4. **[On-chain] Call decreaseLiquidity**
   - Params: `{tokenId: 1234, liquidity: <full_liquidity_from_positions>, amount0Min: 0, amount1Min: 0, deadline: now+1200}`
   - Command:
     ```
     onchainos wallet contract-call \
       --chain 56 \
       --to 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364 \
       --input-data <encoded_decreaseLiquidity_calldata>
     ```

5. **[On-chain] Call collect to transfer tokens to wallet**
   - Params: `{tokenId: 1234, recipient: <wallet_address>, amount0Max: 0xffffffffffffffffffffffffffffffff, amount1Max: 0xffffffffffffffffffffffffffffffff}`
   - Command:
     ```
     onchainos wallet contract-call \
       --chain 56 \
       --to 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364 \
       --input-data <encoded_collect_calldata>
     ```

6. **[Off-chain] Report amounts received to user.**
   - Parse Transfer events from the collect receipt to confirm token amounts.

---

### Scenario 5: Check My LP Positions

**User says:** "Show me all my PancakeSwap V3 positions on Base"

**Agent action sequence:**

1. **[Off-chain] Get wallet address from onchainos context**

2. **[Off-chain] Query subgraph for positions**
   ```graphql
   {
     positions(where: { owner: "<wallet_address_lowercase>", liquidity_gt: "0" }) {
       id
       token0 { symbol, decimals }
       token1 { symbol, decimals }
       feeTier
       tickLower { tickIdx }
       tickUpper { tickIdx }
       liquidity
       collectedFeesToken0
       collectedFeesToken1
       depositedToken0
       depositedToken1
     }
   }
   ```
   Endpoint: `https://api.studio.thegraph.com/query/45376/exchange-v3-base/version/latest`

3. **[Off-chain] For each position, fetch current data via eth_call**
   - `NonfungiblePositionManager.positions(tokenId)` to get `tokensOwed0`, `tokensOwed1` (uncollected fees)

4. **[Off-chain] Format and present to user**
   - Show: position ID, token pair, fee tier, price range (human-readable), current liquidity, uncollected fees

---

## 4. External API Dependencies

| Dependency | URL / Endpoint | Purpose | Auth Required |
|------------|---------------|---------|---------------|
| BSC public RPC | `https://bsc-dataseed1.binance.org` (primary) / `https://bsc-dataseed2.binance.org` (fallback) | All eth_call reads on BSC | No (public) |
| Base public RPC | `https://mainnet.base.org` (primary) / `https://base.publicnode.com` (fallback) | All eth_call reads on Base | No (public) |
| Private RPC (configured) | User-configured via `rpc_url_bsc` / `rpc_url_base` | Override default RPC | No |
| PancakeSwap V3 Subgraph — BSC | `https://api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-bsc` (hosted, deprecated) or `https://gateway.thegraph.com/api/<API_KEY>/subgraphs/id/78EUqzJmEVJsAKvWghn7qotf9LVGqcTQxJhT5z84ZmgJ` (decentralized) | Pool info, positions, volume for BSC | TheGraph API key for decentralized |
| PancakeSwap V3 Subgraph — Base | `https://api.studio.thegraph.com/query/45376/exchange-v3-base/version/latest` | Pool info, positions for Base | No (studio) |
| PancakeSwap Info API | `https://api.pancakeswap.info/api/v2/tokens/{address}` | Token price in USD, market data | No (public) |
| onchainos wallet contract-call | CLI command | All on-chain write operations | Managed by onchainos |

**Notes:**
- The TheGraph hosted service (`api.thegraph.com/subgraphs/name/...`) is being deprecated; prefer the decentralized network or Studio endpoints.
- For high-frequency quote fetching, prefer `eth_call` to QuoterV2 over subgraph (lower latency, no indexing lag).
- BSC public RPCs have rate limits; configure a private node URL for production use.
- No API key is required for basic operation; TheGraph decentralized requires an API key for production throughput.

---

## 5. Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `chain` | u64 | `56` | Target chain ID. Supported: 56 (BSC), 8453 (Base) |
| `slippage_bps` | u32 | `50` | Slippage tolerance in basis points. 50 = 0.5%. Applied to amountOutMinimum. |
| `deadline_seconds` | u64 | `1200` | Transaction deadline offset from current block timestamp (seconds). Used for LP operations that include deadline. |
| `dry_run` | bool | `false` | Simulate all operations without broadcasting. Prints calldata and estimated output without submitting to chain. |
| `rpc_url_bsc` | String | `"https://bsc-dataseed1.binance.org"` | Custom JSON-RPC endpoint for BSC. Override for private node. |
| `rpc_url_base` | String | `"https://mainnet.base.org"` | Custom JSON-RPC endpoint for Base. Override for private node. |
| `thegraph_api_key` | String (optional) | `""` | TheGraph decentralized network API key. Required for production subgraph queries on BSC. |
| `default_fee_tier` | u32 | `500` | Default V3 fee tier to try first for pool lookup. Options: 100, 500, 2500, 10000. |
| `max_hops` | u8 | `3` | Maximum number of hops in a multi-hop route. |
| `approve_max` | bool | `true` | If true, approve `type(uint256).max` to avoid repeated approvals. If false, approve exact amount per swap. |

---

## Appendix A: ABI Function Selector Reference

| Operation | Contract | Selector | Full Signature |
|-----------|----------|----------|----------------|
| approve | ERC-20 token | `0x095ea7b3` | `approve(address,uint256)` |
| allowance | ERC-20 token | `0xdd62ed3e` | `allowance(address,address)` |
| balanceOf | ERC-20 token | `0x70a08231` | `balanceOf(address)` |
| decimals | ERC-20 token | `0x313ce567` | `decimals()` |
| symbol | ERC-20 token | `0x95d89b41` | `symbol()` |
| exactInputSingle | SmartRouter | `0x04e45aaf` | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint160))` |
| exactInput | SmartRouter | `0xb858183f` | `exactInput((bytes,address,uint256,uint256))` |
| exactInputSingle (legacy SwapRouter) | SwapRouter | `0x414bf389` | `exactInputSingle((address,address,uint24,address,uint256,uint256,uint256,uint160))` |
| quoteExactInputSingle | QuoterV2 | `0xc6a5026a` | `quoteExactInputSingle((address,address,uint256,uint24,uint160))` |
| quoteExactInput | QuoterV2 | `0xcdca1753` | `quoteExactInput(bytes,uint256)` |
| getPool | PancakeV3Factory | `0x1698ee82` | `getPool(address,address,uint24)` |
| slot0 | PancakeV3Pool | `0x3850c7bd` | `slot0()` |
| liquidity | PancakeV3Pool | `0x1a686502` | `liquidity()` |
| mint | NonfungiblePositionManager | `0x88316456` | `mint((address,address,uint24,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` |
| increaseLiquidity | NonfungiblePositionManager | `0x219f5d17` | `increaseLiquidity((uint256,uint256,uint256,uint256,uint256,uint256))` |
| decreaseLiquidity | NonfungiblePositionManager | `0x0c49ccbe` | `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` |
| collect | NonfungiblePositionManager | `0xfc6f7865` | `collect((uint256,address,uint128,uint128))` |
| positions | NonfungiblePositionManager | `0x99fbab88` | `positions(uint256)` |
| balanceOf (ERC721) | NonfungiblePositionManager | `0x70a08231` | `balanceOf(address)` |
| tokenOfOwnerByIndex | NonfungiblePositionManager | `0x2f745c59` | `tokenOfOwnerByIndex(address,uint256)` |

---

## Appendix B: Key Gotchas for Developer

1. **SmartRouter has no `deadline` param.** The SmartRouter's `exactInputSingle` struct has 7 fields, not 8. The legacy `SwapRouter (0x1b81D678)` has 8 fields including `deadline`. Use SmartRouter for all new swaps; do NOT add a deadline field or the calldata will be malformed.

2. **token0 < token1 for LP operations.** NonfungiblePositionManager requires `token0 < token1` (lexicographic/numeric address ordering). Always sort before encoding MintParams. If reversed, the tx will revert.

3. **decreaseLiquidity does not transfer tokens.** After decreaseLiquidity, you must call `collect` to actually receive the tokens. These are always two separate transactions.

4. **Approvals go to different contracts for swap vs LP.** Swaps: approve SmartRouter. LP minting/increasing: approve NonfungiblePositionManager. These are different addresses.

5. **QuoterV2 gas.** QuoterV2 uses gas to simulate swaps via eth_call. Some RPC nodes impose gas limits on eth_call (default 50M). QuoterV2 can use up to 5M gas for complex routes — ensure `eth_call` is called with adequate gas cap (`"gas": "0x4C4B40"` in the JSON-RPC params).

6. **Native BNB/ETH handling.** For swapping native BNB/ETH: set `tokenIn = WBNB/WETH` and pass `--value` to the SmartRouter. It handles wrapping internally. After the swap, if outputting native BNB, the SmartRouter can unwrap — set `recipient = address(0)` or use `multicall` with `unwrapWETH9`. For simplicity, always work with wrapped tokens and require the user to hold WBNB/WETH.

7. **Tick spacing must be respected.** tickLower and tickUpper must be multiples of tickSpacing for the fee tier. Rounding errors cause reverts. Always floor tickLower down and ceil tickUpper up to the nearest multiple.

8. **Base uses USDC.b and USDC native as separate tokens.** `0xd9aaec86...` = USDC.b (bridged), `0x833589fc...` = USDC native. Treat them as distinct tokens for pool selection.
