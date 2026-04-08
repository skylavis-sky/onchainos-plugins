# Meteora DLMM Plugin — Design Document

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `meteora` |
| `dapp_name` | Meteora DLMM |
| `target_chains` | `[501]` (Solana only) |
| `target_protocols` | DLMM (Dynamic Liquidity Market Maker) |
| `plugin_type` | DEX / Liquidity |
| `needs_onchainos_broadcast` | Yes |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | Yes — `meteora-dlmm-sdk` on crates.io (Rust CPI examples); primary write path is TypeScript SDK `@meteora-ag/dlmm` which builds Solana transactions serializable to base64 |
| SDK 支持哪些技术栈？ | TypeScript (primary, `@meteora-ag/dlmm`), Rust (CPI examples / `meteora-dlmm-sdk`), Python (community client in dlmm-sdk repo) |
| 有 REST API？ | Yes — Base URL: `https://dlmm.datapi.meteora.ag`; 7 endpoints covering pools, OHLCV, volume, protocol stats. Rate limit: 30 req/s |
| 有官方 Skill？ | Not found in registry |
| 开源社区有类似 Skill？ | Yes — `fciaf420/Meteora-DLMM-MCP` on GitHub: TypeScript MCP server implementing 18 tools (5 REST-based, 7 SDK read-only, 6 write operations) |
| 支持哪些链？ | Solana only (chain ID 501) |
| 是否需要 onchainos 广播？ | Yes — All write operations (swap, add/remove liquidity, claim fees) build unsigned Solana transactions via the TypeScript SDK. These must be broadcast via `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>` |

### 接入路径

**参考已有 Skill** + **REST API + TypeScript SDK 混合方式**

- 查询操作（pool list, pool detail, OHLCV, stats）: 直接调用 REST API `https://dlmm.datapi.meteora.ag`
- 链上状态查询（active bin, swap quote, user positions）: 通过 `@meteora-ag/dlmm` TypeScript SDK + Solana RPC
- 写操作（swap, add/remove liquidity, claim fees）: TypeScript SDK 构建未签名交易，序列化为 base64，通过 `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>` 广播

参考实现: `fciaf420/Meteora-DLMM-MCP` — 已验证完整的混合架构

---

## §2 操作接口映射

### 2a. 操作列表

| 操作 | 类型 | 链上/链下 | 描述 |
|------|------|-----------|------|
| `get-pools` | 查询 | 链下 | 搜索/列出 DLMM 流动性池，支持按 token pair、TVL、APY 过滤 |
| `get-pool-detail` | 查询 | 链下 | 获取单个池的详情：配置、TVL、bin step、fee 结构 |
| `get-swap-quote` | 查询 | 链下 (SDK via RPC) | 获取 swap 报价：预计输出量、价格影响、routing bins |
| `get-user-positions` | 查询 | 链下 (SDK via RPC) | 查询用户在某池的 LP 仓位、bin 范围、累计费用 |
| `swap` | 写操作 | 链上 | 执行 token 兑换，SDK 构建交易，onchainos 广播 |
| `add-liquidity` | 写操作 | 链上 | 向指定 bin 范围添加流动性（支持 Spot/Curve/BidAsk 策略） |
| `remove-liquidity` | 写操作 | 链上 | 从仓位移除流动性，可选同时 claim 费用并关闭仓位 |
| `claim-fees` | 写操作 | 链上 | 收取累计的 swap 费用和流动性挖矿奖励 |
| `get-protocol-stats` | 查询 | 链下 | 获取协议级别聚合指标：TVL、总交易量、总费用 |

---

### 2b. 链下查询接口

#### `get-pools` — 搜索流动性池

- **Endpoint**: `GET https://dlmm.datapi.meteora.ag/pools`
- **关键参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `page` | int | 页码（默认 1） |
| `page_size` | int | 每页数量（默认 10，最大 100） |
| `sort_key` | string | 排序字段：`tvl`, `volume`, `fee_tvl_ratio`, `apr` |
| `order_by` | string | `asc` 或 `desc` |
| `search_term` | string | 按 token symbol 或池地址搜索 |

