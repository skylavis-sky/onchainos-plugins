# design.md — Pendle Finance Plugin

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `pendle` |
| `dapp_name` | Pendle Finance |
| `version` | 0.1.0 |
| `category` | defi-protocol |
| `tags` | yield-trading, fixed-yield, pt, yt, liquidity |
| `target_chains` | Ethereum (1), Arbitrum (42161), BSC (56), Base (8453) |
| `target_protocols` | Pendle V2 (yield tokenization AMM) |
| `binary_name` | pendle |
| `source_repo` | skylavis-sky/onchainos-plugins |
| `source_dir` | pendle |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **No** — Pendle's official SDKs are TypeScript-only: [pendle-sdk-core-v2-public](https://github.com/pendle-finance/pendle-sdk-core-v2-public) (archived) and [sdk-boros-public](https://github.com/pendle-finance/sdk-boros-public). No Rust SDK exists. |
| SDK 支持哪些技术栈？ | TypeScript/JavaScript only (both the v2 SDK and Boros SDK) |
| 有 REST API？ | **Yes** — Full REST API at `https://api-v2.pendle.finance/core` ([Swagger docs](https://api-v2.pendle.finance/core/docs)). Covers markets, assets, prices, positions, and a Hosted SDK endpoint for generating transaction calldata. |
| 有官方 Skill？ | **No** — No official Pendle OnchainOS / plugin-store skill found. |
| 开源社区有类似 Skill？ | **Partial** — A Python MCP server exists ([maneesha029/Pendle_mcp](https://glama.ai/mcp/servers/@maneesha029/Pendle_mcp)) but only simulates operations. A multi-protocol MCP ([Finanzgoblin/spectra-mcp-server](https://github.com/Finanzgoblin/spectra-mcp-server)) covers Pendle in Python/TypeScript but is not a Rust plugin-store skill. Neither is directly usable as a reference implementation. Integration path: use REST API directly. |
| 支持哪些链？ | Ethereum (1), Optimism (10), BSC (56), Base (8453), Arbitrum (42161), Sonic (146), Mantle (5000), HyperEVM (999), Berachain (80094). **OnchainOS-supported subset: Ethereum (1), BSC (56), Base (8453), Arbitrum (42161).** |
| 是否需要 onchainos 广播？ | **Yes** — All write operations (buy PT/YT, add/remove liquidity, mint/redeem PT+YT) require submitting signed transactions on-chain. The Pendle Hosted SDK API generates calldata; `onchainos wallet contract-call` broadcasts the transaction. |

### 接入路径

**API** — No Rust SDK exists. Use Pendle's REST API (`api-v2.pendle.finance/core`) for data queries and the Hosted SDK endpoint (`POST /v3/sdk/{chainId}/convert`) to generate transaction calldata. All on-chain operations are submitted via `onchainos wallet contract-call`.

---

## §2 接口映射

### 2a. 需要接入的操作表

| 操作 | 类型 | 描述 |
|------|------|------|
| `list-markets` | 链下查询 | 列出所有激活的 Pendle 市场（含 APY、TVL、PT/YT 地址） |
| `get-market` | 链下查询 | 查询指定市场详情（implied APY、PT 价格、剩余期限） |
| `get-positions` | 链下查询 | 查询用户在所有链上的持仓（PT、YT、LP） |
| `get-asset-price` | 链下查询 | 查询 PT/YT/LP token 的 USD 价格 |
| `buy-pt` | 链上写操作 | 用底层 token 购买 PT（锁定固定收益） |
| `sell-pt` | 链上写操作 | 出售 PT 换回底层 token（提前退出固定收益） |
| `buy-yt` | 链上写操作 | 用底层 token 购买 YT（做多浮动收益） |
| `sell-yt` | 链上写操作 | 出售 YT 换回底层 token |
| `add-liquidity` | 链上写操作 | 向 Pendle AMM 池添加流动性（单 token 模式） |
| `remove-liquidity` | 链上写操作 | 从 Pendle AMM 池移除流动性（单 token 模式） |
| `mint-py` | 链上写操作 | 用底层 token 同时铸造 PT + YT |
| `redeem-py` | 链上写操作 | 将等量 PT + YT 赎回为底层 token |

---

### 2b. 链下查询表

#### list-markets

- **Endpoint**: `GET https://api-v2.pendle.finance/core/v2/markets/all`
- **参数**:
  - `chainId` (optional, integer) — 过滤指定链，不传则返回所有链
  - `isActive` (optional, boolean) — `true` 仅返回活跃市场
  - `skip` (integer, default 0)
  - `limit` (integer, default 20, max 100)
- **返回关键字段**:
  - `results[].address` — 市场合约地址
  - `results[].name` — 市场名称（如 "PT-stETH 26DEC2025"）
  - `results[].chainId`
  - `results[].expiry` — 到期日（Unix timestamp）
  - `results[].pt` — PT token 地址
  - `results[].yt` — YT token 地址
  - `results[].sy` — SY token 地址
  - `results[].impliedApy` — 隐含 APY（固定收益率）
  - `results[].liquidity.usd` — 池子 TVL（USD）
  - `results[].tradingVolume.usd` — 24h 交易量

#### get-market

- **Endpoint**: `GET https://api-v2.pendle.finance/core/v3/{chainId}/markets/{marketAddress}/historical-data`
- **参数**:
  - `chainId` (path, integer)
  - `marketAddress` (path, string) — 市场合约地址
  - `time_frame` (optional, string) — `"1D"`, `"1W"`, `"1M"`
  - `includeApyBreakdown` (optional, boolean)
- **返回关键字段**:
  - `results[].impliedApy`
  - `results[].maxApy`
  - `results[].tvl`
  - `results[].ptPrice` — PT 价格（以底层 token 计）
  - `results[].ytPrice`
  - `results[].timestamp`

#### get-positions

- **Endpoint**: `GET https://api-v2.pendle.finance/core/v1/dashboard/positions/database/{user}`
- **参数**:
  - `user` (path, string) — 用户钱包地址
  - `filterUsd` (optional, number) — 过滤小于此 USD 值的持仓
- **返回关键字段**:
  - `positions[].chainId`
  - `positions[].marketAddress`
  - `positions[].ptBalance` — PT 余额（wei）
  - `positions[].ytBalance` — YT 余额（wei）
  - `positions[].lpBalance` — LP 余额（wei）
  - `positions[].valueUsd` — 持仓 USD 价值
  - `positions[].impliedApy`

#### get-asset-price

- **Endpoint**: `GET https://api-v2.pendle.finance/core/v1/prices/assets`
- **参数**:
  - `chainId` (optional, integer)
  - `ids` (optional, string) — 逗号分隔的 token 地址列表
  - `type` (optional, string) — `"PT"`, `"YT"`, `"LP"`, `"SY"`
- **返回关键字段**:
  - `priceMap` — `{tokenAddress: usdPrice}` 映射

---

### 2c. 链上写操作表

所有链上写操作流程：
1. 调用 Pendle Hosted SDK API 生成 calldata（`POST /v3/sdk/{chainId}/convert`）
2. 检查 `response.requiredApprovals` — 若需 ERC-20 approve，先提交 approve calldata
3. 用户确认后，执行 `onchainos wallet contract-call` 提交交易

**Router 合约地址（所有支持链统一）**: `0x888888888889758F76e7103c6CbF23ABbF58F946`
> 注：此地址经多链 block explorer 验证（Ethereum/Arbiscan/OP/BSC）且为 Pendle RouterV4，运行时可通过 Pendle Deployments API 动态确认。

---

#### buy-pt（swapExactTokenForPt）

**Hosted SDK 请求** (`POST /v3/sdk/{chainId}/convert`):
```json
{
  "inputs": [
    {
      "tokenIn": "<underlying_token_address>",
      "amountIn": "<amount_in_wei>"
    }
  ],
  "outputs": [
    {
      "tokenOut": "<pt_address>",
      "minAmountOut": "<min_pt_out_wei>"
    }
  ],
  "receiver": "<user_wallet_address>",
  "slippage": 0.01
}
```

**Response 提取**:
- `response.routes[0].tx.data` → calldata (hex)
- `response.routes[0].tx.to` → 应等于 Router 地址
- `response.requiredApprovals` → 检查是否需要 approve

**ERC-20 Approve（若需要）**:
```bash
# approve(address spender, uint256 amount) = 0x095ea7b3
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <TOKEN_ADDRESS> \
  --input-data 0x095ea7b3\
000000000000000000000000888888888889758F76e7103c6CbF23ABbF58F946\
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
  --from <USER_WALLET>
```

**主交易**:
```bash
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to 0x888888888889758F76e7103c6CbF23ABbF58F946 \
  --input-data <calldata_from_sdk_response> \
  --from <USER_WALLET> \
  --force
```

---

#### sell-pt（swapExactPtForToken）

**Hosted SDK 请求**: 与 buy-pt 相同结构，`tokenIn` = PT 地址，`tokenOut` = 底层 token 地址。

**Calldata**: 从 `response.routes[0].tx.data` 获取，提交方式同 buy-pt。

---

#### buy-yt（swapExactTokenForYt）

**Hosted SDK 请求**: `tokenIn` = 底层 token，`tokenOut` = YT 地址。其余字段与 buy-pt 相同。

---

#### sell-yt（swapExactYtForToken）

**Hosted SDK 请求**: `tokenIn` = YT 地址，`tokenOut` = 底层 token。

---

#### add-liquidity（addLiquiditySingleToken）

**Hosted SDK 请求** (`POST /v3/sdk/{chainId}/convert`):
```json
{
  "inputs": [
    {
      "tokenIn": "<underlying_token_address>",
      "amountIn": "<amount_in_wei>"
    }
  ],
  "outputs": [
    {
      "tokenOut": "<lp_token_address>",
      "minAmountOut": "<min_lp_out_wei>"
    }
  ],
  "receiver": "<user_wallet_address>",
  "slippage": 0.01
}
```

Hosted SDK automatically routes to `addLiquiditySingleToken`. Calldata from `response.routes[0].tx.data`, submit via `wallet contract-call`.

---

#### remove-liquidity（removeLiquiditySingleToken）

**Hosted SDK 请求**: `tokenIn` = LP 地址，`tokenOut` = 底层 token 地址。Hosted SDK routes to `removeLiquiditySingleToken`.

---

#### mint-py（mintPyFromToken）

**Hosted SDK 请求**:
```json
{
  "inputs": [{"tokenIn": "<underlying_token>", "amountIn": "<wei>"}],
  "outputs": [
    {"tokenOut": "<pt_address>", "minAmountOut": "0"},
    {"tokenOut": "<yt_address>", "minAmountOut": "0"}
  ],
  "receiver": "<user_wallet>",
  "slippage": 0.005
}
```

---

#### redeem-py（redeemPyToToken）

**Hosted SDK 请求**:
```json
{
  "inputs": [
    {"tokenIn": "<pt_address>", "amountIn": "<pt_amount_wei>"},
    {"tokenIn": "<yt_address>", "amountIn": "<yt_amount_wei>"}
  ],
  "outputs": [{"tokenOut": "<underlying_token>", "minAmountOut": "0"}],
  "receiver": "<user_wallet>",
  "slippage": 0.005
}
```

---

## §3 用户场景

### 场景 1：购买固定收益（buy PT）

**用户说**: "帮我在 Arbitrum 上用 1000 USDC 买入 Pendle stETH 市场的 PT，锁定固定收益"

**Agent 动作序列**:

1. [链下查询] 调用 `onchainos wallet addresses` 获取用户 EVM 钱包地址
2. [链下查询] 调用 `onchainos token search --keyword USDC --chain 42161` 解析 USDC 合约地址和精度
3. [链下查询] `GET /v2/markets/all?chainId=42161&isActive=true` — 搜索包含 "stETH" 或用户指定关键词的市场
4. [链下查询] 展示匹配市场列表（名称、implied APY、TVL、到期日），请用户确认目标市场
5. [链下查询] 调用 `GET /v1/prices/assets?chainId=42161&ids=<PT_ADDRESS>` 获取 PT 当前价格，计算预期 PT 数量
6. [链下查询] `POST /v3/sdk/42161/convert` — 请求 `tokenIn=USDC, amountIn=1000 * 10^6, tokenOut=<PT_ADDRESS>, slippage=0.01` 生成交易 calldata
7. 向用户展示：花费 1000 USDC → 收到约 X PT，隐含固定 APY = Y%，到期日 = Z，价格影响
8. **请用户确认** 是否继续
9. [链上操作] 检查 `response.requiredApprovals`，若需要对 USDC approve Router：
   ```
   onchainos wallet contract-call --chain 42161 --to <USDC_ADDRESS> --input-data 0x095ea7b3...(Router地址+max_uint256) --from <WALLET>
   ```
10. [链上操作] 提交主交易（用户确认后）:
    ```
    onchainos wallet contract-call --chain 42161 --to 0x888888888889758F76e7103c6CbF23ABbF58F946 --input-data <calldata> --from <WALLET> --force
    ```
11. 展示交易哈希，确认 PT 已到账

---

### 场景 2：查询持仓与市场行情

**用户说**: "查一下我在 Pendle 上的所有持仓，还有每个市场现在的 APY"

**Agent 动作序列**:

1. [链下查询] 调用 `onchainos wallet addresses` 获取用户钱包地址
2. [链下查询] `GET /v1/dashboard/positions/database/{user}` — 获取用户所有链上的 Pendle 持仓（PT、YT、LP）
3. [链下查询] 对每个持仓的 `marketAddress`，调用 `GET /v2/markets/all?chainId=<chainId>` 获取该市场的当前 implied APY 和 TVL
4. [链下查询] `GET /v1/prices/assets?ids=<所有PT/YT/LP地址>` — 批量获取 USD 价格
5. 格式化展示：
   - 每个持仓的链、市场名称、持有 PT/YT/LP 数量及 USD 价值
   - 当前市场 implied APY（固定收益率）
   - 到期日、剩余时间
   - 总持仓 USD 价值汇总

---

### 场景 3：添加流动性

**用户说**: "我想在 Base 上往 Pendle USDC 市场添加 500 USDC 的流动性"

**Agent 动作序列**:

1. [链下查询] 获取用户钱包地址 (`onchainos wallet addresses`)
2. [链下查询] `GET /v2/markets/all?chainId=8453&isActive=true` — 列出 Base 上的活跃市场，过滤包含 USDC 的市场
3. 展示匹配市场，请用户确认目标市场（含当前 LP APY、TVL）
4. [链下查询] `onchainos wallet balance --chain 8453` — 验证 USDC 余额 >= 500
5. [链下查询] `POST /v3/sdk/8453/convert` — 请求 `tokenIn=USDC_address, amountIn=500*10^6, tokenOut=<LP_ADDRESS>, slippage=0.005`
6. 向用户展示：预期获得 LP 数量、当前 LP APY、价格影响
7. **请用户确认** 是否继续
8. [链上操作] 若需 approve USDC:
   ```
   onchainos wallet contract-call --chain 8453 --to <USDC_BASE> --input-data 0x095ea7b3...<Router_no0x>...ffffffff --from <WALLET>
   ```
9. [链上操作] 提交 addLiquidity（用户确认后）:
   ```
   onchainos wallet contract-call --chain 8453 --to 0x888888888889758F76e7103c6CbF23ABbF58F946 --input-data <calldata> --from <WALLET> --force
   ```
10. 展示交易哈希和收到的 LP token 数量

---

### 场景 4：到期前出售 PT

**用户说**: "Pendle 市场快到期了，帮我把 Arbitrum 上的 PT-stETH 全部卖掉换回 ETH"

**Agent 动作序列**:

1. [链下查询] 获取用户钱包地址
2. [链下查询] `GET /v1/dashboard/positions/database/{user}` — 获取用户 PT 持仓，找到目标 PT-stETH
3. [链下查询] `GET /v1/prices/assets?chainId=42161&ids=<PT_ADDRESS>` — 获取当前 PT 价格，计算可获 ETH 数量
4. 检查市场到期时间 — 若已到期，建议用 `redeem-py` 而非 sell-pt（更少滑点）
5. [链下查询] `POST /v3/sdk/42161/convert` — 请求 `tokenIn=PT_ADDRESS, amountIn=<全部PT余额>, tokenOut=WETH_ADDRESS, slippage=0.01`
6. 展示：卖出 X PT → 预期收到 Y ETH，价格影响
7. **请用户确认** 是否继续（警告：若价格影响 > 5% 则特别提示）
8. [链上操作] 若需 approve PT:
   ```
   onchainos wallet contract-call --chain 42161 --to <PT_ADDRESS> --input-data 0x095ea7b3...<Router_no0x>...ffffffff --from <WALLET>
   ```
9. [链上操作] 提交 sell-PT 交易（用户确认后）:
   ```
   onchainos wallet contract-call --chain 42161 --to 0x888888888889758F76e7103c6CbF23ABbF58F946 --input-data <calldata> --from <WALLET> --force
   ```
10. 展示交易哈希，确认 ETH 已到账

---

## §4 外部 API 依赖

| API | Base URL | 用途 | 认证 |
|-----|----------|------|------|
| Pendle Core API | `https://api-v2.pendle.finance/core` | 市场数据、资产价格、用户持仓、交易历史 | 可选 Bearer token（免费 100 CU/min）；`Authorization: Bearer <API_KEY>` |
| Pendle Hosted SDK | `https://api-v2.pendle.finance/core/v3/sdk/{chainId}/convert` | 生成所有链上操作的交易 calldata | 同上（共享 rate limit） |
| Ethereum RPC | `https://cloudflare-eth.com` | 链上数据查询（链 1） | 无 |
| BSC RPC | `https://bsc-rpc.publicnode.com` | 链上数据查询（链 56） | 无 |
| Base RPC | `https://base-rpc.publicnode.com` | 链上数据查询（链 8453） | 无 |
| Arbitrum RPC | `https://arb1.arbitrum.io/rpc` | 链上数据查询（链 42161） | 无 |

**Rate limits (Pendle API, free tier)**: 100 CU/minute, 200,000 CU/week. Most endpoints cost 1–5 CU each.

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 描述 |
|------|------|--------|------|
| `pendle_api_key` | `Option<String>` | `None` | Pendle API Bearer token（可选，无 key 也可使用，但 rate limit 更低） |
| `default_slippage` | `f64` | `0.01` | 默认滑点容差（1%），适用于 swap/add/remove liquidity |
| `default_chain_id` | `u64` | `42161` | 默认链（Arbitrum，Pendle TVL 最大） |
| `max_price_impact_warn` | `f64` | `0.05` | 价格影响超过此值（5%）时发出警告 |
| `max_price_impact_block` | `f64` | `0.15` | 价格影响超过此值（15%）时阻止交易 |
| `dry_run` | `bool` | `false` | 启用时仅模拟，不提交链上交易（在 plugin wrapper 层处理，不传给 onchainos CLI） |
| `pendle_api_base_url` | `String` | `"https://api-v2.pendle.finance/core"` | Pendle API base URL（测试时可覆盖） |

---

## §6 技术架构说明

### 核心数据流

```
用户请求
  ↓
[命令解析] — 识别操作类型（list/query/write）
  ↓ (链下查询)
[Pendle REST API] — 获取市场数据 / 价格 / 持仓
  ↓ (链上写操作)
[Pendle Hosted SDK API POST /v3/sdk/{chainId}/convert]
  → 返回: calldata + requiredApprovals
  ↓
[approve 检查] — 若需要，先提交 ERC-20 approve
  ↓
[用户确认]
  ↓
[onchainos wallet contract-call --to <ROUTER> --input-data <calldata> --force]
  → 返回: txHash
```

### 关键注意事项

1. **Router 地址统一**: `0x888888888889758F76e7103c6CbF23ABbF58F946` 在所有支持链上相同，已通过 Etherscan/Arbiscan/OP Etherscan 验证。运行时仍应从 Pendle API `/v1/chains` 动态确认链支持。

2. **Hosted SDK calldata 直接使用**: Pendle Hosted SDK 返回的 `routes[0].tx.data` 即为完整 calldata，直接传入 `--input-data`，无需手动 ABI 编码。

3. **approve 流程**: 检查 `requiredApprovals` 数组 — 若非空，对每个 token 构造 `approve(Router, uint256.max)` calldata (`0x095ea7b3` + Router地址32字节 + `ffffffff...`)，通过 `wallet contract-call` 提交，等待确认后再提交主交易。

4. **dry_run 处理**: 在 plugin wrapper 层实现 early-return，**不**将 `--dry-run` 传给 `onchainos wallet contract-call`（该 flag 不被支持）。

5. **链名称 vs 链 ID**: `defi` 命令用链名称字符串（"arbitrum"），`wallet contract-call` 用数字链 ID（42161）。

6. **到期处理**: 市场到期后，PT 可 1:1 赎回底层资产。应检查 `expiry` 时间戳，到期后建议用 `redeem-py` 而非 `sell-pt`（避免滑点）。

7. **PENDLE token 质押 (vePENDLE/sPENDLE)**: 本版本不实现 PENDLE 锁仓/质押，因为涉及复杂的 vePENDLE 合约交互，可作为 v0.2.0 扩展功能。

---

## §7 参考资料

- Pendle V2 API Docs (Swagger): https://api-v2.pendle.finance/core/docs
- Pendle Developer Docs: https://docs.pendle.finance/pendle-v2/Developers/Backend/HostedSdk
- Pendle API Overview: https://docs.pendle.finance/pendle-v2/Developers/Backend/ApiOverview
- Pendle Router Integration Guide: https://docs.pendle.finance/pendle-v2/Developers/Contracts/PendleRouter/ContractIntegrationGuide
- Pendle Deployments: https://docs.pendle.finance/pendle-v2/Developers/Deployments
- Pendle Core GitHub: https://github.com/pendle-finance/pendle-core-v2-public
- Router V4 on Etherscan: https://etherscan.io/address/0x888888888889758f76e7103c6cbf23abbf58f946
- Router V4 on Arbiscan: https://arbiscan.io/address/0x888888888889758F76e7103c6CbF23ABbF58F946
