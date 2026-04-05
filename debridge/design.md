# §0 Plugin Meta

```
plugin_name: debridge
dapp_name: deBridge (DLN — Decentralized Liquidity Network)
version: 0.1.0
target_chains: [evm, solana]
category: Bridge/Cross-chain Swap
```

---

# §1 接入可行性调研表

| 项目 | 结论 |
|------|------|
| 接入路径 | API（REST） |
| 核心产品 | DLN — 去中心化流动性网络，支持跨链 swap，无 TVL 设计（逐单结算） |
| EVM 支持 | Ethereum (1), Arbitrum (42161), Base (8453), Optimism (10), BSC (56), Polygon (137), Avalanche (43114) 等 |
| Solana 支持 | chain ID: 7565164（deBridge 内部 ID） |
| onchainos 可行性 | EVM: `wallet contract-call` with `tx.data` from API; Solana: hex→base58 + `wallet contract-call --unsigned-tx` |
| ERC-20 approve 需要 | 是，approve to `tx.to` (DlnSource) before createOrder |
| 主要限制 | Solana tx 需 hex→base58 转换；API 速率：未认证 50 RPM，认证 300 RPM |

---

# §2 接口映射表

## 操作列表

| 操作 | 类型 | 描述 |
|------|------|------|
| `get-quote` | 链下查询 | 获取跨链 swap 报价（不生成 tx） |
| `create-order` | 链上写 | 在源链创建 DLN 跨链订单 |
| `get-order-status` | 链下查询 | 查询订单状态 |
| `get-supported-chains` | 链下查询 | 获取支持的链列表 |

---

## 链下查询

### 1. get-quote（报价，不含 tx 构建）

```
GET https://dln.debridge.finance/v1.0/dln/order/create-tx
```

**用于 get-quote 时的参数（省略 authority/recipient 地址，API 只返回 estimation）：**

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `srcChainId` | string | 是 | 源链 ID（EVM: `1`/`42161`/`8453`；Solana: `7565164`） |
| `srcChainTokenIn` | string | 是 | 源链 token 地址；native ETH = `0x0000000000000000000000000000000000000000`；native SOL = `11111111111111111111111111111111` |
| `srcChainTokenInAmount` | string | 是 | 输入数量（含 decimals），或 `"auto"` |
| `dstChainId` | string | 是 | 目标链 ID |
| `dstChainTokenOut` | string | 是 | 目标链 token 地址 |
| `dstChainTokenOutAmount` | string | 否 | 建议设 `"auto"` 让 solver 匹配最优价格 |
| `prependOperatingExpenses` | boolean | 否 | `true` 将运营费用加入输入量，报价更透明 |

**响应关键字段：**

```json
{
  "estimation": {
    "srcChainTokenIn": {
      "symbol": "USDC",
      "decimals": 6,
      "amount": "1000000",
      "approximateUsdValue": 1.0
    },
    "dstChainTokenOut": {
      "symbol": "USDC",
      "decimals": 6,
      "amount": "995000",
      "recommendedAmount": "995000",
      "approximateUsdValue": 0.995
    },
    "costsDetails": [...]
  },
  "fixFee": "3000000000000000",
  "orderId": "0x...",
  "order": {
    "approximateFulfillmentDelay": 10
  }
}
```

**注：** 省略 `srcChainOrderAuthorityAddress` / `dstChainOrderAuthorityAddress` / `dstChainTokenOutRecipient` 时，API 不返回 `tx` 字段，只返回 estimation（报价模式）。

---

### 2. get-order-status

```
GET https://dln.debridge.finance/v1.0/dln/order/{orderId}/status
```

| 参数 | 类型 | 必需 | 说明 |
|------|------|------|------|
| `orderId` | string (path) | 是 | 由 create-order 返回的订单 ID（0x 十六进制字符串） |

**响应：**