- **返回关键字段**:
  - `data[].address` — 池的 Solana pubkey（作为后续 SDK 调用的 lbPair 参数）
  - `data[].name` — 池名称（如 "SOL-USDC"）
  - `data[].tvl` — 总锁仓量（USD）
  - `data[].current_price` — 当前价格
  - `data[].pool_config.bin_step` — bin step（价格精度单位）
  - `data[].pool_config.base_fee_pct` — 基础手续费率
  - `data[].apr` / `data[].apy` — 年化收益率
  - `data[].token_x` / `data[].token_y` — 代币信息（address, symbol, decimals）

---

#### `get-pool-detail` — 获取单池详情

- **Endpoint**: `GET https://dlmm.datapi.meteora.ag/pools/{address}`
- **关键参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `address` | string | 池的 Solana pubkey（路径参数） |

- **返回关键字段**: 同 `get-pools` 单条记录，另包含：
  - `data.pool_config.max_fee_pct`
  - `data.pool_config.protocol_fee_pct`
  - `data.dynamic_fee_pct`
  - `data.reserve_x` / `data.reserve_y` — 储备量（minimal units）
  - `data.cumulative_metrics.volume` / `data.cumulative_metrics.fees` — 累计指标

---

#### `get-swap-quote` — 获取 Swap 报价（SDK via Solana RPC）

- **方式**: TypeScript SDK `@meteora-ag/dlmm`
- **SDK 调用流程**:
  1. `DLMM.create(connection, lbPairPubKey)` — 加载池状态
  2. `dlmm.getBinArrayForSwap(swapYtoX)` — 获取相关 bin arrays
  3. `dlmm.swapQuote(inAmount, swapYtoX, feeBps, binArrays)` — 计算报价
- **关键参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `lbPair` | PublicKey | 池地址（从 get-pools 获取） |
| `inAmount` | BN | 输入金额（minimal units，需乘以 10^decimals） |
| `swapYtoX` | boolean | true = Y→X，false = X→Y |
| `feeBps` | BN | 滑点容忍度（基点，如 50 = 0.5%） |

- **返回关键字段**:
  - `minOutAmount` — 最小输出量（含滑点）
  - `outAmount` — 预计输出量
  - `fee` — 预计手续费
  - `binArraysPubkey` — 用于构建 swap 交易的 bin array 公钥列表

---

#### `get-user-positions` — 查询用户仓位（SDK via Solana RPC）

- **SDK 调用**:
  1. `DLMM.create(connection, lbPairPubKey)`
  2. `dlmm.getPositionsByUserAndLbPair(userPublicKey)`
- **返回关键字段**:
  - `userPositions[].publicKey` — 仓位 pubkey
  - `userPositions[].positionData.totalXAmount` / `totalYAmount` — 两种代币金额
  - `userPositions[].positionData.positionBinData` — 每个 bin 的详情
  - `userPositions[].positionData.feeX` / `feeY` — 累计费用

---

#### `get-protocol-stats` — 协议统计

- **Endpoint**: `GET https://dlmm.datapi.meteora.ag/stats/protocol_metrics`
- **无需参数**
- **返回关键字段**:
  - `tvl` — 协议总锁仓量（USD）
  - `volume_24h` — 24 小时交易量
  - `fees_24h` — 24 小时手续费

---

### 2c. 链上写操作

> **Solana 特殊说明**: Solana 没有 calldata。所有写操作通过 TypeScript SDK (`@meteora-ag/dlmm`) 构建 `Transaction` 对象，序列化为 base64，然后通过 `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>` 广播签名。

---

#### `swap` — 执行代币兑换

