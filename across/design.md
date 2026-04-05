# §0 Plugin Meta

```yaml
plugin_name: across
dapp_name: Across Protocol
version: 0.1.0
target_chains: [evm]  # Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), Polygon (137)
category: Bridge/Cross-chain
```

---

# §1 接入可行性调研表

| 项目 | 详情 |
|------|------|
| **接入路径** | REST API + 链上写操作（SpokePool.depositV3） |
| **API Base URL** | `https://app.across.to/api` |
| **认证要求** | 生产环境需要 API Key + Integrator ID（联系 Telegram 获取）；基础查询端点无需鉴权 |
| **链上合约** | 每条链上部署了 SpokePool 合约；用户在源链调用 `depositV3`，relayer 在目标链填单 |
| **ERC-20 Approve** | 需要；bridge ERC-20 前需 approve SpokePool 地址；ETH 桥接无需 approve |
| **已知限制** | `isAmountTooLow` 字段指示金额是否过小；有 `minDeposit`/`maxDeposit` 限制 |
| **社区 Skill/MCP** | 官方提供 MCP Server：`https://mcp.across.to/mcp`；npx skills 安装：`npx skills add https://github.com/across-protocol/skills` |

**结论：可接入。** 流程为：① 查询费用（suggested-fees API）→ ② approve ERC-20（如需）→ ③ 调用 SpokePool.depositV3 → ④ 轮询 deposit/status。

---

# §2 接口映射表

## 操作列表

| 操作 | 类型 | 描述 |
|------|------|------|
| `get-quote` | 链下读 | 获取跨链报价（费率、预计金额、fillDeadline、timestamp） |
| `get-limits` | 链下读 | 获取特定 route 的最小/最大转账限制及流动性 |
| `get-routes` | 链下读 | 查询支持的跨链路由（源链/目标链/token） |
| `bridge` | 链上写 | approve ERC-20 + 调用 SpokePool.depositV3 发起跨链 |
| `get-status` | 链下读 | 查询跨链交易填单状态 |

---

## 链下查询

### 1. get-quote（`/api/suggested-fees`）

**GET** `https://app.across.to/api/suggested-fees`

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `inputToken` | address | ✅ | 源链 token 地址 |
| `outputToken` | address | ✅ | 目标链 token 地址 |
| `originChainId` | int | ✅ | 源链 ID |
| `destinationChainId` | int | ✅ | 目标链 ID |
| `amount` | uint256 | ✅ | 转账金额（最小单位，含 decimals） |
| `depositor` | address | ❌ | 存款人地址（可选，用于精确报价） |
| `recipient` | address | ❌ | 目标链接收地址 |

**Response（关键字段）：**

```json
{
  "outputAmount": "999687",           // 目标链实际到账金额（扣费后）
  "totalRelayFee": {
    "pct": "313000000000000",         // 总费率（1e18 = 100%）
    "total": "313"                    // 总费用（token 最小单位）
  },
  "relayerCapitalFee": { "pct": "...", "total": "..." },
  "relayerGasFee":     { "pct": "...", "total": "..." },
  "lpFee":             { "pct": "...", "total": "..." },
  "timestamp": "1775384651",          // 报价时间戳（传给 depositV3 的 quoteTimestamp）
  "fillDeadline": "1775391851",       // 填单截止时间（传给 depositV3 的 fillDeadline）
  "exclusiveRelayer": "0x0000...0000",// 专属 relayer 地址（传给 depositV3）
  "exclusivityDeadline": 0,           // 专属截止时间（传给 depositV3）
  "spokePoolAddress": "0x5c7B...",    // 源链 SpokePool 地址（动态返回，无需硬编码）
  "isAmountTooLow": false,
  "estimatedFillTimeSec": 4,
  "limits": {
    "minDeposit": "500000",
    "maxDeposit": "1297512693112",
    "maxDepositInstant": "184803787351",
    "maxDepositShortDelay": "1297512693112",
    "recommendedDepositInstant": "184803787351"
  },
  "inputToken": { "address": "0x...", "symbol": "USDC", "decimals": 6, "chainId": 1 },
  "outputToken": { "address": "0x...", "symbol": "USDC", "decimals": 6, "chainId": 10 }
}
```

