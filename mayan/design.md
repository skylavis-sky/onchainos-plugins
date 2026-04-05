# §0 Plugin Meta

```
plugin_name: mayan
dapp_name: Mayan
version: 0.1.0
target_chains: [solana, evm]  # Solana + Ethereum, Arbitrum, Base, BSC, Polygon, Optimism, Avalanche
category: Bridge/Cross-chain Swap
```

---

# §1 接入可行性调研表

| 项目 | 结论 | 备注 |
|------|------|------|
| Rust SDK | 无 | 仅有 TypeScript/JavaScript SDK (`@mayanfinance/swap-sdk`) |
| 其他 SDK | TypeScript SDK (npm) | `npm install @mayanfinance/swap-sdk`，提供 fetchQuote / swapFromSolana / swapFromEvm |
| REST API | 有，可直接调用 | `https://price-api.mayan.finance/v3/` — 支持 quote、token 列表、swap tx 构建 |
| 官方 Skill/Plugin | 无 | Mayan 官方未发布 onchainos / Skill 插件 |
| 社区 Skill | 无 | 未发现 |
| 支持链 | Solana + 多条 EVM | EVM: Ethereum (1), BSC (56), Polygon (137), Avalanche (43114), Arbitrum (42161), Optimism (10), Base (8453); Aptos **不支持** (skip) |
| onchainos 需求 | 需要 | Solana 链上写操作用 `wallet contract-call --unsigned-tx`；EVM 链上写操作用 `wallet contract-call --input-data` |

**接入路径：** API（REST） — 链下查询调用 Mayan price-api；链上执行通过 onchainos CLI 广播预构建交易

---

# §2 接口映射表

## 操作列表

| 操作 | 类型 | 说明 |
|------|------|------|
| `get-quote` | 链下查询 | 获取跨链 swap 报价，返回最优路由（Swift / MCTP / WH） |
| `get-tokens` | 链下查询 | 获取指定链支持的 token 列表 |
| `get-status` | 链下查询 | 通过交易哈希查询 swap 进度 |
| `swap` | 链上写操作 | 发起跨链 swap（Solana→EVM、EVM→Solana、EVM→EVM、Solana→Solana） |

---

## 链下查询

### get-quote

**Endpoint:** `GET https://price-api.mayan.finance/v3/quote`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `amountIn64` | string | 是 | 输入数量（最小单位，base units） |
| `fromToken` | string | 是 | 源 token 合约地址（Solana: mint 地址；EVM: 0x 地址；native ETH 用 `0x0000000000000000000000000000000000000000`） |
| `fromChain` | string | 是 | 源链名称（见链名映射表） |
| `toToken` | string | 是 | 目标 token 合约地址 |
| `toChain` | string | 是 | 目标链名称 |
| `slippageBps` | number | 是 | 滑点（基点，100 = 1%；最大 300） |
| `gasDrop` | number | 否 | 目标链 gas 补充数量（native token，默认 0） |
| `referrer` | string | 否 | Referrer 地址 |
| `referrerBps` | number | 否 | Referrer 费率（基点） |
| `destinationAddress` | string | 否 | 提高报价精度 |
| `fullList` | boolean | 否 | true 返回所有路由（默认 false，返回最快 + 最优 2 条） |
| `apiKey` | string | 否 | 防止 IP 限流 |

**响应关键字段：**

```json
[
  {
    "type": "SWIFT",              // 路由类型: SWIFT | MCTP | WH
    "expectedAmountOut": "123.45",
    "minAmountOut": "120.00",     // 拍卖最小输出
    "minReceived": "119.50",      // 扣除 relayer 费后最小到账
    "price": 1.23,                // 每单位输入对应的输出
    "etaSeconds": 15,             // 预计完成时间（秒）
    "effectiveAmountIn": "100.0",
    "fromToken": { "contract": "...", "symbol": "...", "decimals": 6 },
    "toToken": { "contract": "...", "symbol": "...", "decimals": 6 },
    "fromChain": "solana",
    "toChain": "base",
    "slippageBps": 100,
    "swiftRelayerFee": "0.5",
    "redeemRelayerFee": "0.3",
    "refundRelayerFee": "0.2"
  }
]
```

