# Curve DEX Plugin — Design Document

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `curve` |
| `dapp_name` | Curve DEX |
| `version` | 0.1.0 |
| `category` | defi-protocol |
| `tags` | dex, swap, stablecoin, amm, liquidity |
| `target_chains` | EVM: Ethereum (1), Arbitrum (42161), Base (8453), Polygon (137), BSC (56) |
| `target_protocols` | CurveRouterNG (swap), Curve StableSwap pools (add/remove liquidity) |
| `binary_name` | curve |
| `source_repo` | skylavis-sky/onchainos-plugins |
| `source_dir` | curve |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **无**。官方只有 TypeScript SDK (`@curvefi/api`, https://github.com/curvefi/curve-js)。无官方 Rust SDK。 |
| SDK 支持哪些技术栈？ | TypeScript / JavaScript 仅有。社区无 Rust 封装。 |
| 有 REST API？ | **有**。`https://api.curve.finance/v1/` 提供 getPools、getVolumes、APY 等只读查询接口。文档：https://api.curve.finance/v1/documentation/ |
| 有官方 Skill？ | **无**。Bitable 记录确认无官方 Skill。 |
| 开源社区有类似 Skill？ | **无**。Bitable 记录确认无社区 Skill 参考。 |
| 支持哪些链？ | EVM 25+ 链，主要：Ethereum (1)、Arbitrum (42161)、Base (8453)、Polygon (137)、BSC (56)、Optimism (10)、Avalanche、Fantom 等。本插件仅接入 Ethereum + Arbitrum + Base + Polygon + BSC（TVL 占比 >95%）。 |
| 是否需要 onchainos 广播？ | **Yes**。swap、add_liquidity、remove_liquidity 均为链上写操作，必须走 `onchainos wallet contract-call`。 |

**接入路径：API**

无 Rust SDK、无社区 Skill 参考。有官方 REST API 用于链下查询（获取 pool 列表、TVL、APY、coins）；链上操作通过 `wallet contract-call` 直接 ABI 编码调用 CurveRouterNG 合约（swap）和各 pool 合约（add/remove liquidity）。

---

## §2 接口映射

### 2a. 需要接入的操作

| 操作 | 链上/链下 | 说明 |
|------|----------|------|
| `get-pools` | 链下查询 | 查询指定链上的 Curve pools 列表（名称、地址、coins、TVL、APY） |
| `quote` | 链下查询 | 通过 CurveRouterNG.get_dy() eth_call 查询 swap 预期输出金额 |
| `swap` | 链上写操作 | 通过 CurveRouterNG.exchange() 执行 token swap |
| `add-liquidity` | 链上写操作 | 调用 pool 合约 add_liquidity() 注入流动性，获取 LP token |
| `remove-liquidity` | 链上写操作 | 调用 pool 合约 remove_liquidity() 或 remove_liquidity_one_coin() 赎回流动性 |
| `get-pool-info` | 链下查询 | 查询单个 pool 详情：coins、余额、费率、APY |
| `get-balances` | 链下查询 | 查询用户在各 pool 中的 LP token 余额及对应价值 |

### 2b. 链下查询接口

#### get-pools

**API Endpoint:** `GET https://api.curve.finance/api/getPools/{blockchainId}/{registryId}`

| 参数 | 类型 | 说明 |
|------|------|------|
| `blockchainId` | string | 链名：`ethereum` / `arbitrum` / `base` / `polygon` / `bsc` |
| `registryId` | string | 注册表类型：`main` / `crypto` / `factory` / `factory-crypto` |

**返回关键字段：**
```json
{
  "data": {
    "poolData": [
      {
        "id": "3pool",
        "address": "0xbebc...",
        "name": "Curve.fi DAI/USDC/USDT",
        "coins": [{"address": "0x...", "symbol": "DAI", "decimals": "18"}],
        "usdTotal": 123456789.00,
        "virtualPrice": "1001234567890000000",
        "fee": "4000000"
      }
    ]
  }
}
```

**注意：** 需要遍历 4 个 registryId 聚合所有 pool（或仅查询 `main` + `factory` 作为快速路径）。

---

#### quote (get_dy via eth_call)

无官方 REST quote API。通过直接 eth_call 调用 **CurveRouterNG.get_dy()** 获取 swap 报价。

**合约方法（Vyper ABI）：**
```
get_dy(
  _route: address[11],
  _swap_params: uint256[5][5],
  _amount: uint256,
  _pools: address[5]
) -> uint256
```

**ABI 编码说明：**
- 函数选择器：`keccak256("get_dy(address[11],uint256[5][5],uint256,address[5])")` → `0xe2ad025a`
- `_route`: 11 个地址，token-in 在 index 0，token-out 在最后非零位，其余填 `address(0)`
- `_swap_params`: 5×5 uint256 数组，每行对应一段 swap：`[in_index, out_index, swap_type, pool_type, n_coins]`
- `_amount`: 输入金额（minimal units）
- `_pools`: 路由经过的 pool 地址，不足 5 个填 `address(0)`

**RPC eth_call：**
```
POST {RPC_URL}
{"method":"eth_call","params":[{"to":"{ROUTER}","data":"0xe2ad025a{ABI_ENCODED_ARGS}"},"latest"],"id":1}
```

---

#### get-pool-info

**API Endpoint:** `GET https://api.curve.finance/api/getPools/{blockchainId}/{registryId}`

过滤 `poolData` 数组找到目标 pool 地址。或直接通过 eth_call 查询 pool 合约：
- `balances(i: uint256) -> uint256` — 各 coin 余额
- `fee() -> uint256` — 当前手续费（除以 1e10 得到百分比）
- `virtual_price() -> uint256` — LP token 当前虚拟价值

---

#### get-balances (LP token balance)

通过 `wallet balance` + ERC-20 `balanceOf` eth_call 查询用户 LP token 余额。LP token 地址即 pool 合约地址（Curve pools 中 LP token == pool 合约）。

**balanceOf eth_call：**
```
selector: 0x70a08231
data: 0x70a08231 + abi_encode_address(wallet_address)
```

---

### 2c. 链上写操作

#### swap

**目标合约：** CurveRouterNG（每链一个地址，运行时从配置获取）

| 链 | CurveRouterNG 地址 |
|----|-------------------|
| Ethereum (1) | `0x45312ea0eFf7E09C83CBE249fa1d7598c4C8cd4e` |
| Arbitrum (42161) | `0x2191718CD32d02B8E60BAdFFeA33E4B5DD9A0A0D` |
| Base (8453) | `0x4f37A9d177470499A2dD084621020b023fcffc1F` |
| Polygon (137) | `0x0DCDED3545D565bA3B19E683431381007245d983` |
| BSC (56) | `0xA72C85C258A81761433B4e8da60505Fe3Dd551CC` |

**合约方法：**
```
exchange(
  _route: address[11],
  _swap_params: uint256[5][5],
  _amount: uint256,
  _expected: uint256,
  _pools: address[5],
  _receiver: address
)
```

**函数选择器：** `keccak256("exchange(address[11],uint256[5][5],uint256,uint256,address[5],address)")` → `0x5f575529`

**Calldata 构造方式（ABI 编码）：**
1. `_route` — `address[11]`：token-in 在 index 0，token-out 在最后非零位，其他填零地址
2. `_swap_params` — `uint256[5][5]`：每行 `[in_coin_idx, out_coin_idx, swap_type, pool_type, n_coins]`
3. `_amount` — `uint256`：输入金额 minimal units
4. `_expected` — `uint256`：最低输出金额（= get_dy 结果 × (1 - slippage)）
5. `_pools` — `address[5]`：路由经过的 pool，填充零地址
6. `_receiver` — `address`：接收地址（用户钱包地址，dry-run 时用零地址）

**ERC-20 Approve（若 token-in 非原生代币）：**
```
onchainos wallet contract-call \
  --chain {CHAIN_ID} \
  --to {TOKEN_IN_ADDRESS} \
  --input-data 0x095ea7b3{ROUTER_PADDED}{UINT256_MAX} \
  --from {WALLET_ADDRESS}
```
先查 allowance，若已足够则跳过 approve（见 dex.md#allowance-check）。

**onchainos 命令（swap 主体）：**
```
onchainos wallet contract-call \
  --chain {CHAIN_ID} \
  --to {CURVE_ROUTER_NG} \
  --input-data {CALLDATA_HEX} \
  --from {WALLET_ADDRESS} \
  --force
```

---

#### add-liquidity

**目标合约：** 具体 Curve pool 地址（运行时通过 API 查询，不硬编码）

**合约方法（StableSwap N-coin）：**
```
add_liquidity(
  amounts: uint256[N],    -- N 随 pool 币种数变化（2~4）
  min_mint_amount: uint256,
  receiver: address       -- 可选，接收 LP token 的地址
)
```

**函数选择器（2-coin stable pool）：** `keccak256("add_liquidity(uint256[2],uint256)")` → `0x0b4c7e4d`
**函数选择器（3-coin stable pool）：** `keccak256("add_liquidity(uint256[3],uint256)")` → `0x4515cef3`
**函数选择器（2-coin with receiver）：** `keccak256("add_liquidity(uint256[2],uint256,address)")` → `0x7328333b`

**步骤：**
1. ERC-20 approve 每个输入 token → pool 合约（检查 allowance，跳过已足额的）
2. 等待 approve 确认（5s 延迟，见 dex.md#lp-nonce-delay）
3. 调用 `add_liquidity(amounts, min_mint_amount)`

**onchainos 命令：**
```
onchainos wallet contract-call \
  --chain {CHAIN_ID} \
  --to {POOL_ADDRESS} \
  --input-data {CALLDATA_HEX} \
  --from {WALLET_ADDRESS} \
  --force
```

---

#### remove-liquidity

**合约方法（按比例赎回）：**
```
remove_liquidity(
  _amount: uint256,         -- LP token 数量
  min_amounts: uint256[N],  -- 每种 coin 的最低赎回量
  receiver: address         -- 可选
)
```
**函数选择器（2-coin）：** `keccak256("remove_liquidity(uint256,uint256[2])")` → `0x5b36389c`
**函数选择器（3-coin）：** `keccak256("remove_liquidity(uint256,uint256[3])")` → `0x1a4d01d2`

**合约方法（单币赎回）：**
```
remove_liquidity_one_coin(
  _token_amount: uint256,  -- LP token 数量
  i: int128,               -- 赎回 coin 的 index
  _min_amount: uint256     -- 最低赎回量
)
```
**函数选择器：** `keccak256("remove_liquidity_one_coin(uint256,int128,uint256)")` → `0x1a4d01d2`（实际：`0x517a55a3`）

**步骤：**
1. 查询用户 LP token 余额（balanceOf eth_call）
2. LP token approve → pool 合约（LP token 即 pool 合约本身，部分 pool 不需要 approve — 需运行时验证）
3. 调用 remove_liquidity / remove_liquidity_one_coin

**onchainos 命令：**
```
onchainos wallet contract-call \
  --chain {CHAIN_ID} \
  --to {POOL_ADDRESS} \
  --input-data {CALLDATA_HEX} \
  --from {WALLET_ADDRESS} \
  --force
```

---

## §3 用户场景

### 场景 1：查询最佳 swap 报价并执行 USDC → DAI swap（ETH 主网）

**用户说：** "在以太坊主网用 1000 USDC 换 DAI，帮我看看能换多少？"

**Agent 动作序列：**

1. **[链下查询]** `onchainos token search --keyword USDC --chain 1` → 解析 USDC 地址（`0xA0b86991...`，decimals=6）
2. **[链下查询]** `onchainos token search --keyword DAI --chain 1` → 解析 DAI 地址（`0x6B175474...`，decimals=18）
3. **[链下查询]** eth_call CurveRouterNG `get_dy(route, swap_params, 1000_000000, pools)` — 构造 USDC→DAI 路由（3pool），查询预期输出
4. **[链下查询]** `onchainos market price --address {USDC} --chain 1` → 查询当前价格，计算 price impact
5. **展示报价：** "1000 USDC → ~999.52 DAI，通过 Curve 3pool，手续费 ~0.04 USDC，Price impact <0.01%"
6. **询问确认：** "是否确认执行这笔 swap？（请用户确认后执行）"
7. **[链下查询]** eth_call ERC-20 `allowance(wallet, router)` — 检查 USDC 对 Router 的授权
8. **[链上操作]** 若授权不足：`onchainos wallet contract-call --chain 1 --to {USDC} --input-data 0x095ea7b3{ROUTER}{MAX} --from {WALLET}`（ERC-20 approve）
9. 等待 3 秒（approve 确认）
10. **[链上操作]** `onchainos wallet contract-call --chain 1 --to {ROUTER_NG} --input-data {EXCHANGE_CALLDATA} --from {WALLET} --force`
11. **展示结果：** 解析 `.data.txHash`，展示交易哈希及 Etherscan 链接

---

### 场景 2：查询 Curve pools 列表并向 3pool 注入流动性（ETH 主网）

**用户说：** "在以太坊 Curve 上有哪些稳定币 pool？帮我向 3pool 存入 500 USDC 和 500 USDT。"

**Agent 动作序列：**

1. **[链下查询]** `GET https://api.curve.finance/api/getPools/ethereum/main` + `GET .../ethereum/factory` → 解析所有稳定币 pool（过滤 stablecoin），返回 pool 名称、地址、TVL、APY
2. **展示 pool 列表：** 显示 Top pools（3pool、FRAX/USDC、LUSD/3CRV 等），重点标注 3pool APY 和 TVL
3. **[链下查询]** `onchainos token search --keyword USDC --chain 1` → USDC 地址及 decimals
4. **[链下查询]** `onchainos token search --keyword USDT --chain 1` → USDT 地址及 decimals
5. **[链下查询]** `onchainos wallet balance --chain 1 --output json` → 确认 USDC/USDT 余额充足
6. **展示预览：** "即将向 Curve 3pool 存入 500 USDC + 500 USDT，预计获得约 XXX LP tokens（3CRV）。手续费 ~0.04%。（请用户确认后执行）"
7. **[链上操作]** ERC-20 approve USDC → 3pool 合约（先查 allowance，跳过已足额）
8. **[链上操作]** ERC-20 approve USDT → 3pool 合约（先查 allowance，跳过已足额）
9. 等待 5 秒（nonce 间隔，见 dex.md#lp-nonce-delay）
10. **[链上操作]** `onchainos wallet contract-call --chain 1 --to {3POOL_ADDR} --input-data {ADD_LIQUIDITY_3COIN_CALLDATA} --from {WALLET} --force`（amounts=[0, 500_000000, 500_000000]，min_mint=0）
11. **展示结果：** 解析 txHash，显示交易链接及预估获得 LP token 数量

---

### 场景 3：查询 LP 持仓并单币赎回（Arbitrum）

**用户说：** "我在 Arbitrum Curve 的 2pool 里有多少流动性？帮我把我的 LP 全部赎回成 USDC。"

**Agent 动作序列：**

1. **[链下查询]** `GET https://api.curve.finance/api/getPools/arbitrum/main` + `../arbitrum/factory` → 找到 2pool（USDC/USDT）地址
2. **[链下查询]** eth_call ERC-20 `balanceOf(wallet)` on 2pool LP token → 查询用户 LP token 余额
3. **[链下查询]** eth_call pool `calc_withdraw_one_coin(lp_amount, 0)` → 估算单币赎回 USDC 数量
4. **展示持仓：** "您持有 XXX 2CRV LP token，当前价值约 $XXX。赎回全部为 USDC 约可获 XXX USDC（含手续费）。（请用户确认后执行）"
5. **[安全检查]** 确认 pool 合约地址来自官方 API，不是用户输入
6. **询问确认：** "是否确认赎回全部 LP token 为 USDC？请确认。"
7. **[链上操作]** `onchainos wallet contract-call --chain 42161 --to {2POOL_ADDR} --input-data {REMOVE_LIQUIDITY_ONE_COIN_CALLDATA} --from {WALLET} --force`（`remove_liquidity_one_coin(lp_amount, 0, min_amount)`，i=0 表示 USDC）
8. **展示结果：** 解析 txHash，显示 Arbiscan 链接，提示到账 USDC 数量

---

### 场景 4：查询 pool 详情和 APY（只读）

**用户说：** "Curve 上现在 ETH/stETH pool 的 APY 是多少？"

**Agent 动作序列：**

1. **[链下查询]** `GET https://api.curve.finance/api/getPools/ethereum/main` → 查找 steth pool
2. **展示结果：** 池子名称、TVL、base APY（手续费收入）、CRV APY（激励）、总 APY；当前价格偏差、各 coin 余额

---

## §4 外部 API 依赖

| 服务 | URL | 用途 | 认证 |
|------|-----|------|------|
| Curve REST API | `https://api.curve.finance/api/getPools/{chain}/{registry}` | Pool 列表、coins、TVL、APY | 无需认证（公开） |
| Curve REST API | `https://api.curve.finance/api/getApys?address=...` | APY 详情 | 无需认证 |
| Ethereum RPC | `https://eth.llamarpc.com` 或 `https://cloudflare-eth.com` | eth_call（get_dy, balanceOf, allowance） | 无需认证 |
| Arbitrum RPC | `https://arb1.arbitrum.io/rpc` | eth_call | 无需认证 |
| Base RPC | `https://base-rpc.publicnode.com` | eth_call | 无需认证 |
| Polygon RPC | `https://polygon-rpc.com` | eth_call | 无需认证 |
| BSC RPC | `https://bsc-rpc.publicnode.com` | eth_call | 无需认证 |
| onchainos wallet contract-call | CLI | 链上写操作广播 | onchainos 已登录 |

**注意：**
- Base 高频调用（多 fee-tier 路由查询）请用 `base-rpc.publicnode.com`，避免 `-32016 over rate limit`
- BSC 请用 `bsc-rpc.publicnode.com`，`bsc-dataseed.binance.org` 在沙箱环境有 TLS 问题
- Curve API 无需 API Key，免费公开，但有一定请求频率限制

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `chain_id` | u64 | `1` | 目标链 ID（1/42161/8453/137/56） |
| `slippage` | f64 | `0.005` | 允许滑点（0.5%），swap 时计算 min_expected |
| `dry_run` | bool | `false` | true 时只模拟，不广播链上交易 |
| `rpc_url` | String | 见 §4 各链默认 | 自定义 RPC URL（可选覆盖默认） |
| `gas_limit` | Option<u64> | `None`（自动估算） | 手动指定 gas limit |

**dry_run 说明：** `--dry-run` 不可传给 `onchainos wallet contract-call`（CLI 不支持该 flag）。dry_run 在插件 wrapper 层实现：构造 calldata 后直接返回预览信息，不调用 onchainos 命令。receiver 地址在 dry_run 时使用零地址 `0x0000000000000000000000000000000000000000` 作为占位符（见 dex.md#dry-run-placeholder）。

---

## §6 关键实现注意事项

### 路由构建

CurveRouterNG 的 `exchange()` 和 `get_dy()` 接受预计算的路由参数（`_route`、`_swap_params`、`_pools`），不自动寻路。插件需要：
1. 调用 Curve API 获取 pool 列表
2. 基于 token-in、token-out 找到包含这两种 token 的 pool
3. 确定 coin indices（token 在 pool 内的 index）
4. 构造 `_route`（address[11]）和 `_swap_params`（uint256[5][5]）
5. 对于跨 pool 的多段路由，使用 CurveRouterNG 的多 swap 能力（单笔 tx 最多 5 段）

**直接路由（推荐实现 V1 时仅支持单段）：**
```
_route = [token_in, pool_address, token_out, 0x0, 0x0, ...(11个)]
_swap_params = [[in_idx, out_idx, swap_type, pool_type, n_coins], [0,0,0,0,0], ...(5行)]
```

### swap_type 编码
- `1` — StableSwap exchange
- `2` — CryptoSwap exchange
- `3` — StableSwap exchange_underlying
- `4` — exchange via zap (meta pools)
- `7` — 原生 ETH weth 包装类型

### pool_type 编码
- `1` — 普通 StableSwap（2-4 coin）
- `2` — CryptoSwap（volatile assets）
- `3` — Tricrypto

### 代币地址解析

- 用户输入 symbol 时，先用 `onchainos token search` 解析地址（见 dex.md#symbol-resolution）
- 原生 ETH 使用 `0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee`（Curve 的 exchange 也接受原生 ETH，用 `--amt` 传 wei 值）

### 安全规则

- Pool 合约地址**只**从 Curve 官方 API（`api.curve.finance`）获取，绝不接受用户输入合约地址
- swap 执行前调用 `onchainos security token-scan` 检查 token-in 和 token-out
- price impact > 5% 时 WARN 用户；> 10% 时阻止执行
- `--force` 标志仅在用户明确确认后添加（见 dex.md 中 DEX swap 的 `--force` 要求）

### AddressProvider（备用路由）

若 CurveRouterNG 地址需运行时验证，可通过 AddressProvider 查询（所有 EVM 链统一地址 `0x5ffe7FB82894076ECB99A30D6A32e969e6e35E98`，调用 `get_address(2)` 获取当前 Exchange Router）。