**构建流程**:
1. 调用 REST API 确认池存在并获取 token decimals
2. 通过 SDK `DLMM.create(connection, lbPairPubKey)` 加载池状态
3. 调用 `dlmm.getBinArrayForSwap(swapYtoX)` 获取 bin arrays
4. 调用 `dlmm.swapQuote(inAmount, swapYtoX, feeBps, binArrays)` 获取 `minOutAmount` 和 `binArraysPubkey`
5. 调用 `dlmm.swap({ inToken, binArraysPubkey, inAmount, lbPair, user, minOutAmount, outToken })` 获取未签名 `Transaction`
6. 序列化为 base64: `tx.serialize({ requireAllSignatures: false }).toString('base64')`
7. 广播:
   ```
   onchainos wallet contract-call \
     --chain 501 \
     --unsigned-tx <base64_serialized_tx>
   ```

**关键参数**:

| 参数 | 来源 |
|------|------|
| `lbPair` | REST API `get-pools` 返回的 `address` |
| `inToken` | REST API `token_x.address` 或 `token_y.address` |
| `outToken` | 另一个代币的 address |
| `inAmount` | 用户输入 × 10^decimals（BN 类型） |
| `minOutAmount` | `swapQuote()` 返回值（含滑点保护） |
| `user` | `onchainos wallet addresses` 返回的用户 Solana 地址 |

**风控检查**:
- 价格影响 > 5%: 警告用户
- 输入金额 > 余额: 阻止并提示
- MEV 保护: 当 swap 金额 > $1000 时启用 `--jito-unsigned-tx` flag

---

#### `add-liquidity` — 添加流动性

**构建流程**:
1. 检查是否已有仓位（`getPositionsByUserAndLbPair`）
2. 若无仓位，使用 `dlmm.initializePositionAndAddLiquidityByStrategy(...)` 同时初始化和添加
3. 若有仓位，使用 `dlmm.addLiquidityByStrategy(...)` 增加
4. 序列化 Transaction → base64 → `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>`

**关键参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `positionPubKey` | PublicKey | 仓位地址（新建或现有） |
| `user` | PublicKey | 用户钱包地址 |
| `totalXAmount` | BN | X 代币输入量（minimal units） |
| `totalYAmount` | BN | Y 代币输入量（minimal units） |
| `strategy.minBinId` | number | 仓位下限 bin ID |
| `strategy.maxBinId` | number | 仓位上限 bin ID |
| `strategy.strategyType` | StrategyType | `Spot` / `Curve` / `BidAsk` |

---

#### `remove-liquidity` — 移除流动性

**构建流程**:
1. 调用 `getPositionsByUserAndLbPair` 获取用户仓位详情
2. 调用 `dlmm.removeLiquidity({ position, user, fromBinId, toBinId, liquiditiesBpsToRemove, shouldClaimAndClose })`
3. 序列化 → base64 → `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>`

**关键参数**:

| 参数 | 类型 | 说明 |
|------|------|------|
| `position` | PublicKey | 仓位地址（从 get-user-positions 获取） |
| `liquiditiesBpsToRemove` | BN[] | 每个 bin 移除比例（基点，10000 = 100%） |
| `shouldClaimAndClose` | boolean | 是否同时 claim 费用并关闭仓位 |

---

#### `claim-fees` — 收取手续费和奖励

**构建流程**:
1. 调用 `getPositionsByUserAndLbPair` 获取所有仓位
2. 调用 `dlmm.claimAllSwapFee({ owner, positions })` 构建 Transaction 数组
3. 每笔交易依次序列化 → base64 → `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>`

---

## §3 用户场景

### 场景 1: 查询 SOL-USDC 最佳池并执行 Swap

**用户说**: "帮我用 10 SOL 换成 USDC，找最好的 Meteora 池"