---

### get-tokens

**Endpoint:** `GET https://price-api.mayan.finance/v3/tokens`

**请求参数：**

| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| `chain` | string | 是 | 链名称（如 `solana`, `ethereum`, `base`） |

**响应：** token 对象数组，每项含 `contract`、`symbol`、`name`、`decimals`、`logoUrl`。

---

### get-status

**Endpoint:** `GET https://explorer-api.mayan.finance/v3/swap/trx/{transactionHash}`

**路径参数：** `transactionHash` — 源链交易哈希（Solana tx signature 或 EVM tx hash）

**响应关键字段：**

| 字段 | 说明 |
|------|------|
| `status` | 当前状态（如 `INPROGRESS`, `SETTLED_ON_SOLANA`, `COMPLETED`, `REFUNDED`） |
| `clientStatus` | 客户端状态：`INPROGRESS` / `COMPLETED` / `REFUNDED` |
| `sourceTxHash` | 源链交易哈希 |
| `fromAmount` | 输入数量 |
| `toAmount` | 到账数量 |
| `fromTokenSymbol` | 源 token 符号 |
| `toTokenSymbol` | 目标 token 符号 |
| `sourceChain` | 源链 |
| `destChain` | 目标链 |
| `destAddress` | 收款地址 |
| `initiatedAt` | 开始时间 |
| `completedAt` | 完成时间（null 表示未完成） |

---

## 链上写操作 — Solana

### swap（Solana 发起）

**流程：**
1. 调用 `GET https://price-api.mayan.finance/v3/quote` 获取报价，选取 `type == "SWIFT"` 或 `"MCTP"` 的最优条目。
2. 调用 `GET https://price-api.mayan.finance/v3/get-swap/solana` 获取预构建的 Solana 交易指令。
3. 将返回的 base64 序列化交易转换为 base58，通过 `onchainos wallet contract-call --unsigned-tx` 广播。

**get-swap/solana 请求参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `amountIn64` | string | 输入数量（base units） |
| `fromToken` | string | 源 token mint 地址 |
| `userWallet` | string | 用户 Solana 钱包地址 |
| `slippageBps` | number | 滑点基点 |
| `chainName` | string | 目标链名称（"ethereum", "base", "arbitrum" 等） |
| `depositMode` | string | `SWIFT` / `SWIFT_GASLESS` / `WITH_FEE` 等（由报价 type 决定） |
| `userLedger` | string | 用户 ledger/ATA 地址（由 SDK 计算） |
| `middleToken` | string | 中间 token（来自报价） |
| `minMiddleAmount` | number | 最小中间数量（来自报价） |
| `referrerAddress` | string | Referrer 地址（可选） |

**get-swap/solana 响应（SolanaClientSwap）：**

```json
{
  "computeBudgetInstructions": [...],
  "setupInstructions": [...],
  "swapInstruction": {
    "programId": "BLZRi6frs4X4DNLw56V4EXai1b6QVESN1BhHBTYM9VcY",
    "accounts": [...],
    "data": "<base64 instruction data>"
  },
  "cleanupInstruction": {...},
  "addressLookupTableAddresses": ["Ff3yi1meWQQ19VPZMzGg6H8JQQeRudiV7QtVtyzJyoht"]
}
```

**注意事项：**
- Mayan API 返回的序列化交易为 **base64**，`--unsigned-tx` 需要 **base58**。须在代码中做 `base64 → bytes → base58` 转换（参见 KB gotchas: `#unsigned-tx-base58`）。
- Solana `wallet balance --chain 501` **不加** `--output json`（参见 KB gotchas: `#solana-no-output-json`）。
- wallet 地址从 `data["details"][0]["tokenAssets"][0]["address"]` 解析（参见 KB gotchas: `#solana-wallet-address-path`）。

**onchainos 调用：**

