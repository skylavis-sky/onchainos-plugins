# design.md — Lido Plugin

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `lido` |
| `dapp_name` | Lido |
| `version` | 0.1.0 |
| `target_chains` | Ethereum (1) — primary; Arbitrum (42161), Base (8453), Optimism (10) — wstETH read + wrap/unwrap |
| `target_protocols` | Lido liquid staking (stETH, wstETH, WithdrawalQueue) |
| `binary_name` | `lido` |
| `source_repo` | `skylavis-sky/onchainos-plugins` |
| `source_dir` | `lido` |
| `category` | `defi-protocol` |
| `tags` | `staking`, `liquid-staking`, `ethereum`, `steth`, `wsteth` |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **No.** Lido 没有官方 Rust SDK。官方只有 JS SDK (@lidofinance/lido-ethereum-sdk, https://github.com/lidofinance/lido-ethereum-sdk)。接入方式：直接通过 RPC `eth_call` + `wallet contract-call` 调合约。 |
| SDK 支持哪些技术栈？ | TypeScript/JavaScript（@lidofinance/lido-ethereum-sdk）；Python（无官方）；Rust（无官方） |
| 有 REST API？ | **Yes.** 官方 REST API：APR — `https://eth-api.lido.fi/v1/protocol/steth/apr/sma`；奖励历史 — `https://reward-history-backend.lido.fi/`；提现队列时间 — `https://wq-api.lido.fi/v2/request-time` |
| 有官方 Skill？ | **No.** 无官方 OnchainOS/MCP Skill |
| 开源社区有类似 Skill？ | **No.** 未发现社区 Skill 实现 |
| 支持哪些链？ | Ethereum mainnet（stETH + wstETH + WithdrawalQueue 全功能）；Arbitrum, Base, Optimism, Polygon, zkSync（wstETH ERC-20 持仓 + wrap/unwrap） |
| 是否需要 onchainos 广播？ | **Yes.** stake（`submit()`）、wrap（`wrap()`）、unwrap（`unwrap()`）、requestWithdrawals、claimWithdrawals 均为写操作，必须通过 `onchainos wallet contract-call` 广播 |

**接入路径：API + RPC 直调合约**

无 Rust SDK，无社区 Skill。链下查询通过 Lido REST API + `eth_call` 实现；链上操作通过 `onchainos wallet contract-call` 提交 ABI 编码 calldata。

---

## §2 接口映射

### 2a. 需要接入的操作表

| 操作 | 命令 | 类型 | 链 |
|------|------|------|----|
| 质押 ETH → stETH | `stake` | 链上写 | Ethereum (1) |
| 查询 stETH 余额 / 持仓 | `get-position` | 链下查询 | Ethereum (1) + L2s |
| 查询当前 APR | `get-apr` | 链下查询 | — (REST API) |
| 将 stETH 转换为 wstETH | `wrap` | 链上写 | Ethereum (1) |
| 将 wstETH 转换回 stETH | `unwrap` | 链上写 | Ethereum (1) + L2s |
| 发起提现申请（stETH → ETH） | `request-withdrawal` | 链上写 | Ethereum (1) |
| 查询提现请求状态 | `get-withdrawal-status` | 链下查询 | Ethereum (1) |
| 领取已完成提现 | `claim-withdrawal` | 链上写 | Ethereum (1) |

### 2b. 链下查询表

#### `get-apr` — 查询当前质押 APR

| 字段 | 内容 |
|------|------|
| API Endpoint | `GET https://eth-api.lido.fi/v1/protocol/steth/apr/sma` |
| 参数 | 无 |
| 响应关键字段 | `data.smaApr` (string, %) — 7日均 APR；`data.apr` 数组含每日明细 |

备用：`GET https://eth-api.lido.fi/v1/protocol/steth/apr/last` → `data.apr`（最新单日 APR）

#### `get-position` — 查询持仓与汇率

分三步 `eth_call`：

**1. stETH 余额**

```
合约: 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84  (Lido/stETH, Ethereum)
函数: balanceOf(address)
选择器: 0x70a08231  [来源: eth_utils keccak256("balanceOf(address)")]
calldata: 0x70a08231 + abi_encode(wallet_address)
返回: uint256 (stETH amount, 18 decimals)
```

**2. wstETH 余额**（Ethereum 及各 L2 各自的 wstETH 合约）

```
合约: 见合约地址表（各链不同）
函数: balanceOf(address)
选择器: 0x70a08231
calldata: 0x70a08231 + abi_encode(wallet_address)
返回: uint256 (wstETH amount, 18 decimals)
```

**3. 汇率（stETH per wstETH）**

```
合约: 0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0  (wstETH, Ethereum)
函数: stEthPerToken()
选择器: 0x035faf82  [来源: eth_utils keccak256("stEthPerToken()")]
calldata: 0x035faf82
返回: uint256 (stETH per 1 wstETH, 18 decimals)
```

额外可查（可选）：

```
getTotalPooledEther()
  合约: 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84
  选择器: 0x37cfdaca  [来源: eth_utils keccak256("getTotalPooledEther()")]
  返回: uint256 (协议总质押 ETH)

getCurrentStakeLimit()
  合约: 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84
  选择器: 0x609c4c6c  [来源: eth_utils keccak256("getCurrentStakeLimit()")]
  返回: uint256 (当前可质押 ETH 上限, 0=暂停, 2^256-1=无限)
```

#### `get-withdrawal-status` — 查询提现状态

**1. 查询请求状态**

```
合约: 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1  (WithdrawalQueueERC721)
函数: getWithdrawalStatus(uint256[])
选择器: 0xb8c4b85a  [来源: eth_utils keccak256("getWithdrawalStatus(uint256[])")]
calldata: 0xb8c4b85a + abi_encode(request_ids_array)
返回: WithdrawalRequestStatus[] {
  amountOfStETH: uint256,
  amountOfShares: uint256,
  owner: address,
  timestamp: uint256,
  isFinalized: bool,
  isClaimed: bool
}
```

**2. 查询预计等待时间（REST API）**

```
GET https://wq-api.lido.fi/v2/request-time?ids=<id1>&ids=<id2>
参数: ids (comma-separated request IDs)
返回: [{ requestId, type, status, expectedAt }]  或 { queue: [...], bunkerMode }
```

**3. 为 claimWithdrawals 获取 hint（必须在 claim 前调用）**

```
合约: 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1
函数: getLastCheckpointIndex()
选择器: 0x526eae3e  [来源: eth_utils keccak256("getLastCheckpointIndex()")]
calldata: 0x526eae3e
返回: uint256 (最新 checkpoint 索引)

函数: findCheckpointHints(uint256[],uint256,uint256)
选择器: 0x62abe3fa  [来源: eth_utils keccak256("findCheckpointHints(uint256[],uint256,uint256)")]
calldata: 0x62abe3fa + abi_encode(requestIds, 1, lastCheckpointIndex)
返回: uint256[] (hints, 与 requestIds 一一对应)
```

### 2c. 链上写操作表

> 所有写操作提交前必须获得用户确认，执行后提取 `.data.txHash`。

---

#### `stake` — 质押 ETH 获取 stETH

**操作:** 调用 Lido 合约的 `submit(address)` 函数，传入可选的 referral 地址（传零地址则无推荐）。发送 ETH value。

**合约:** Lido (stETH) — `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84`（Ethereum 主网，代理合约，运行时确认）

**函数选择器:** `0xa1903eab` — `submit(address)` [来源: eth_utils keccak256]

**calldata 构造:**
```
selector:   a1903eab
referral:   000000000000000000000000 0000000000000000000000000000000000000000  (32字节，零地址)

完整 calldata:
0xa1903eab0000000000000000000000000000000000000000000000000000000000000000
```

**onchainos 命令:**
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
  --input-data 0xa1903eab0000000000000000000000000000000000000000000000000000000000000000 \
  --amt <ETH_AMOUNT_IN_WEI> \
  --from <WALLET_ADDRESS> \
  --force
```

**预检查:**
1. 用 `getCurrentStakeLimit()` 检查质押是否暂停及上限
2. 检查钱包 ETH 余额 ≥ 质押量 + gas 费用
3. 展示当前 APR，请求用户确认后再执行

**注意:** 使用代理合约地址，不要硬编码实现合约地址。官方地址可从 `https://docs.lido.fi/deployed-contracts/` 验证。

---

#### `wrap` — stETH → wstETH

**步骤 1: 检查并设置 approve**

```
合约: Lido (stETH)  0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84
函数: allowance(address,address) 选择器: 0xdd62ed3e  [来源: eth_utils keccak256]
检查: allowance(wallet, wstETH_contract) >= stETH_amount

若不足，先 approve:
calldata: 0x095ea7b3
  + 000000000000000000000000 7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0  (spender)
  + <stETH_amount_32bytes>

onchainos wallet contract-call --chain 1 \
  --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
  --input-data <approve_calldata> --from <WALLET> --force
```

**步骤 2: 调用 wrap**

```
合约: wstETH  0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0
函数: wrap(uint256)  选择器: 0xea598cb0  [来源: eth_utils keccak256]
calldata: 0xea598cb0 + <stETH_amount_32bytes>

onchainos wallet contract-call \
  --chain 1 \
  --to 0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0 \
  --input-data <wrap_calldata> \
  --from <WALLET_ADDRESS> \
  --force
```

**预检查:** 验证 stETH 余额 ≥ 目标 wrap 金额

---

#### `unwrap` — wstETH → stETH

**合约地址:** 运行时按链选择（见合约地址表）

**函数选择器:** `0xde0e9a3e` — `unwrap(uint256)` [来源: eth_utils keccak256]

**calldata:**
```
0xde0e9a3e + <wstETH_amount_32bytes>
```

**onchainos 命令:**
```bash
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <WSTETH_CONTRACT_FOR_CHAIN> \
  --input-data <unwrap_calldata> \
  --from <WALLET_ADDRESS> \
  --force
```

**预检查:** 验证 wstETH 余额 ≥ 目标 unwrap 金额

---

#### `request-withdrawal` — 发起 ETH 提现（stETH → ETH）

**注意:** 仅支持 Ethereum 主网。提现使用 stETH，最小提现金额 100 gwei，最大 1000 stETH（单次请求）。超过 1000 stETH 需拆分为多个请求。

**步骤 1: approve stETH 给 WithdrawalQueue**

```
合约: Lido (stETH)  0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84
approve(WithdrawalQueue, amount)
选择器: 0x095ea7b3  [来源: eth_utils keccak256("approve(address,uint256)")]
spender: 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1
```

**步骤 2: 调用 requestWithdrawals**

```
合约: WithdrawalQueueERC721  0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1
函数: requestWithdrawals(uint256[],address)
选择器: 0xd6681042  [来源: eth_utils keccak256]
calldata: 0xd6681042 + abi_encode(amounts_array, owner_address)

onchainos wallet contract-call \
  --chain 1 \
  --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
  --input-data <requestWithdrawals_calldata> \
  --from <WALLET_ADDRESS> \
  --force
```

返回 NFT 形式的 requestIds，记录供后续 claim 使用。

---

#### `claim-withdrawal` — 领取已完成的提现

**前置条件:** 通过 `getWithdrawalStatus()` 确认 `isFinalized == true`。

**步骤 1:** `getLastCheckpointIndex()` 获取最新 checkpoint

**步骤 2:** `findCheckpointHints(requestIds, 1, lastCheckpointIndex)` 获取 hints

**步骤 3:** 调用 claimWithdrawals

```
合约: WithdrawalQueueERC721  0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1
函数: claimWithdrawals(uint256[],uint256[])
选择器: 0xe3afe0a3  [来源: eth_utils keccak256]
calldata: 0xe3afe0a3 + abi_encode(requestIds_array, hints_array)

onchainos wallet contract-call \
  --chain 1 \
  --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
  --input-data <claimWithdrawals_calldata> \
  --from <WALLET_ADDRESS> \
  --force
```

---

### 合约地址表（运行时引用，不得硬编码于业务逻辑）

| 链 | 合约 | 地址 | 来源 |
|----|------|------|------|
| Ethereum (1) | Lido (stETH) 代理 | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` | docs.lido.fi/deployed-contracts |
| Ethereum (1) | wstETH | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0` | docs.lido.fi/deployed-contracts |
| Ethereum (1) | WithdrawalQueueERC721 | `0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1` | docs.lido.fi/deployed-contracts |
| Arbitrum (42161) | wstETH | `0x5979D7b546E38E414F7E9822514be443A4800529` | docs.lido.fi/deployed-contracts |
| Base (8453) | wstETH | `0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452` | docs.lido.fi/deployed-contracts |
| Optimism (10) | wstETH | `0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb` | docs.lido.fi/deployed-contracts |

> **重要:** 合约地址应在启动时从配置中加载。Ethereum 主网地址可通过访问 `https://docs.lido.fi/deployed-contracts/` 二次验证。

---

### 函数选择器汇总（均经 eth_utils keccak256 验证）

| 选择器 | 函数签名 | 合约 |
|--------|---------|------|
| `0xa1903eab` | `submit(address)` | Lido (stETH) |
| `0xea598cb0` | `wrap(uint256)` | wstETH |
| `0xde0e9a3e` | `unwrap(uint256)` | wstETH |
| `0x095ea7b3` | `approve(address,uint256)` | ERC-20 (stETH/wstETH) |
| `0x70a08231` | `balanceOf(address)` | ERC-20 (stETH/wstETH) |
| `0xdd62ed3e` | `allowance(address,address)` | ERC-20 (stETH/wstETH) |
| `0x7a28fb88` | `getPooledEthByShares(uint256)` | Lido (stETH) |
| `0x19208451` | `getSharesByPooledEth(uint256)` | Lido (stETH) |
| `0x37cfdaca` | `getTotalPooledEther()` | Lido (stETH) |
| `0xd5002f2e` | `getTotalShares()` | Lido (stETH) |
| `0xf5eb42dc` | `sharesOf(address)` | Lido (stETH) |
| `0x035faf82` | `stEthPerToken()` | wstETH |
| `0x9576a0c8` | `tokensPerStEth()` | wstETH |
| `0xb0e38900` | `getWstETHByStETH(uint256)` | wstETH |
| `0xbb2952fc` | `getStETHByWstETH(uint256)` | wstETH |
| `0x609c4c6c` | `getCurrentStakeLimit()` | Lido (stETH) |
| `0x665b4b0b` | `getStakeLimitFullInfo()` | Lido (stETH) |
| `0xd6681042` | `requestWithdrawals(uint256[],address)` | WithdrawalQueueERC721 |
| `0x19aa6257` | `requestWithdrawalsWstETH(uint256[],address)` | WithdrawalQueueERC721 |
| `0xe3afe0a3` | `claimWithdrawals(uint256[],uint256[])` | WithdrawalQueueERC721 |
| `0x5e7eead9` | `claimWithdrawalsTo(uint256[],uint256[],address)` | WithdrawalQueueERC721 |
| `0xb8c4b85a` | `getWithdrawalStatus(uint256[])` | WithdrawalQueueERC721 |
| `0x62abe3fa` | `findCheckpointHints(uint256[],uint256,uint256)` | WithdrawalQueueERC721 |
| `0x526eae3e` | `getLastCheckpointIndex()` | WithdrawalQueueERC721 |

---

## §3 用户场景

### 场景 1：质押 ETH 获取 stETH（核心 happy path）

**用户:** "帮我把 1 ETH 质押到 Lido"

**Agent 动作序列:**

1. **[链下查询]** 调用 `onchainos wallet balance --chain 1` 检查钱包 ETH 余额 ≥ 1 ETH + gas
2. **[链下查询]** eth_call `getCurrentStakeLimit()` 到 stETH 合约 → 确认质押未暂停，当前上限 ≥ 1 ETH
3. **[链下查询]** `GET https://eth-api.lido.fi/v1/protocol/steth/apr/sma` → 获取当前 APR（约 3-4%）
4. **[链下查询]** eth_call `getSharesByPooledEth(1e18)` → 计算预计收到的 stETH shares（stETH 与 ETH 近乎 1:1，但略有差异）
5. **询问用户确认:** 展示信息：
   - 质押: 1 ETH → ~1 stETH
   - 当前 APR: {apr}%
   - 手续费: Lido 收取 10% 奖励作为协议费用
   - Gas 预估: ~{gas} ETH
   - 是否确认？
6. **[链上操作]** 用户确认后执行:
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
     --input-data 0xa1903eab0000000000000000000000000000000000000000000000000000000000000000 \
     --amt 1000000000000000000 \
     --from <WALLET_ADDRESS> \
     --force
   ```
7. **[结果]** 从 `.data.txHash` 提取交易哈希，等待确认，展示最终 stETH 余额

---

### 场景 2：查询持仓与当前 APR（查询类）

**用户:** "查询我在 Lido 的持仓"

**Agent 动作序列:**

1. **[链下查询]** `onchainos wallet addresses` 获取钱包 EVM 地址
2. **[链下查询]** eth_call `balanceOf(wallet)` 到 stETH 合约 (0xae7ab...) → 获取 stETH 余额
3. **[链下查询]** eth_call `balanceOf(wallet)` 到各链 wstETH 合约 → 获取 Ethereum / Arbitrum / Base 上的 wstETH 余额
4. **[链下查询]** eth_call `stEthPerToken()` 到 wstETH 合约 → 获取汇率（1 wstETH = X stETH）
5. **[链下查询]** eth_call `getTotalPooledEther()` → 协议总 TVL
6. **[链下查询]** `GET https://eth-api.lido.fi/v1/protocol/steth/apr/sma` → 当前 APR
7. **[结果]** 汇总展示:
   - Ethereum stETH: {balance} (~{eth_value} ETH)
   - Ethereum wstETH: {balance} (~{steth_value} stETH)
   - Arbitrum wstETH: {balance}
   - Base wstETH: {balance}
   - 当前 APR: {apr}%
   - 1 wstETH = {rate} stETH

---

### 场景 3：发起 ETH 提现（风控场景）

**用户:** "我想把 0.5 stETH 换回 ETH"

**Agent 动作序列:**

1. **[链下查询]** eth_call `balanceOf(wallet)` 到 stETH 合约 → 验证余额 ≥ 0.5 stETH
2. **[链下查询]** eth_call `allowance(wallet, WithdrawalQueue)` → 检查现有授权
3. **[链下查询]** `GET https://wq-api.lido.fi/v2/request-time/calculate?amount=0.5` → 获取预计等待时间
4. **询问用户确认:** 展示:
   - 提现: 0.5 stETH → ~0.5 ETH（当前汇率 ≈ 1:1）
   - 预计等待: {time}（通常 1-5 天，极端情况最多几周）
   - 提现为 NFT 形式，需要两笔交易（request + claim）
   - 是否继续？
5. **[链上操作]** 若 allowance 不足，先 approve（用户再次确认）:
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84 \
     --input-data 0x095ea7b3 \
       000000000000000000000000889edc2edab5f40e902b864ad4d7ade8e412f9b1 \
       000000000000000000000000000000000000000000000006f05b59d3b2000000 \
     --from <WALLET> --force
   ```
6. **[链上操作]** 发起提现请求（用户确认）:
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
     --input-data <requestWithdrawals_calldata> \
     --from <WALLET> --force
   ```
7. **[结果]** 展示 requestId（NFT ID），提示用户：请求已提交，可通过 `get-withdrawal-status --request-id <id>` 跟踪状态，完成后再执行 `claim-withdrawal`

---

### 场景 4：wrap stETH 为 wstETH（DeFi 整合场景）

**用户:** "把我的 100 stETH 转换成 wstETH"

**Agent 动作序列:**

1. **[链下查询]** eth_call `balanceOf(wallet)` 到 stETH 合约 → 确认 ≥ 100 stETH
2. **[链下查询]** eth_call `getWstETHByStETH(100e18)` 到 wstETH 合约 → 计算预计获得的 wstETH
3. **[链下查询]** eth_call `allowance(wallet, wstETH_contract)` → 检查现有授权
4. **询问用户确认:** 展示: 100 stETH → {wstETH_amount} wstETH（汇率 {rate}）
5. **[链上操作]** 若授权不足，先 approve stETH 给 wstETH 合约（用户确认）
6. **[链上操作]** 执行 wrap（用户确认）:
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0 \
     --input-data 0xea598cb0 + <100_stETH_32bytes> \
     --from <WALLET> --force
   ```
7. **[结果]** 展示交易哈希和最终 wstETH 余额

---

### 场景 5：领取已完成的提现（claim）

**用户:** "我的 Lido 提现请求 #123 完成了吗？帮我领取"

**Agent 动作序列:**

1. **[链下查询]** eth_call `getWithdrawalStatus([123])` → 检查 `isFinalized` 和 `isClaimed`
2. **若未完成:** `GET https://wq-api.lido.fi/v2/request-time?ids=123` 获取预计完成时间，告知用户
3. **若已完成且未领取:**
4. **[链下查询]** eth_call `getLastCheckpointIndex()` → 获取 `lastIndex`
5. **[链下查询]** eth_call `findCheckpointHints([123], 1, lastIndex)` → 获取 `hints`
6. **询问用户确认:** 展示可领取金额，请确认操作
7. **[链上操作]** 执行 claimWithdrawals（用户确认）:
   ```bash
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1 \
     --input-data <claimWithdrawals_calldata> \
     --from <WALLET> --force
   ```
8. **[结果]** 展示交易哈希，ETH 已到账钱包

---

## §4 外部 API 依赖

| API | 用途 | 端点 | 认证 |
|-----|------|------|------|
| Lido APR API | 获取 stETH 当前质押年化收益 | `https://eth-api.lido.fi/v1/protocol/steth/apr/sma` | 无需认证 |
| Lido APR API (最新) | 获取最新单日 APR | `https://eth-api.lido.fi/v1/protocol/steth/apr/last` | 无需认证 |
| Lido Withdrawal Queue API | 查询提现预计等待时间 | `https://wq-api.lido.fi/v2/request-time` | 无需认证 |
| Ethereum RPC | eth_call 读取合约状态 | `https://ethereum.publicnode.com` | 无需认证 |
| Arbitrum RPC | 读取 Arbitrum wstETH 余额 | `https://arb1.arbitrum.io/rpc` | 无需认证 |
| Base RPC | 读取 Base wstETH 余额 | `https://base-rpc.publicnode.com` | 无需认证 |
| Optimism RPC | 读取 Optimism wstETH 余额 | `https://mainnet.optimism.io` | 无需认证 |

> **RPC 端点选择原则:** 使用 publicnode.com 作为 Ethereum 和 Base 主端点（避免 cloudflare-eth.com 和 llamarpc.com 的稳定性问题）。

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `rpc_url_ethereum` | string | `https://ethereum.publicnode.com` | Ethereum 主网 RPC |
| `rpc_url_arbitrum` | string | `https://arb1.arbitrum.io/rpc` | Arbitrum RPC |
| `rpc_url_base` | string | `https://base-rpc.publicnode.com` | Base RPC |
| `rpc_url_optimism` | string | `https://mainnet.optimism.io` | Optimism RPC |
| `lido_steth_address` | string | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` | Lido/stETH 代理合约 (Ethereum) |
| `wsteth_address_eth` | string | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0` | wstETH 合约 (Ethereum) |
| `withdrawal_queue_address` | string | `0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1` | WithdrawalQueueERC721 (Ethereum) |
| `wsteth_address_arbitrum` | string | `0x5979D7b546E38E414F7E9822514be443A4800529` | wstETH (Arbitrum) |
| `wsteth_address_base` | string | `0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452` | wstETH (Base) |
| `wsteth_address_optimism` | string | `0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb` | wstETH (Optimism) |
| `dry_run` | bool | `false` | true 时跳过链上写操作，仅输出 calldata；绝不将 `--dry-run` 传给 onchainos CLI |
| `max_withdrawal_amount_per_request` | string | `1000000000000000000000` | 单次提现最大 stETH (1000 stETH in wei) |
| `min_withdrawal_amount` | string | `100` | 单次提现最小 stETH (100 gwei in wei) |

---

## §6 实现注意事项

### dry_run 处理
`--dry-run` 不是 `onchainos wallet contract-call` 的合法标志。若配置 `dry_run=true`，应在 Rust 层提前 return，仅打印 calldata，**不调用** onchainos CLI。

### ABI 编码辅助
使用 `alloy-sol-types` crate 的 `sol!` 宏生成选择器和 ABI 编码，不要手动 keccak256（见 KNOWLEDGE_HUB.md 中 Python SHA3 != Keccak-256 的教训）。

### zero-address 处理
dry_run 时若需要 wallet address 作为 ABI 参数，使用 `0x0000000000000000000000000000000000000000` 作为占位符。

### stETH 余额特性
stETH 是 rebasing token，余额每天随奖励自动增加。wstETH 是非 rebasing 的，持有份额固定，汇率每天变化。在展示时需说明区别。

### 提现流程说明
Lido 提现是 V2 withdrawals（2023 年 5 月上线）。提现分两步：
1. `requestWithdrawals()` — 创建提现请求（NFT），stETH 锁定
2. `claimWithdrawals()` — 请求完成后领取 ETH（需要 hints 参数）

两步之间通常等待 1-5 天，极端情况更长。

### L2 上的 wstETH
L2 上的 wstETH（Arbitrum/Base/Optimism）是通过 canonical bridge 锁定 Ethereum 主网 wstETH 铸造的。L2 上不能直接质押 ETH 或发起提现，只能 wrap/unwrap wstETH ↔ stETH（如果该链支持），以及查询余额。

### E106 合规性
所有调用 `wallet contract-call` 的 SKILL.md section 必须在同一 section 内包含 "ask user to confirm" 文本（不能仅在文件头部全局说明）。