```json
{
  "orderId": "0x...",
  "status": "Created | Fulfilled | SentUnlock | ClaimedUnlock | OrderCancelled | SentOrderCancel | ClaimedOrderCancel"
}
```

**状态说明：**
- `Created` — 订单已创建，等待 solver 履行
- `Fulfilled` — 目标链已完成，用户已收到 token
- `SentUnlock` — solver 发起解锁流程
- `ClaimedUnlock` — solver 已领取源链 token（完成结算）
- `OrderCancelled` — 用户发起取消
- `ClaimedOrderCancel` — 取消完成，源链 token 退回用户

---

### 3. get-order-data（完整订单详情）

```
GET https://dln.debridge.finance/v1.0/dln/order/{orderId}
```

**响应关键字段：**

```json
{
  "orderId": "0x...",
  "status": "Fulfilled",
  "orderStruct": {
    "makerSrc": "0x...",
    "giveOffer": { "chainId": "42161", "tokenAddress": "0x...", "amount": 1000000 },
    "receiverDst": "0x...",
    "takeOffer": { "chainId": "8453", "tokenAddress": "0x...", "amount": 995000 }
  }
}
```

---

### 4. get-supported-chains

```
GET https://dln.debridge.finance/v1.0/supported-chains-info
```

**响应（部分）：**

```json
[
  { "chainId": 1,       "chainName": "Ethereum" },
  { "chainId": 42161,   "chainName": "Arbitrum" },
  { "chainId": 8453,    "chainName": "Base" },
  { "chainId": 7565164, "chainName": "Solana" },
  ...
]
```

---

## 链上写操作 — EVM

### create-order（EVM 源链）

**流程：**
1. 调用 create-tx API（带完整 authority/recipient 地址）获取 `tx.to`、`tx.data`、`tx.value`
2. 若 srcToken 为 ERC-20，先 approve `tx.to`（DlnSource）
3. 用 `wallet contract-call` 提交 `tx.data`

**API 调用（获取 tx）：**

```
GET https://dln.debridge.finance/v1.0/dln/order/create-tx?
  srcChainId=42161
  &srcChainTokenIn=0xaf88d065e77c8cc2239327c5edb3a432268e5831
  &srcChainTokenInAmount=1000000
  &dstChainId=8453
  &dstChainTokenOut=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
  &dstChainTokenOutAmount=auto
  &srcChainOrderAuthorityAddress=<wallet_addr>
  &dstChainOrderAuthorityAddress=<wallet_addr>
  &dstChainTokenOutRecipient=<wallet_addr>
  &prependOperatingExpenses=true
```

**API 响应 tx 字段（EVM）：**

```json
{
  "tx": {
    "to": "0xeF4fB24aD0916217251F553c0596F8Edc630EB66",
    "data": "0x...",
    "value": "3000000000000000"
  },
  "orderId": "0x..."
}
```

**DlnSource 合约地址（同一地址部署在所有支持的 EVM 链）：**

| 链 | Chain ID | DlnSource 地址 |
|----|----------|----------------|
| Ethereum | 1 | `0xeF4fB24aD0916217251F553c0596F8Edc630EB66` |
| Arbitrum | 42161 | `0xeF4fB24aD0916217251F553c0596F8Edc630EB66` |
| Base | 8453 | `0xeF4fB24aD0916217251F553c0596F8Edc630EB66` |
| Optimism | 10 | `0xeF4fB24aD0916217251F553c0596F8Edc630EB66` |
| BSC | 56 | `0xeF4fB24aD0916217251F553c0596F8Edc630EB66` |
| Polygon | 137 | `0xeF4fB24aD0916217251F553c0596F8Edc630EB66` |

**DlnDestination（目标链）：** `0xe7351fd770a37282b91d153ee690b63579d6dd7f`（所有 EVM 链相同）

**DlnSource createOrder 函数签名（参考，实际用 API 生成 calldata）：**