```bash
# 获取 Solana 钱包地址
onchainos wallet balance --chain 501
# 从 data.details[0].tokenAssets[0].address 取地址

# 广播预构建交易（base58 编码）
onchainos wallet contract-call \
  --chain 501 \
  --to BLZRi6frs4X4DNLw56V4EXai1b6QVESN1BhHBTYM9VcY \
  --unsigned-tx <base58_serialized_tx> \
  --force
```

**关键 Program IDs（Solana）：**

| Program | ID |
|---------|----|
| SWIFT_PROGRAM_ID (v1) | `BLZRi6frs4X4DNLw56V4EXai1b6QVESN1BhHBTYM9VcY` |
| SWIFT_V2_PROGRAM_ID | `mayan34VedncxdK2XobtvWFDXQASUTBXhUVzt2kKgny` |
| MAYAN_PROGRAM_ID (WH Swap) | `FC4eXxkyrMPTjiYUpp4EAnkmwMbQyZ6NDCh1kfLn6vsf` |
| MCTP_PROGRAM_ID | `dkpZqrxHFrhziEMQ931GLtfy11nFkCsfMftH9u6QwBU` |
| FAST_MCTP_PROGRAM_ID | `Gx9rivpS3YR8pBFwMuP6omYqVxunpLvLkNn7ubNyuZZ5` |
| AUCTION_PROGRAM_ID | `8QJmxZcEzwuYmCPy6XqgN2sHcYCcFq6AEfBMJZZuLo5a` |
| LOOKUP_TABLE | `Ff3yi1meWQQ19VPZMzGg6H8JQQeRudiV7QtVtyzJyoht` |

---

## 链上写操作 — EVM

### swap（EVM 发起）

**流程：**
1. 调用 `GET https://price-api.mayan.finance/v3/quote` 获取报价。
2. 调用 `GET https://price-api.mayan.finance/v3/get-swap/evm` 获取 `{ swapRouterAddress, swapRouterCalldata }`。
3. 若 `fromToken` 非 native ETH，先 approve `MAYAN_FORWARDER_CONTRACT` 花费 `amountIn`（检查已有 allowance，避免重复 approve）。
4. 调用 Mayan Forwarder Contract，根据 token 类型选择对应函数。

**get-swap/evm 请求参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `amountIn64` | string | 输入数量（base units） |
| `fromToken` | string | 源 token 合约地址 |
| `forwarderAddress` | string | `0x337685fdaB40D39bd02028545a4FfA7D287cC3E2` |
| `chainName` | string | 源链名称 |
| `middleToken` | string | 中间 token（来自报价） |
| `slippageBps` | number | 滑点基点 |
| `referrerAddress` | string | Referrer（可选） |

**get-swap/evm 响应：**

```json
{
  "swapRouterAddress": "0x...",
  "swapRouterCalldata": "0x..."
}
```

**Mayan Forwarder Contract：**

- **地址（所有 EVM 链统一）：** `0x337685fdaB40D39bd02028545a4FfA7D287cC3E2`
- **Swift EVM Contract（另一入口）：** `0xC38e4e6A15593f908255214653d3D947CA1c2338`

**Forwarder 函数签名：**

| 函数 | 选择器 | 用途 |
|------|--------|------|
| `forwardERC20(address,uint256,(uint256,uint256,uint8,bytes32,bytes32),address,bytes)` | `0x1e4e5f5f` | ERC-20 直接转发到 Mayan 协议 |
| `forwardEth(address,bytes)` | `0x3a871cdd` | Native ETH 直接转发 |
| `swapAndForwardERC20(address,uint256,(uint256,uint256,uint8,bytes32,bytes32),address,bytes,address,uint256,address,bytes)` | `0x2a8e5d8a` | ERC-20 先 DEX swap 再转发（有中间 token） |
| `swapAndForwardEth(uint256,address,bytes,address,uint256,address,bytes)` | `0xd2bcc892` | ETH 先 DEX swap 再转发 |

> **注意：** 选择器来自 Etherscan 合约页面，未经本地 `cast sig` 验证（cast 不可用）。集成时务必通过 Etherscan `Write Contract` 页面或 `cast keccak` 交叉验证。

**PermitParams 结构（forwardERC20 第三参数）：**

```
(uint256 value, uint256 deadline, uint8 v, bytes32 r, bytes32 s)
```