> **重要：** `spokePoolAddress` 从 API 响应动态读取，不要硬编码。

---

### 2. get-limits（`/api/limits`）

**GET** `https://app.across.to/api/limits`

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `inputToken` | address | ✅ | 源链 token 地址 |
| `outputToken` | address | ✅ | 目标链 token 地址 |
| `originChainId` | int | ✅ | 源链 ID |
| `destinationChainId` | int | ✅ | 目标链 ID |

**Response（关键字段）：**

```json
{
  "minDeposit": "500000",
  "maxDeposit": "1297787653112",
  "maxDepositInstant": "184803787351",
  "maxDepositShortDelay": "1297787653112",
  "recommendedDepositInstant": "184803787351",
  "liquidReserves": "1037198680365",
  "utilizedReserves": "774344431924"
}
```

---

### 3. get-routes（`/api/available-routes`）

**GET** `https://app.across.to/api/available-routes`

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `originChainId` | int | ❌ | 过滤源链 |
| `destinationChainId` | int | ❌ | 过滤目标链 |
| `originToken` | address | ❌ | 过滤源 token |
| `destinationToken` | address | ❌ | 过滤目标 token |

**Response（数组，每项包含）：**

```json
[
  {
    "originChainId": 1,
    "originToken": "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
    "destinationChainId": 10,
    "destinationToken": "0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85",
    "originTokenSymbol": "USDC",
    "destinationTokenSymbol": "USDC",
    "isNative": false
  }
]
```

---

### 4. get-status（`/api/deposit/status`）

**GET** `https://app.across.to/api/deposit/status`

查询方式（三选一）：

| 参数 | 说明 |
|------|------|
| `depositTxnRef` | 源链交易哈希 |
| `depositId` + `originChainId` | Deposit ID + 源链 ID |
| `relayDataHash` | 中继数据哈希 |

**Response（关键字段）：**

```json
{
  "status": "filled",        // pending | filled | expired
  "depositId": 12345,
  "originChainId": 1,
  "destinationChainId": 10,
  "depositTxnHash": "0x...",
  "fillTxnHash": "0x...",    // 目标链填单 tx hash
  "depositRefundTxnHash": null
}
```

> 注意：API 响应有 1-15 秒延迟，轮询间隔建议 5 秒。

---

## 链上写操作 — EVM

### SpokePool.depositV3

#### SpokePool 地址（每链）

| 链 | Chain ID | SpokePool 地址 |
|----|----------|----------------|
| Ethereum | 1 | `0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5` |
| Arbitrum | 42161 | `0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A` |
| Base | 8453 | `0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64` |
| Optimism | 10 | `0x6f26Bf09B1C792e3228e5467807a900A503c0281` |
| Polygon | 137 | `0x9295ee1d8C5b022Be115A2AD3c30C72E34e7F096` |

> 实现时优先用 `suggested-fees` 响应中的 `spokePoolAddress` 字段，上表作为后备。

#### depositV3 函数签名

```solidity
function depositV3(
    address depositor,          // 存款人地址（用户钱包）
    address recipient,          // 目标链接收地址
    address inputToken,         // 源链 token 地址
    address outputToken,        // 目标链 token 地址
    uint256 inputAmount,        // 存款金额（token 最小单位）
    uint256 outputAmount,       // 预期目标链到账金额（来自 suggested-fees.outputAmount）
    uint256 destinationChainId, // 目标链 ID
    address exclusiveRelayer,   // 专属 relayer（来自 suggested-fees.exclusiveRelayer）
    uint32 quoteTimestamp,      // 报价时间戳（来自 suggested-fees.timestamp）
    uint32 fillDeadline,        // 填单截止（来自 suggested-fees.fillDeadline）
    uint32 exclusivityDeadline, // 专属截止（来自 suggested-fees.exclusivityDeadline）
    bytes calldata message      // 附加消息（通常为 0x，即空字节）
) external payable;
```

**Function selector（已验证）：** `0x7b939232`

验证命令：
```bash
cast sig "depositV3(address,address,address,address,uint256,uint256,uint256,address,uint32,uint32,uint32,bytes)"
# => 0x7b939232
```

#### ABI 编码方式

参数编码顺序（严格按上述声明顺序）：

