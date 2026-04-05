# Solayer Plugin — Design Document

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `solayer` |
| `dapp_name` | Solayer |
| `target_chains` | Solana (501) |
| `target_protocols` | Liquid staking / Restaking |
| `category` | defi-protocol |
| `version` | 0.1.0 |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 否 — 官方仅有 TypeScript CLI (`solayer-labs/solayer-cli`) |
| SDK 支持哪些技术栈？ | TypeScript (Node.js) |
| 有 REST API？ | **是** — `https://app.solayer.org/api/partner/restake/ssol` (GET) 返回 base64 序列化交易 |
| 有官方 Skill？ | 否 |
| 开源社区有类似 Skill？ | 否 |
| 支持哪些链？ | Solana mainnet only |
| 是否需要 onchainos 广播？ | **是** — stake 操作通过 `onchainos wallet contract-call --chain 501 --unsigned-tx <base58_tx> --force` 广播 |

**接入路径：API**（用 Rust 调 REST API，获取 serialized tx，通过 onchainos 广播）

---

## §2 接口映射

### 需要接入的操作表

| 操作 | 链上/链下 | 说明 |
|------|---------|------|
| `rates` | 链下查询 | 获取 sSOL/SOL 汇率、APY、TVL |
| `positions` | 链下查询 | 查询用户的 sSOL 余额及价值 |
| `stake` | 链上写操作 | SOL → sSOL（通过 REST API 获取 serialized tx） |
| `unstake` | 链上写操作 | sSOL → SOL（通过复杂 on-chain SDK 指令；暂时 dry-run only） |

### 链下查询表

**rates** — 获取汇率和协议信息

- Endpoint: `GET https://app.solayer.org/api/info`
- 参数：无
- 返回字段：
  - `apy`: float — sSOL 当前 APY (%)
  - `ssol_to_sol`: float — 1 sSOL 对应的 SOL 数量
  - `tvl_sol`: string — 协议 TVL (SOL)
  - `tvl_usd`: string — 协议 TVL (USD)
  - `epoch`: int — 当前 epoch
  - `epoch_diff_time`: string — epoch 剩余时间
  - `ssol_holders`: int — sSOL 持有人数

**positions** — 查询用户 sSOL 余额

- 方式：通过 `onchainos wallet balance --chain 501` 获取所有 token，过滤 sSOL (mint: `sSo14endRuUbvQaJS3dq36Q829a3A6BEfoeeRGJywEh`)
- 如果 wallet 没有 sSOL token account，返回 balance=0
- 结合 `rates` API 计算 sSOL 对应的 SOL 价值

### 链上写操作表

**stake** — SOL → sSOL

- API: `GET https://app.solayer.org/api/partner/restake/ssol?amount={amount}&staker={wallet}&referrerkey={wallet}`
  - `amount`: float — UI 单位 SOL（如 0.001）
  - `staker`: string — 用户的 base58 Solana 地址
  - `referrerkey`: string — 推荐人地址（可与 staker 相同）
- 响应字段：
  - `transaction`: string — **base64** 编码的序列化交易（需转换为 base58）
  - `message`: string — 确认消息（如 "restaking 0.001 SOL for 0.000874098 sSOL"）
- onchainos 命令：
  ```
  onchainos wallet contract-call --chain 501 \
    --to sSo1iU21jBrU9VaJ8PJib1MtorefUV4fzC9GURa2KNn \
    --unsigned-tx <base58_tx> \
    --force
  ```
- ⚠️ API 返回 base64，onchainos 要求 base58 — 必须在代码中转换

**unstake** — sSOL → SOL

- 需要复杂的多指令 on-chain 交易（unrestake + 创建 stake account + withdrawStake + deactivate）
- 没有可用的 REST API endpoint（`/api/partner/unrestake/ssol` 返回 500 错误）
- 实现方案：标记为 dry-run only，显示说明信息；未来可通过 Solana SDK 直接构建交易
- L4 测试：⚠️ SKIP（API 不可用）

---

## §3 用户场景

### 场景 1：查询当前质押利率

**用户对 Agent 说：** "Show me Solayer staking rates"

**Agent 动作序列：**
1. 调用 `solayer rates --chain 501`
2. `GET https://app.solayer.org/api/info` → 返回 APY、汇率等
3. 显示：当前 sSOL APY、1 sSOL = X SOL、当前 TVL

### 场景 2：质押 SOL 获得 sSOL

**用户对 Agent 说：** "Stake 0.001 SOL on Solayer"

**Agent 动作序列：**
1. 先 dry-run：`solayer stake --amount 0.001 --chain 501 --dry-run`
2. **Ask user to confirm** before proceeding
3. 解析用户钱包地址：`resolve_wallet_solana()`
4. 调用 `GET /api/partner/restake/ssol?amount=0.001&staker={wallet}&referrerkey={wallet}`
5. 从响应中提取 `transaction` 字段（base64）
6. 转换为 base58
7. 调用 `onchainos wallet contract-call --chain 501 --to sSo1iU21jBrU9VaJ8PJib1MtorefUV4fzC9GURa2KNn --unsigned-tx <base58_tx> --force`
8. 返回 txHash（`data.txHash`）

### 场景 3：查询 sSOL 持仓

**用户对 Agent 说：** "Check my Solayer positions"

**Agent 动作序列：**
1. 调用 `solayer positions --chain 501`
2. 从 onchainos 获取 wallet balance，查找 sSOL token
3. 调用 rates API 获取汇率
4. 显示：sSOL 数量、折合 SOL 价值、折合 USD 价值

### 场景 4：赎回 sSOL

**用户对 Agent 说：** "Unstake my sSOL on Solayer"

**Agent 动作序列：**
1. 调用 `solayer unstake --amount 0.001 --chain 501 --dry-run`
2. 显示信息：unstake 需要通过 Solayer UI 操作（REST API 不可用）
3. 提供 Solayer app URL：https://app.solayer.org

---

## §4 外部 API 依赖

| API | 用途 | 认证 |
|-----|------|------|
| `https://app.solayer.org/api/info` | 获取 APY、汇率、TVL | 无需认证 |
| `https://app.solayer.org/api/partner/restake/ssol` | 构建 stake 交易 | 无需认证 |
| `https://api.mainnet-beta.solana.com` | 查询 token account 余额 | 无需认证 |

---

## §5 配置参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `chain` | 链 ID | `501` |
| `dry_run` | 模拟运行，不广播 | `false` |
| `amount` | 质押/赎回数量（UI 单位 SOL） | 必填 |

---

## 关键地址

| 名称 | 地址 |
|------|------|
| Restaking Program | `sSo1iU21jBrU9VaJ8PJib1MtorefUV4fzC9GURa2KNn` |
| sSOL Mint | `sSo14endRuUbvQaJS3dq36Q829a3A6BEfoeeRGJywEh` |
| Stake Pool | `po1osKDWYF9oiVEGmzKA4eTs8eMveFRMox3bUKazGN2` |
| Stake Pool Mint (sSOL-raw) | `sSo1wxKKr6zW2hqf5hZrp2CawLibcwi1pMBqk5bg2G4` |