**Agent 动作序列**:
1. **[链下查询]** 调用 `onchainos token search --query SOL` 和 `onchainos token search --query USDC` 获取代币地址
2. **[安全检查]** `onchainos security token-scan --address <usdc_addr> --chain 501`
3. **[链下查询]** `GET https://dlmm.datapi.meteora.ag/pools?search_term=SOL-USDC&sort_key=tvl&order_by=desc` — 获取 TVL 最高的 SOL-USDC 池列表
4. **[链下查询]** 展示前 3 个池（TVL、fee rate、APY），等待用户选择或自动选择 TVL 最高的
5. **[链下查询 via SDK]** 加载选定池，调用 `getBinArrayForSwap(false)` + `swapQuote(10 * 10^9, false, 50, binArrays)` — 获取报价
6. **展示报价**: 预计获得 X USDC，价格影响 Y%，手续费 Z SOL。若价格影响 > 5% 则警告
7. **[等待用户确认]**
8. **[链下查询]** `onchainos wallet addresses` — 获取用户 Solana 地址
9. **[链下查询]** `onchainos portfolio token-balances` — 验证 SOL 余额 ≥ 10 SOL + gas
10. **[构建链上交易]** SDK 调用 `dlmm.swap(...)` 构建 Transaction，序列化为 base64
11. **[链上操作]** `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`
12. **展示结果**: 交易 hash，在 Solscan 确认

---

### 场景 2: 为 SOL-USDC 池提供流动性

**用户说**: "我想在 Meteora 的 SOL-USDC 池子里放流动性，各放 5 SOL 和 800 USDC"

**Agent 动作序列**:
1. **[链下查询]** `GET https://dlmm.datapi.meteora.ag/pools?search_term=SOL-USDC&sort_key=tvl` — 获取最高 TVL 的目标池
2. **[链下查询 via SDK]** `DLMM.create(connection, lbPairPubKey)` + `dlmm.getActiveBin()` — 获取当前活跃 bin 和价格
3. **展示**: 当前价格，建议的 bin 范围（±10 bins 居中策略 Spot），预计 APY
4. **[链下查询]** `onchainos wallet addresses` — 获取用户 Solana 地址
5. **[链下查询]** `onchainos portfolio token-balances` — 验证 SOL ≥ 5 + gas，USDC ≥ 800
6. **[链下查询 via SDK]** `getPositionsByUserAndLbPair(userPubKey)` — 检查是否已有仓位
7. **[等待用户确认]** 展示完整参数（策略类型、bin 范围、金额）
8. **[构建链上交易]** 调用 `dlmm.initializePositionAndAddLiquidityByStrategy(...)` 或 `addLiquidityByStrategy(...)` 构建 Transaction
9. **[链上操作]** `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`
10. **展示结果**: 仓位地址，bin 范围，提供的流动性金额，Solscan 链接

---

### 场景 3: 查看仓位并收取手续费收益

**用户说**: "帮我看看我在 Meteora 的仓位收益如何，顺便把手续费收一下"

**Agent 动作序列**:
1. **[链下查询]** `onchainos wallet addresses` — 获取用户 Solana 地址
2. **[链下查询 via SDK]** 对用户已知的池（或 top 池列表）批量调用 `getPositionsByUserAndLbPair(userPubKey)` — 获取所有仓位
3. **展示仓位列表**: 每个仓位的池名称、bin 范围、X/Y 代币金额、累计未收费用 (feeX + feeY)
4. **计算总收益**: 按当前价格将 fee 折算为 USD
5. **[等待用户确认]** "已找到 N 个仓位，累计 fee X SOL + Y USDC (≈ $Z)，是否全部收取？"
6. **[构建链上交易]** `dlmm.claimAllSwapFee({ owner, positions })` — 构建所有 claim 交易数组
7. **[链上操作 - 逐笔]** 对每笔交易:
   ```
   onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx_i>
   ```
   注意: Solana 交易在 60 秒内过期，提示用户立即签名
8. **展示结果**: 每笔 claim 的 tx hash，成功收取的代币数量

---

### 场景 4: 查看 Meteora 协议全局状态和热门池

**用户说**: "Meteora 现在 TVL 多少？有哪些高收益池？"

