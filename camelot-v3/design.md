# Camelot V3 Plugin — Design Document

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | camelot-v3 |
| dapp_name | Camelot V3 |
| version | 0.1.0 |
| target_chains | Arbitrum (42161) |
| category | dex |
| description | Camelot DEX V3 — concentrated liquidity AMM (Algebra V1 fork) on Arbitrum. Supports swap, quote, and LP positions. |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无官方 Rust SDK |
| SDK 支持哪些技术栈？ | JavaScript/TypeScript (algebra-js), Python unofficial |
| 有 REST API？ | 无专用 REST API；通过直接合约调用（RPC eth_call）交互 |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | 无，但有 PancakeSwap V3 CLMM 参考（类似 Algebra 架构） |
| 支持哪些链？ | Arbitrum (42161) — 主网 |
| 是否需要 onchainos 广播？ | Yes — swap, add/remove liquidity 需要链上写操作 |

**接入路径**: API（直接 RPC eth_call + onchainos wallet contract-call）

**参考项目**: pancakeswap-v3-clmm（同为 UniV3 fork，相似架构）

---

## §2 接口映射

### 2a. 操作列表

| # | 操作 | 类型 |
|---|------|------|
| 1 | quote — 查询交换报价 | 链下（eth_call） |
| 2 | swap — 执行代币交换 | 链上写操作 |
| 3 | positions — 查询 LP 持仓 | 链下（eth_call） |
| 4 | add-liquidity — 添加流动性 | 链上写操作 |
| 5 | remove-liquidity — 移除流动性 | 链上写操作 |

### 2b. 链下查询

#### Quote (quoteExactInputSingle)

**合约**: Quoter `0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E`

**Algebra V1 不同于 UniV3**:
- 无 fee tier 参数（单 pool per pair）
- `quoteExactInputSingle(address tokenIn, address tokenOut, uint256 amountIn, uint160 limitSqrtPrice)`
- Selector: `0x2d9ebd1d` (verified with `cast sig`)
- Returns: `(uint256 amountOut, uint16 fee, uint256 sqrtPrice, uint256 sqrtPriceX96After, uint32 initializedTicksCrossed, uint256 gasEstimate)`

**Pool existence check**:
- AlgebraFactory `0x1a3c9B1d2F0529D97f2afC5136Cc23e58f1FD35B`
- `poolByPair(address tokenA, address tokenB)` → selector `0xd9a641e1`
- Returns pool address (zero if not deployed)

#### Positions (NonfungiblePositionManager)

**合约**: NFPM `0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15`

- `positions(uint256 tokenId)` → selector `0x99fbab88`
- Returns: `(uint96 nonce, address operator, address token0, address token1, int24 tickLower, int24 tickUpper, uint128 liquidity, ...)`

### 2c. 链上写操作

#### Swap (exactInputSingle)

**合约**: SwapRouter `0x1F721E2E82F6676FCE4eA07A5958cF098D339e18`

**Algebra V1 ExactInputSingleParams struct**:
```
struct ExactInputSingleParams {
    address tokenIn;
    address tokenOut;
    address recipient;
    uint256 deadline;
    uint256 amountIn;
    uint256 amountOutMinimum;
    uint160 limitSqrtPrice;  // 0 = no limit
}
```

Selector: `0xbc651188` (verified with `cast sig "exactInputSingle((address,address,address,uint256,uint256,uint256,uint160))"`)

**ERC-20 approve**: `approve(address,uint256)` selector `0x095ea7b3`

**onchainos 命令**:
```bash
onchainos wallet contract-call --chain 42161 --to 0x1F721E2E82F6676FCE4eA07A5958cF098D339e18 --input-data <calldata> --force
```

#### Add Liquidity (NFPM mint)

**合约**: NFPM `0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15`

**MintParams struct**:
```
struct MintParams {
    address token0;
    address token1;
    int24 tickLower;
    int24 tickUpper;
    uint256 amount0Desired;
    uint256 amount1Desired;
    uint256 amount0Min;
    uint256 amount1Min;
    address recipient;
    uint256 deadline;
}
```

Selector: `0xa232240b` (verified with `cast sig "mint((address,address,int24,int24,uint256,uint256,uint256,uint256,address,uint256))"`)

**onchainos 命令**:
```bash
# Step 1: approve token0
onchainos wallet contract-call --chain 42161 --to <token0> --input-data <approve_calldata> --force
# Step 2: approve token1
onchainos wallet contract-call --chain 42161 --to <token1> --input-data <approve_calldata> --force
# Step 3: mint
onchainos wallet contract-call --chain 42161 --to 0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15 --input-data <mint_calldata> --force
```

#### Remove Liquidity (decreaseLiquidity + collect + burn)

**合约**: NFPM `0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15`

Step 1: `decreaseLiquidity((uint256 tokenId, uint128 liquidity, uint256 amount0Min, uint256 amount1Min, uint256 deadline))`
Selector: `0x0c49ccbe`

Step 2: `collect((uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max))`
Selector: `0xfc6f7865`