```
selector (4 bytes)
+ depositor         (address, 32 bytes, 左 0 填充)
+ recipient         (address, 32 bytes)
+ inputToken        (address, 32 bytes)
+ outputToken       (address, 32 bytes)
+ inputAmount       (uint256, 32 bytes)
+ outputAmount      (uint256, 32 bytes)
+ destinationChainId(uint256, 32 bytes)
+ exclusiveRelayer  (address, 32 bytes)
+ quoteTimestamp    (uint32, 32 bytes)
+ fillDeadline      (uint32, 32 bytes)
+ exclusivityDeadline (uint32, 32 bytes)
+ message offset    (uint256, 32 bytes = 0x180 = 384)
+ message length    (uint256, 32 bytes = 0 for empty)
+ message data      (0 bytes for empty)
```

ETH 桥接时需传 `--amt <inputAmount>` (wei)；ERC-20 桥接时 `--amt 0`。

#### ERC-20 Approve

桥接 ERC-20 前需先 approve SpokePool：

```bash
# approve(address,uint256) selector = 0x095ea7b3
# spender = SpokePool 地址, amount = u128::MAX (无限授权)

onchainos wallet contract-call \
  --chain <originChainId> \
  --to <tokenAddress> \
  --input-data 0x095ea7b3<spender_padded><amount_padded> \
  --output json \
  --force
```

#### bridge 调用示例（ERC-20）

```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5 \
  --input-data 0x7b939232<depositor><recipient><inputToken><outputToken><inputAmount><outputAmount><destinationChainId><exclusiveRelayer><quoteTimestamp><fillDeadline><exclusivityDeadline><message_offset><message_length> \
  --output json \
  --force
```

#### bridge 调用示例（ETH 原生）

ETH 桥接时 inputToken = `0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE`，outputToken 为目标链 WETH 地址，使用 `--amt`：

```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5 \
  --input-data 0x7b939232<...> \
  --amt <inputAmount_in_wei> \
  --output json \
  --force
```

---

# §3 用户场景

## 场景 1：USDC 从 Ethereum 桥接到 Optimism

```
用户: "把 100 USDC 从 Ethereum 桥到 Optimism"

1. resolve_wallet: onchainos wallet balance --chain 1 --output json
   → wallet = data.address

2. GET /api/suggested-fees?inputToken=0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48
     &outputToken=0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85
     &originChainId=1&destinationChainId=10&amount=100000000
   → outputAmount, timestamp, fillDeadline, exclusiveRelayer, exclusivityDeadline, spokePoolAddress

3. 检查 USDC allowance for spokePoolAddress
   如果不足: wallet contract-call approve (sleep 3s)

4. wallet contract-call depositV3 on chain 1
   → txHash

5. GET /api/deposit/status?depositTxnRef=<txHash>&originChainId=1
   → status: filled, fillTxnHash

输出: "已发起桥接，预计 4 秒到账。源链 tx: 0x..., 目标链 tx: 0x..."
```

## 场景 2：WETH 从 Arbitrum 桥接到 Base

```
用户: "把 0.01 WETH 从 Arbitrum 桥到 Base"

1. resolve_wallet: chain 42161

2. GET /api/available-routes?originChainId=42161&destinationChainId=8453
   → 找到 WETH route: inputToken=0x82aF49447D8a07e3bd95BD0d56f35241523fBab1,
     outputToken=0x4200000000000000000000000000000000000006

3. GET /api/suggested-fees?inputToken=...&outputToken=...&originChainId=42161&destinationChainId=8453&amount=10000000000000000

4. approve WETH → SpokePool (Arbitrum: 0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A)

5. depositV3 on Arbitrum

6. 轮询状态（最多 30 秒, 每 5 秒一次）
```

## 场景 3：查询路由支持及限额

```
用户: "Across 支持从 Base 桥 USDC 到 Polygon 吗？最多能桥多少？"

1. GET /api/available-routes?originChainId=8453&destinationChainId=137
   → 找到 USDC route

2. GET /api/limits?inputToken=<base_usdc>&outputToken=<polygon_usdc>
     &originChainId=8453&destinationChainId=137
   → maxDepositInstant, maxDeposit, minDeposit

输出: "支持，当前即时最大桥接额度为 184,803 USDC，最小 0.5 USDC。"
```

## 场景 4：干运行（Dry Run）查询费用