无 permit 时全部填 0（`(0,0,0,0x0...0,0x0...0)`）。

**ERC-20 approve 流程：**

```bash
# 1. 检查已有 allowance（selector: 0xdd62ed3e）
# allowance(address owner, address spender)
# owner = 用户地址，spender = 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2

# 2. 若 allowance 不足，approve max
# approve(address,uint256) selector: 0x095ea7b3
# spender = 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2
# value = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff
onchainos wallet contract-call \
  --chain <chain_id> \
  --to <token_address> \
  --input-data 0x095ea7b3000000000000000000000000337685fdab40d39bd02028545a4ffa7d287cc3e2ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
  --force

# 3. 等待 3 秒后执行 swap

# 4. 调用 forwarder（calldata 由 get-swap/evm API 返回）
onchainos wallet contract-call \
  --chain <chain_id> \
  --to 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2 \
  --input-data <calldata_from_api> \
  --force
```

**注意 Native ETH 调用（forwardEth）：**

```bash
# 发送 ETH value 时须加 --amt（wei 单位）
onchainos wallet contract-call \
  --chain <chain_id> \
  --to 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2 \
  --input-data <calldata_from_api> \
  --amt <amountIn_in_wei> \
  --force
```

**链名映射表（Mayan API chainName vs EVM chain_id）：**

| Mayan chainName | EVM chain_id | Wormhole chain_id |
|-----------------|-------------|-------------------|
| `ethereum` | 1 | 2 |
| `bsc` | 56 | 4 |
| `polygon` | 137 | 5 |
| `avalanche` | 43114 | 6 |
| `arbitrum` | 42161 | 23 |
| `optimism` | 10 | 24 |
| `base` | 8453 | 30 |
| `solana` | 501 (onchainos) | 1 |

---

# §3 用户场景

## 场景 1：Solana USDC → Base USDC（MCTP 路由）

**用户意图：** "把 Solana 上的 100 USDC 桥接到 Base"

**动作序列：**

```
1. 获取钱包地址
   onchainos wallet balance --chain 501
   → data.details[0].tokenAssets[0].address = <solana_wallet>

2. 获取报价
   GET https://price-api.mayan.finance/v3/quote
     amountIn64 = "100000000"     (100 USDC, 6 decimals)
     fromToken  = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"  (Solana USDC)
     fromChain  = "solana"
     toToken    = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"     (Base USDC)
     toChain    = "base"
     slippageBps = 50
   → 选择 type == "MCTP" 条目，记录 expectedAmountOut、etaSeconds

3. 获取 Solana swap 交易
   GET https://price-api.mayan.finance/v3/get-swap/solana
     amountIn64    = "100000000"
     fromToken     = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
     userWallet    = <solana_wallet>
     slippageBps   = 50
     chainName     = "base"
     depositMode   = "WITH_FEE"
     ...
   → 返回序列化交易（base64）

4. base64 → bytes → base58 转换

5. 广播交易
   onchainos wallet contract-call \
     --chain 501 \
     --to dkpZqrxHFrhziEMQ931GLtfy11nFkCsfMftH9u6QwBU \
     --unsigned-tx <base58_tx> \
     --force
   → 记录 sourceTxHash

6. （可选）轮询状态
   GET https://explorer-api.mayan.finance/v3/swap/trx/<sourceTxHash>
   → clientStatus = "COMPLETED"
```

---

## 场景 2：Arbitrum ETH → Solana SOL（Swift 路由）

**用户意图：** "把 0.01 ETH 从 Arbitrum 兑换成 Solana 上的 SOL"

**动作序列：**