```solidity
function createOrder(
    DlnOrderLib.OrderCreation calldata _orderCreation,
    bytes calldata _affiliateFee,
    uint32 _referralCode,
    bytes memory _permitEnvelope
) external payable returns (bytes32)
```

函数选择器（用 cast sig 验证）：
```
cast sig "createOrder((uint256,bytes,uint256,bytes,uint256,uint256,bytes,uint256,bytes,bytes,bytes),bytes,uint32,bytes)"
```
**注意：实际 calldata 由 API 生成，插件不需要手动 ABI 编码 createOrder。**

**Step 1 — ERC-20 approve（如果 srcToken 非 native ETH）：**

```bash
# approve(address,uint256) selector = 0x095ea7b3
# spender = tx.to (DlnSource = 0xeF4fB24aD0916217251F553c0596F8Edc630EB66)
onchainos wallet contract-call \
  --chain <chain_id> \
  --to <src_token_address> \
  --input-data 0x095ea7b3<spender_padded_32bytes><amount_padded_32bytes> \
  --force
```

**Step 2 — createOrder（提交 API 返回的 tx.data）：**

```bash
onchainos wallet contract-call \
  --chain <chain_id> \
  --to 0xeF4fB24aD0916217251F553c0596F8Edc630EB66 \
  --input-data <tx.data from API> \
  --amt <tx.value in wei>  \  # protocol flat fee，native ETH
  --force
```

**注意：**
- native ETH 作为输入时不需要 approve，但 `tx.value` 会更大（包含 token amount + flat fee）
- approve 与 createOrder 之间建议 sleep 3s（见 gotchas.md multi-step nonce issue）
- 先检查 allowance，若已足够则跳过 approve

---

## 链上写操作 — Solana

### create-order（Solana 源链）

**Solana DLN Program IDs：**

| 合约 | Program ID |
|------|------------|
| DlnSource | `src5qyZHqTqecJV4aY6Cb6zDZLMDzrDKKezs22MPHr4` |
| DlnDestination | `dst5MGcFPoBeREFAA5E3tU5ij8m5uVYwkzkSAbsLbNo` |

**API 调用（Solana 源链）：**

```
GET https://dln.debridge.finance/v1.0/dln/order/create-tx?
  srcChainId=7565164
  &srcChainTokenIn=EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
  &srcChainTokenInAmount=1000000
  &dstChainId=8453
  &dstChainTokenOut=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913
  &dstChainTokenOutAmount=auto
  &srcChainOrderAuthorityAddress=<solana_wallet_pubkey>
  &dstChainOrderAuthorityAddress=<evm_wallet_addr>
  &dstChainTokenOutRecipient=<evm_wallet_addr>
  &prependOperatingExpenses=true
```

**API 响应 tx 字段（Solana 源链）：**

```json
{
  "tx": {
    "data": "0x0100000000000000..."   // hex-encoded VersionedTransaction
  },
  "orderId": "0x..."
}
```

**编码说明：**
- Solana 源链时，`tx.data` 为 **hex 编码**的 VersionedTransaction（非 base64，非 base58）
- `onchainos wallet contract-call --unsigned-tx` 要求 **base58**
- 转换路径：`hex → bytes → base58`

**Rust 转换代码：**

```rust
// hex → bytes → base58
let hex_str = tx_data.trim_start_matches("0x");
let tx_bytes = hex::decode(hex_str)?;
let tx_base58 = bs58::encode(&tx_bytes).into_string();
```

**Cargo.toml 依赖：**

```toml
hex = "0.4"
bs58 = "0.5"
```

**onchainos 调用：**

```bash
onchainos wallet contract-call \
  --chain 501 \
  --to src5qyZHqTqecJV4aY6Cb6zDZLMDzrDKKezs22MPHr4 \
  --unsigned-tx <base58_encoded_tx> \
  --force
```

**注意：**
- Solana `wallet balance --chain 501` 不支持 `--output json`，直接 parse JSON
- 钱包地址路径：`data["details"][0]["tokenAssets"][0]["address"]`