Step 3 (optional): `burn(uint256 tokenId)` - selector `0x42966c68`

---

## §3 用户场景

### 场景 1: 查询 ETH → USDT 价格

用户说: "在 Camelot V3 上查询用 0.001 ETH 能换多少 USDT"

1. `quote --token-in WETH --token-out USDT --amount 1000000000000000 --chain 42161`
2. 查询 AlgebraFactory.poolByPair(WETH, USDT) → 确认 pool 存在
3. 调用 Quoter.quoteExactInputSingle(WETH, USDT, 1e15, 0) → 返回 amountOut
4. 输出 JSON: `{"token_in": "WETH", "token_out": "USDT", "amount_in": "1000000000000000", "amount_out": "..."}`

### 场景 2: 用 USDT 换 WETH

用户说: "在 Camelot V3 上用 1 USDT 换 WETH，链 42161"

1. `swap --token-in USDT --token-out WETH --amount-in 1000000 --chain 42161`
2. resolve_wallet(42161) → 获取收款地址
3. quoteExactInputSingle → 计算 amountOutMinimum (0.5% slippage)
4. 检查并设置 USDT allowance → approve if needed
5. 5s 延迟
6. exactInputSingle calldata → `wallet contract-call --force`
7. 输出 txHash

### 场景 3: 查询 LP 持仓

用户说: "查询我在 Camelot V3 的流动性仓位"

1. `positions --chain 42161`
2. 调用 NFPM.balanceOf(wallet) → 获取持仓数量
3. 遍历 tokenOfOwnerByIndex → 获取 tokenId 列表
4. 对每个 tokenId 调用 positions(tokenId) → 获取仓位详情
5. 输出 JSON 列表

### 场景 4: 添加流动性

用户说: "在 Camelot V3 USDT/WETH 池子中添加 0.01 USDT 的流动性"

1. `add-liquidity --token0 USDT --token1 WETH --amount0 10000 --amount1 0 --tick-lower -887200 --tick-upper 887200 --chain 42161`
2. resolve_wallet → recipient
3. approve token0 + token1
4. 5s 延迟
5. NFPM.mint calldata → `wallet contract-call --force`
6. 输出 tokenId, txHash

### 场景 5: 移除流动性

用户说: "移除 Camelot V3 position #123 的所有流动性"

1. `remove-liquidity --token-id 123 --liquidity 100 --chain 42161`
2. positions(123) → 查询当前流动性
3. decreaseLiquidity → `wallet contract-call --force`
4. 5s 延迟
5. collect → `wallet contract-call --force`
6. 输出 txHash, amounts received

---

## §4 外部 API 依赖

| API | 用途 |
|-----|------|
| `https://arb1.arbitrum.io/rpc` | Arbitrum mainnet RPC (default) |
| `https://arbitrum-one-rpc.publicnode.com` | Arbitrum public RPC (fallback) |
| AlgebraFactory `0x1a3c9B1d2F0529D97f2afC5136Cc23e58f1FD35B` | Pool 地址查询 |
| Quoter `0x0Fc73040b26E9bC8514fA028D998E73A254Fa76E` | 报价 |
| SwapRouter `0x1F721E2E82F6676FCE4eA07A5958cF098D339e18` | 交换 |
| NFPM `0x00c7f3082833e796A5b3e4Bd59f6642FF44DCD15` | LP 仓位管理 |

---

## §5 配置参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| chain | 42161 | Arbitrum mainnet |
| slippage | 0.5 | Slippage tolerance (%) |
| deadline_minutes | 20 | Transaction deadline |
| dry_run | false | Dry-run mode (no broadcast) |

---

## Selector Reference (all verified with `cast sig`)

| Function | Selector | Note |
|----------|----------|------|
| `exactInputSingle((address,address,address,uint256,uint256,uint256,uint160))` | `0xbc651188` | Algebra V1 struct (no fee tier) |
| `quoteExactInputSingle(address,address,uint256,uint160)` | `0x2d9ebd1d` | Algebra V1 (no fee, returns amountOut) |
| `poolByPair(address,address)` | `0xd9a641e1` | AlgebraFactory |
| `positions(uint256)` | `0x99fbab88` | NFPM |
| `mint((address,address,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` | `0xa232240b` | NFPM |
| `increaseLiquidity((uint256,uint256,uint256,uint256,uint256,uint256))` | `0x219f5d17` | NFPM |
| `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` | `0x0c49ccbe` | NFPM |
| `collect((uint256,address,uint128,uint128))` | `0xfc6f7865` | NFPM |
| `burn(uint256)` | `0x42966c68` | NFPM |
| `approve(address,uint256)` | `0x095ea7b3` | ERC-20 |
| `allowance(address,address)` | `0xdd62ed3e` | ERC-20 |
| `balanceOf(address)` | `0x70a08231` | ERC-20 |
| `decimals()` | `0x313ce567` | ERC-20 |
| `symbol()` | `0x95d89b41` | ERC-20 |