```
1. 获取 Arbitrum 钱包地址（EVM）
   onchainos wallet balance --chain 42161 --output json
   → data.address = <evm_wallet>

2. 获取 Solana 钱包地址
   onchainos wallet balance --chain 501
   → data.details[0].tokenAssets[0].address = <solana_wallet>

3. 获取报价
   GET https://price-api.mayan.finance/v3/quote
     amountIn64  = "10000000000000000"  (0.01 ETH, 18 decimals)
     fromToken   = "0x0000000000000000000000000000000000000000"  (native ETH)
     fromChain   = "arbitrum"
     toToken     = "So11111111111111111111111111111111111111112"  (wSOL)
     toChain     = "solana"
     slippageBps = 100
   → 选择 type == "SWIFT" 条目

4. 获取 EVM swap calldata
   GET https://price-api.mayan.finance/v3/get-swap/evm
     amountIn64       = "10000000000000000"
     fromToken        = "0x0000000000000000000000000000000000000000"
     forwarderAddress = "0x337685fdaB40D39bd02028545a4FfA7D287cC3E2"
     chainName        = "arbitrum"
     slippageBps      = 100
     ...
   → swapRouterCalldata（以 0x3a871cdd 开头 = forwardEth）

5. 广播 EVM 交易（native ETH，须带 --amt）
   onchainos wallet contract-call \
     --chain 42161 \
     --to 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2 \
     --input-data <swapRouterCalldata> \
     --amt 10000000000000000 \
     --force
   → 记录 sourceTxHash

6. 查询状态
   GET https://explorer-api.mayan.finance/v3/swap/trx/<sourceTxHash>
```

---

## 场景 3：Base USDC → Solana USDC（MCTP 路由，EVM 发起）

**用户意图：** "从 Base 桥接 50 USDC 到 Solana"

**动作序列：**

```
1. 获取 Base 钱包地址
   onchainos wallet balance --chain 8453 --output json
   → data.address = <evm_wallet>

2. 获取 Solana 钱包地址
   onchainos wallet balance --chain 501
   → <solana_wallet>

3. 获取报价
   GET https://price-api.mayan.finance/v3/quote
     amountIn64  = "50000000"  (50 USDC, 6 decimals)
     fromToken   = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"  (Base USDC)
     fromChain   = "base"
     toToken     = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"  (Solana USDC)
     toChain     = "solana"
     slippageBps = 50

4. 获取 EVM calldata
   GET https://price-api.mayan.finance/v3/get-swap/evm
     amountIn64       = "50000000"
     fromToken        = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
     forwarderAddress = "0x337685fdaB40D39bd02028545a4FfA7D287cC3E2"
     chainName        = "base"
     slippageBps      = 50

5. 检查 USDC allowance（spender = 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2）
   若 allowance < 50000000，执行 approve：
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
     --input-data 0x095ea7b3000000000000000000000000337685fdab40d39bd02028545a4ffa7d287cc3e2ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
     --force
   sleep(3s)

6. 广播 swap
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2 \
     --input-data <swapRouterCalldata> \
     --force

7. 查询状态
   GET https://explorer-api.mayan.finance/v3/swap/trx/<txHash>
```

---

## 场景 4：Ethereum WETH → Base USDC（EVM→EVM，Swift 路由）

**用户意图：** "把以太坊上的 0.05 WETH 换成 Base 上的 USDC"

**动作序列：**

```
1. 获取 Ethereum 钱包地址
   onchainos wallet balance --chain 1 --output json
   → data.address = <evm_wallet>

2. 获取报价（fromChain ethereum → toChain base）
   GET https://price-api.mayan.finance/v3/quote
     fromToken   = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"  (WETH on Ethereum)
     fromChain   = "ethereum"
     toToken     = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"  (USDC on Base)
     toChain     = "base"
     amountIn64  = "50000000000000000"  (0.05 WETH, 18 decimals)
     slippageBps = 100

3. 获取 EVM calldata
   GET https://price-api.mayan.finance/v3/get-swap/evm
     amountIn64       = "50000000000000000"
     fromToken        = "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"
     forwarderAddress = "0x337685fdaB40D39bd02028545a4FfA7D287cC3E2"
     chainName        = "ethereum"
     slippageBps      = 100

4. 检查 WETH allowance，若不足则 approve（与场景 3 步骤 5 相同）

5. 广播 swap
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2 \
     --input-data <swapRouterCalldata> \
     --force

6. 查询状态
   GET https://explorer-api.mayan.finance/v3/swap/trx/<txHash>
```

---

# §4 外部 API 依赖