---

# §3 用户场景

## 场景 1：EVM → EVM 跨链 swap（Arbitrum USDC → Base USDC）

**用户输入：**
```
bridge 10 USDC from Arbitrum to Base
```

**执行流程：**
1. 解析 wallet 地址：`onchainos wallet balance --chain 42161 --output json` → `data.address`
2. 调用 get-quote API，srcChainId=42161, dstChainId=8453，展示报价给用户确认
3. 调用 create-tx API 获取 `tx.to`、`tx.data`、`tx.value`
4. 检查 USDC allowance；若不足，approve DlnSource（sleep 3s）
5. 提交 `wallet contract-call --chain 42161 --to 0xeF4fB24aD0916217251F553c0596F8Edc630EB66 --input-data <tx.data> --amt <tx.value> --force`
6. 返回 txHash 和 orderId
7. 可选：轮询 `/v1.0/dln/order/{orderId}/status` 直至 `Fulfilled`

**预期结果：** USDC 在 Base 链到账，预计完成时间约 10s（`approximateFulfillmentDelay`）

---

## 场景 2：Solana → EVM 跨链 swap（Solana USDC → Base USDC）

**用户输入：**
```
bridge 10 USDC from Solana to Base
```

**执行流程：**
1. 解析 Solana wallet 地址：`onchainos wallet balance --chain 501` → `data["details"][0]["tokenAssets"][0]["address"]`
2. 解析 EVM wallet 地址（目标链接收地址）：`onchainos wallet balance --chain 8453 --output json` → `data.address`
3. 调用 create-tx API：srcChainId=7565164, srcChainTokenIn=EPjFWdd5... dstChainId=8453
4. API 返回 hex 编码 VersionedTransaction（`tx.data`）
5. hex → bytes → base58 转换
6. `onchainos wallet contract-call --chain 501 --to src5qyZHqTqecJV4aY6Cb6zDZLMDzrDKKezs22MPHr4 --unsigned-tx <base58_tx> --force`
7. 返回 txHash 和 orderId

**预期结果：** USDC 在 Base 链到账

---

## 场景 3：EVM → Solana 跨链 swap（Base USDC → Solana USDC）

**用户输入：**
```
bridge 10 USDC from Base to Solana
```

**执行流程：**
1. 解析 Base wallet 地址（srcChainOrderAuthorityAddress + approve 用）
2. 解析 Solana wallet 地址（dstChainTokenOutRecipient + dstChainOrderAuthorityAddress）
3. 调用 create-tx API：srcChainId=8453, dstChainId=7565164, dstChainTokenOut=EPjFWdd5...
4. 检查 USDC allowance 并 approve（sleep 3s）
5. 提交 `wallet contract-call --chain 8453 --to 0xeF4fB24... --input-data <tx.data> --amt <tx.value> --force`
6. 返回 txHash 和 orderId

**注意：** Solana 地址作为 `dstChainOrderAuthorityAddress` / `dstChainTokenOutRecipient` 时，传 base58 公钥即可（API 接受）；若目标地址为 PDA，需加 `skipSolanaRecipientValidation=true`

---

# §4 外部 API 依赖

| API | Base URL | 用途 | 认证 |
|-----|----------|------|------|
| DLN Order API | `https://dln.debridge.finance/v1.0` | 报价、创建订单、查询状态 | 无认证（50 RPM）；可申请 accesstoken（300 RPM） |
| Stats/Order Tracking API | `https://stats-api.dln.trade` | 历史订单、筛选查询 | 无 |

**关键端点汇总：**

```
GET /v1.0/dln/order/create-tx       # 报价 + 生成 tx
GET /v1.0/dln/order/{id}/status     # 查询订单状态
GET /v1.0/dln/order/{id}            # 查询完整订单数据
GET /v1.0/supported-chains-info     # 支持的链列表
GET /v1.0/token-list?chainId=<id>   # 支持的 token 列表
```

