# Marinade Liquid Plugin PRD

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `marinade` |
| `dapp_name` | Marinade Liquid |
| `target_chains` | Solana (501) |
| `target_protocols` | Liquid Staking |
| `version` | 0.1.0 |
| `category` | defi-protocol |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无官方 Rust SDK。有 TypeScript SDK：https://github.com/marinade-finance/marinade-ts-sdk（Anchor-based，构建 Solana 交易） |
| SDK 支持哪些技术栈？ | TypeScript (官方)；Rust SDK（来自 liquid-staking-program 仓库，但不适合直接调用） |
| 有 REST API？ | ✅ 有限的 REST API：`api.marinade.finance/msol/price_sol`（实测可用，返回 mSOL/SOL 汇率） |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | 无已知社区 Plugin-Store Skill |
| 支持哪些链？ | Solana mainnet (501) 唯一支持链 |
| 是否需要 onchainos 广播？ | Yes — 所有链上写操作走 `onchainos swap execute --chain 501`（通过 Jupiter 路由 SOL↔mSOL） |

**接入路径：** API — Marinade 没有返回序列化交易的 REST API；TypeScript SDK 基于 Anchor 不适合直接用 Rust 封装。最佳路径是通过 `onchainos swap execute` 用 Jupiter 聚合路由进行 SOL→mSOL（stake）和 mSOL→SOL（unstake），链下查询用 Marinade REST API + Solana RPC。

---

## §2 接口映射

### 2a. 操作列表

| # | 操作 | 类型 | 说明 |
|---|------|------|------|
| 1 | `rates` | 链下查询 | 查询 mSOL/SOL 汇率、APY、TVL |
| 2 | `positions` | 链下查询 | 查询用户 mSOL 持仓余额 |
| 3 | `stake` | 链上写操作 | SOL → mSOL（通过 Jupiter/onchainos swap） |
| 4 | `unstake` | 链上写操作 | mSOL → SOL（通过 Jupiter/onchainos swap） |

### 2b. 链下查询接口

**rates — 获取 mSOL 汇率与 APY**

```
GET https://api.marinade.finance/msol/price_sol
```
- 返回：纯 float 数字，如 `1.3713931272762248`（代表 1 mSOL = 1.371 SOL）
- 注意：此 API 直接返回数字，不是 JSON 对象

**APY 计算方式：**
- mSOL/SOL 汇率随时间增长，APY ≈ 7-8%（可以从 Solana RPC 读取 epoch info 估算，或硬编码显示为 "~7% APY"）
- 参考 DefiLlama Marinade: `https://yields.llama.fi/pools` (filter pool=Marinade)

**Solana RPC — 获取 mSOL 总供应量（TVL 指标）**

```
POST https://api.mainnet-beta.solana.com
body: {"jsonrpc":"2.0","id":1,"method":"getTokenSupply","params":["mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"]}
```
- 返回：`result.value.uiAmount` — mSOL 流通量
- TVL ≈ mSOL 供应量 × mSOL/SOL 汇率 × SOL 价格

**positions — 用户 mSOL 余额**

```
POST https://api.mainnet-beta.solana.com
body: {
  "jsonrpc":"2.0","id":1,"method":"getTokenAccountsByOwner",
  "params": [
    "<wallet_address>",
    {"mint": "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"},
    {"encoding": "jsonParsed"}
  ]
}
```
- 返回：`result.value[0].account.data.parsed.info.tokenAmount.uiAmountString`
- 如果 `result.value` 为空列表，说明钱包没有 mSOL（无 mSOL token account）

### 2c. 链上写操作

**stake — SOL → mSOL**

```
onchainos swap execute
  --chain 501
  --from 11111111111111111111111111111111  (native SOL)
  --to mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So  (mSOL mint)
  --readable-amount <amount_sol>
  --wallet <user_wallet>
  --slippage 1.0
```

- Jupiter 自动路由到 Marinade 池（最优路径）
- txHash 从 `data.swapTxHash` 提取（fallback `data.txHash`）
- `--from` 为原生 SOL 地址（32个1）
- amount 单位：UI 单位（SOL，如 `"0.001"`）

**unstake — mSOL → SOL**

```
onchainos swap execute
  --chain 501
  --from mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So  (mSOL mint)
  --to 11111111111111111111111111111111  (native SOL)
  --readable-amount <amount_msol>
  --wallet <user_wallet>
  --slippage 1.0
```

---

## §3 用户场景

### 场景 1：查询当前质押利率和汇率
**用户说**："Marinade 现在的质押 APY 是多少？mSOL 汇率是多少？"

**Agent 动作：**
1. 调用 `marinade rates` 命令
2. [链下] GET `https://api.marinade.finance/msol/price_sol` → 获取 mSOL/SOL 价格
3. [链下] 调用 Solana RPC `getTokenSupply` 获取 mSOL 总供应量
4. 输出：`{ "msol_per_sol": 1.371, "sol_per_msol": 0.729, "total_msol_supply": "...", "staking_apy_approx": "~7%" }`

### 场景 2：查询持仓余额
**用户说**："我目前持有多少 mSOL？"

**Agent 动作：**
1. 调用 `marinade positions --chain 501`
2. [链下] 从 onchainos 解析 Solana 钱包地址
3. [链下] 调用 Solana RPC `getTokenAccountsByOwner` 查询 mSOL 余额
4. 输出：`{ "wallet": "...", "msol_balance": "0.001234", "sol_value": "0.001690" }`

### 场景 3：质押 SOL 获取 mSOL
**用户说**："帮我质押 0.001 SOL 到 Marinade 获取 mSOL"

**Agent 动作：**
1. [链下] 确认 dry-run 或请用户确认
2. 调用 `marinade stake --amount 0.001 --chain 501`
3. **询问用户确认**后执行
4. [链上] `onchainos swap execute --chain 501 --from 11111...1 --to mSoL... --readable-amount 0.001 --wallet <addr>`
5. 提取 txHash，输出 solscan 链接

### 场景 4：解质押 mSOL 换回 SOL
**用户说**："把我的 0.001 mSOL 换回 SOL"

**Agent 动作：**
1. [链下] 确认持仓
2. 调用 `marinade unstake --amount 0.001 --chain 501`
3. **询问用户确认**后执行
4. [链上] `onchainos swap execute --chain 501 --from mSoL... --to 11111...1 --readable-amount 0.001 --wallet <addr>`
5. 输出 txHash

---

## §4 外部 API 依赖

| API | 用途 | 认证 | 频率限制 |
|-----|------|------|---------|
| `https://api.marinade.finance/msol/price_sol` | mSOL/SOL 汇率 | 无需 | 宽松 |
| `https://api.mainnet-beta.solana.com` | Solana RPC（getTokenSupply, getTokenAccountsByOwner） | 无需 | 公共节点，可能限速 |

---

## §5 配置参数

| 参数 | 默认值 | 说明 |
|------|-------|------|
| `chain` | 501 | Solana mainnet，唯一支持链 |
| `dry_run` | false | dry_run=true 时跳过链上执行 |
| `slippage` | 1.0 | 滑点容忍度（%），stake/unstake swap 用 |

---

## §6 关键地址

| 名称 | 地址 |
|------|------|
| Marinade Program | `MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD` |
| mSOL Mint | `mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So` |
| Native SOL (Jupiter) | `11111111111111111111111111111111` |
| Marinade State Account | `8szGkuLTAux9XMgZ2vtY39jVSowEcpBfFfD8hXSEqdGC` |