```
用户: "查一下把 1000 USDC 从 Ethereum 桥到 Arbitrum 需要多少手续费"（dry-run）

1. GET /api/suggested-fees (same as 场景1 but destinationChainId=42161)
   → relayFeeTotal, outputAmount

2. 显示费用明细，不执行链上交易

输出:
  输入金额: 1000 USDC
  桥接费用: ~0.31 USDC (资本费 + gas 费)
  预计到账: 999.69 USDC
  预计时间: 4 秒
```

---

# §4 外部 API 依赖

| API | 端点 | 用途 | 限速/注意事项 |
|-----|------|------|---------------|
| Across REST API | `https://app.across.to/api/suggested-fees` | 报价、费率、fillDeadline 等 | 生产需 API Key |
| Across REST API | `https://app.across.to/api/limits` | 转账额度查询 | 无需鉴权 |
| Across REST API | `https://app.across.to/api/available-routes` | 路由可用性查询 | 无需鉴权 |
| Across REST API | `https://app.across.to/api/deposit/status` | 跨链状态轮询 | 1-15s 延迟 |

**reqwest 代理注意：** 使用 `build_client()` 显式读取 `HTTPS_PROXY` 环境变量，避免代理环境下请求失败（参考 gotchas.md）。

---

# §5 配置参数

```rust
// config.rs

/// Across API base URL
pub const ACROSS_API_BASE: &str = "https://app.across.to/api";

/// SpokePool 地址（后备，优先用 API 返回的 spokePoolAddress）
pub fn get_spoke_pool(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5",  // Ethereum
        10    => "0x6f26Bf09B1C792e3228e5467807a900A503c0281",  // Optimism
        137   => "0x9295ee1d8C5b022Be115A2AD3c30C72E34e7F096",  // Polygon
        8453  => "0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64",  // Base
        42161 => "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A",  // Arbitrum
        _     => "",
    }
}

/// 支持链列表
pub const SUPPORTED_CHAINS: &[u64] = &[1, 10, 137, 8453, 42161];

/// 推荐 RPC（参考 gotchas.md）
pub fn get_rpc(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "https://ethereum.publicnode.com",
        10    => "https://optimism-rpc.publicnode.com",
        137   => "https://polygon-rpc.com",
        8453  => "https://base-rpc.publicnode.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        _     => "",
    }
}

/// ETH 原生 token 占位地址（EVM 约定）
pub const ETH_ADDRESS: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

/// depositV3 function selector（已验证）
pub const DEPOSIT_V3_SELECTOR: &str = "0x7b939232";

/// ERC-20 approve selector
pub const APPROVE_SELECTOR: &str = "0x095ea7b3";

/// 状态轮询：最多重试次数 & 间隔（秒）
pub const STATUS_MAX_RETRIES: u32 = 12;
pub const STATUS_POLL_INTERVAL_SECS: u64 = 5;

/// approve → deposit 间等待时间（秒）
pub const APPROVE_DELAY_SECS: u64 = 3;
```

---

# §6 实现注意事项

## 钱包地址获取

```rust
// EVM 链（链 ID 数字）
onchainos wallet balance --chain <chain_id> --output json
// → data.address
```

## 多步骤事务时序

```
1. allowance check (eth_call)
2. if insufficient: approve tx → sleep(3s)
3. depositV3 tx
4. poll /deposit/status every 5s (max 60s)
```

## outputAmount 处理

`suggested-fees` 返回的 `outputAmount` 已扣除全部费用，直接作为 depositV3 的 `outputAmount` 参数传入。若用户要求最小到账保证，可在此值基础上设置滑点（如 `outputAmount * 99 / 100`）。

## message 字段（空消息）

普通桥接传空字节：ABI 编码为 `offset=0x180`（12个参数后的位置），`length=0x00`，无 data。若需跨链执行（embedded actions），则在 message 中编码目标链调用数据。

## isAmountTooLow 检查

调用 `suggested-fees` 后，若 `isAmountTooLow == true`，立即返回用户友好错误，不继续执行链上交易。

## 错误处理

- `exit_code == 2 && confirming == true`：提示用户二次确认，重发加 `--force`
- status 超时（60s 无 filled）：返回 pending 状态 + txHash，建议用户稍后查询