**注意事项：**
- 订单 API 返回的 tx 需要在 **30 秒内**提交，否则报价可能过期
- 建议在实际提交前先展示报价，用户确认后再构建 tx
- reqwest 需显式读取 HTTPS_PROXY（见 gotchas.md）

---

# §5 配置参数

```rust
// config.rs

// DLN API
pub const DLN_API_BASE: &str = "https://dln.debridge.finance/v1.0";

// deBridge Chain IDs (DLN internal, NOT standard EVM chain IDs for Solana)
pub const DEBRIDGE_CHAIN_ID_SOLANA: &str = "7565164";
pub const DEBRIDGE_CHAIN_ID_ETH: &str = "1";
pub const DEBRIDGE_CHAIN_ID_ARBITRUM: &str = "42161";
pub const DEBRIDGE_CHAIN_ID_BASE: &str = "8453";
pub const DEBRIDGE_CHAIN_ID_OPTIMISM: &str = "10";
pub const DEBRIDGE_CHAIN_ID_BSC: &str = "56";
pub const DEBRIDGE_CHAIN_ID_POLYGON: &str = "137";

// EVM DlnSource — same address on all supported EVM chains
pub const DLN_SOURCE_EVM: &str = "0xeF4fB24aD0916217251F553c0596F8Edc630EB66";

// Solana Program IDs
pub const DLN_SOURCE_SOLANA: &str = "src5qyZHqTqecJV4aY6Cb6zDZLMDzrDKKezs22MPHr4";
pub const DLN_DESTINATION_SOLANA: &str = "dst5MGcFPoBeREFAA5E3tU5ij8m5uVYwkzkSAbsLbNo";

// Native token identifiers
pub const NATIVE_EVM: &str = "0x0000000000000000000000000000000000000000";
pub const NATIVE_SOL: &str = "11111111111111111111111111111111";

// Well-known token addresses
pub const USDC_ARBITRUM: &str  = "0xaf88d065e77c8cc2239327c5edb3a432268e5831";
pub const USDC_BASE: &str      = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
pub const USDC_ETHEREUM: &str  = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
pub const USDC_SOLANA: &str    = "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v";
pub const WETH_ARBITRUM: &str  = "0x82af49447d8a07e3bd95bd0d56f35241523fbab1";
pub const WETH_BASE: &str      = "0x4200000000000000000000000000000000000006";

// RPC endpoints (from gotchas.md)
pub fn rpc_url(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "https://ethereum.publicnode.com",
        42161 => "https://arb1.arbitrum.io/rpc",
        8453  => "https://base-rpc.publicnode.com",
        10    => "https://mainnet.optimism.io",
        56    => "https://bsc-rpc.publicnode.com",
        137   => "https://polygon-rpc.com",
        _     => "https://ethereum.publicnode.com",
    }
}
```

---

# §6 已知风险与注意事项

| 风险 | 描述 | 缓解措施 |
|------|------|----------|
| Solana tx 编码 | API 返回 hex 编码 VersionedTransaction；onchainos 需要 base58 | hex → bytes → base58 转换 |
| tx 过期 | create-tx API 返回的 tx 需 30s 内提交 | 先报价展示，用户确认后立即构建+提交 |
| ERC-20 nonce 冲突 | approve + createOrder 连续调用可能 nonce 冲突 | approve 后 sleep 3s |
| 重复 approve | 多次运行可能重复 approve | 先 check allowance，已足够则跳过 |
| 订单未履行 | solver 可能不履行订单（流动性不足时）| 使用 `dstChainTokenOutAmount=auto` 提高被接单概率 |
| Solana 目标地址验证 | PDA 地址会被 API 拒绝 | 加 `skipSolanaRecipientValidation=true` |
| reqwest HTTPS_PROXY | 代理环境下 reqwest 不读取系统代理 | 使用 `build_client()` 显式传 proxy |