**Agent 动作序列**:
1. **[链下查询]** `GET https://dlmm.datapi.meteora.ag/stats/protocol_metrics` — 获取协议级别 TVL、交易量、费用
2. **[链下查询]** `GET https://dlmm.datapi.meteora.ag/pools?sort_key=apr&order_by=desc&page_size=10` — 获取 APY 最高的前 10 个池
3. **展示**: 协议总 TVL，24h 交易量；表格列出前 5 高 APY 池（池名、TVL、bin step、fee rate、APY）
4. 若 APY > 50% 则标注"高风险警告"提示

---

## §4 外部 API 依赖

| 依赖 | 类型 | 用途 | 限制 |
|------|------|------|------|
| `https://dlmm.datapi.meteora.ag` | REST API | 池列表、池详情、OHLCV、历史交易量、协议统计 | 30 req/s；无需认证 |
| Solana RPC Endpoint | JSON-RPC | SDK 加载池状态、获取 bin arrays、构建交易、查询仓位 | 取决于用户配置的 RPC provider；推荐 Helius 或 QuickNode |
| `@meteora-ag/dlmm` (npm) | TypeScript SDK | 所有链上状态查询 + 交易构建 | 无速率限制；需 Node.js 18+ |
| `onchainos wallet contract-call` | OnchainOS skill | 广播 Solana 交易（所有写操作） | 需用户登录 wallet |
| `onchainos wallet addresses` | OnchainOS skill | 获取用户 Solana 钱包地址 | 需用户登录 |
| `onchainos portfolio token-balances` | OnchainOS skill | 交易前余额验证 | 最多 20 条 |
| `onchainos security token-scan` | OnchainOS skill | 代币安全检查（交互未知代币时） | 必须在任何 swap 前执行 |
| `onchainos token search` | OnchainOS skill | 通过名称/symbol 解析代币地址 | 合约地址是唯一可信标识符 |

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `rpc_url` | string | `https://api.mainnet-beta.solana.com` | Solana RPC 端点；推荐使用付费节点（Helius/QuickNode）以保证速度 |
| `default_slippage_bps` | number | `50` | 默认滑点容忍度（基点，50 = 0.5%） |
| `price_impact_warn_threshold` | number | `5.0` | 价格影响警告阈值（百分比） |
| `price_impact_block_threshold` | number | `15.0` | 价格影响阻止阈值（百分比，超过则拒绝交易） |
| `mev_protection_threshold_usd` | number | `1000` | 超过此 USD 金额时启用 Jito MEV 保护 |
| `max_pools_display` | number | `5` | 池搜索结果默认展示数量 |
| `default_liquidity_strategy` | string | `Spot` | 默认流动性策略（`Spot` / `Curve` / `BidAsk`） |
| `default_bin_range` | number | `10` | 默认仓位 bin 范围（活跃 bin ± N） |
| `api_base_url` | string | `https://dlmm.datapi.meteora.ag` | Meteora REST API 基础 URL（正常情况下不需修改） |
| `dry_run` | boolean | `false` | 若为 true，构建交易但不广播，仅展示模拟结果（用于调试和测试） |
| `tx_expiry_warn_seconds` | number | `45` | Solana 交易过期前 N 秒警告用户尽快签名（Solana 交易有效期约 60s） |
| `apy_risk_warn_threshold` | number | `50` | 池 APY 超过此值时显示"高风险"警告 |

---

## 附录：关键地址和常量

> **⚠️ 不要硬编码合约地址** — Meteora DLMM 是 Solana 原生程序，池地址在运行时从 REST API 动态获取。

| 常量 | 值 | 来源 |
|------|-----|------|
| Meteora DLMM Program ID | `LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo` | 官方文档 / Solana Explorer 验证 |
| SOL Native Address (Solana) | `11111111111111111111111111111111` | OnchainOS 标准 |
| API Base URL | `https://dlmm.datapi.meteora.ag` | 社区 Skill 验证 |

> Pool addresses (lbPair public keys) MUST be resolved at runtime via the REST API. Never hardcode individual pool addresses.