| API | Base URL | 用途 | 认证 |
|-----|----------|------|------|
| Mayan Price API | `https://price-api.mayan.finance/v3` | quote、token 列表、swap 交易构建 | 可选 API key（防限流） |
| Mayan Explorer API | `https://explorer-api.mayan.finance/v3` | swap 状态查询、EVM tx 提交 | 无 |
| Mayan Chain Init API | `https://sia.mayan.finance/v10/init` | 链配置、支持状态 | 无 |
| Mayan Gas Estimate API | `https://gas-estimate.mayan.finance/v2` | EVM gas 估算 | 无 |

**API key：** 需向 `support@mayan.finance` 申请，免费 tier 有 IP 限流。

---

# §5 配置参数

```toml
[mayan]
# Mayan API key（可选，防止 IP 限流）
api_key = ""

# 默认滑点（基点，100 = 1%）
default_slippage_bps = 100

# 最大允许滑点（基点）
max_slippage_bps = 300

# 默认路由偏好（"SWIFT" | "MCTP" | "WH" | "auto"）
# auto = 由 API 选择最优路由
preferred_route = "auto"

# 目标链 gas 补充（native token 数量，0 = 不补充）
default_gas_drop = 0.0

# Referrer 地址（可选，留空不设置）
referrer_address = ""

# Referrer 费率（基点，0 = 不收取）
referrer_bps = 0

# Mayan Forwarder Contract（所有 EVM 链统一，勿改）
mayan_forwarder_contract = "0x337685fdaB40D39bd02028545a4FfA7D287cC3E2"

# 状态轮询间隔（秒）
status_poll_interval_secs = 10

# 状态轮询最大次数（超时判定）
status_poll_max_attempts = 60

# 是否 dry-run（不广播交易，只打印 calldata/tx）
dry_run = false
```

**推荐 RPC 端点（EVM）：**

```toml
[rpc]
ethereum  = "https://ethereum.publicnode.com"
base      = "https://base-rpc.publicnode.com"
arbitrum  = "https://arb1.arbitrum.io/rpc"
optimism  = "https://optimism.publicnode.com"
polygon   = "https://polygon-rpc.com"
bsc       = "https://bsc-rpc.publicnode.com"
avalanche = "https://avalanche-c-chain-rpc.publicnode.com"
```

---

# §6 已知限制与注意事项

1. **Aptos 不支持：** onchainos 不支持 Aptos，Mayan 的 Aptos 路由一律跳过。

2. **Solana `--unsigned-tx` 需要 base58：** Mayan API 返回 base64；代码中必须做转换（`base64 → bytes → bs58::encode`），参见 KB `#unsigned-tx-base58`。

3. **Solana `wallet balance` 不加 `--output json`：** chain 501 直接返回 JSON，加参数会 EOF 报错，参见 KB `#solana-no-output-json`。

4. **Solana 钱包地址路径：** `data["details"][0]["tokenAssets"][0]["address"]`，不是 `data.address`，参见 KB `#solana-wallet-address-path`。

5. **EVM approve + swap 须有 3 秒间隔：** 避免 nonce 冲突，参见 dex.md `#allowance-check`。

6. **ERC-20 swap 前先检查 allowance：** 避免重复 approve 导致 nonce 冲突，参见 dex.md `#allowance-check`。

7. **Native ETH 调用须加 `--amt`：** `forwardEth` 函数是 payable，须通过 `--amt <wei>` 传递 ETH value，参见 KB `#eth-native-value-with-contract-call`。

8. **onchainos exit code 2 = 二次确认：** 高额交易可能需要 `--force` 确认，参见 KB `#exit-code-2`。

9. **Swift v1 已过时：** 新集成优先使用 Swift v2（program ID: `mayan34VedncxdK2XobtvWFDXQASUTBXhUVzt2kKgny`）；API 会自动选择版本。

10. **Forwarder 函数选择器未经 cast 本地验证：** `cast` 在当前环境不可用，选择器来源于 Etherscan；集成时应通过 `cast keccak "functionSig(...)"` 或 Etherscan Write Contract 页交叉验证。
