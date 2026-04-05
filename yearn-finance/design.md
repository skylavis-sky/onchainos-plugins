# Plugin PRD — Yearn Finance

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `yearn-finance` |
| dapp_name | Yearn Finance |
| target_chains | Ethereum (1) |
| target_protocols | yVaults v3 (ERC-4626), yVaults v2 (legacy) |
| category | defi-protocol |
| tags | yield, vault, erc4626, ethereum, stablecoin |
| version | 0.1.0 |
| author | GeoGu360 |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无官方 Rust SDK |
| SDK 支持哪些技术栈？ | JavaScript/TypeScript (yearn-sdk), Python (web3.py patterns) |
| 有 REST API？ | ✅ yDaemon REST API: `https://ydaemon.yearn.fi` — 无需 API Key，返回 vault 列表/详情/APR/TVL |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | 无直接 Skill；有 yearn-sdk JS 库 |
| 支持哪些链？ | Ethereum (1), Arbitrum (42161), Optimism (10), Base (8453), Polygon (137) |
| 是否需要 onchainos 广播？ | ✅ Yes — deposit/withdraw 是链上写操作，需 ERC-4626 deposit/redeem + ERC-20 approve |

**接入路径：API** — Rust 直接调用 yDaemon REST API 获取链下数据，链上操作手动构造 ERC-4626 calldata 通过 onchainos 广播。

---

## §2 接口映射

### 需要接入的操作

| # | 操作 | 类型 | 优先级 |
|---|------|------|--------|
| 1 | `vaults` — 列出 Ethereum 上所有活跃 yVaults | 链下查询 | P0 |
| 2 | `positions` — 查询用户在各 vault 的持仓余额 | 链下查询 | P0 |
| 3 | `rates` — 查询各 vault 的 APR/APY 信息 | 链下查询 | P0 |
| 4 | `deposit` — 向 yVault 存入 ERC-20 资产 | 链上写操作 | P0 |
| 5 | `withdraw` — 从 yVault 赎回 ERC-20 资产 | 链上写操作 | P0 |

---

### 链下查询表

#### 1. vaults — 列出活跃 vault

- **Endpoint**: `GET https://ydaemon.yearn.fi/{chainID}/vaults/all?limit=200`
- **Parameters**: `chainID` (path, default: 1), `limit` (query, default: 200)
- **返回值关键字段**:
  ```json
  [
    {
      "address": "0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa",
      "name": "USDT-1 yVault",
      "symbol": "yvUSDT-1",
      "version": "3.0.2",
      "kind": "Multi Strategy",
      "token": {
        "address": "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        "symbol": "USDT",
        "decimals": 6
      },
      "apr": { "netAPR": 0.0329 },
      "tvl": { "tvl": 7604530.73, "totalAssets": "7605912702794" },
      "info": { "isRetired": false, "isHidden": false }
    }
  ]
  ```
- **过滤条件**: 排除 `isRetired=true` 和 `isHidden=true`

#### 2. positions — 用户持仓余额

- **Endpoint**: `GET https://ydaemon.yearn.fi/{chainID}/vaults/all?limit=200`
- **实现**: 遍历 vaults，对每个 vault 调用 `eth_call` 查询:
  - `balanceOf(address)` selector `0x70a08231` → 用户的 vault token 余额 (shares)
  - `pricePerShare()` selector `0x99530b06` → 每 share 对应的底层资产数量
  - 换算: `balance_assets = shares * pricePerShare / 10^decimals`
- **RPC**: `https://ethereum.publicnode.com` (Ethereum mainnet)

#### 3. rates — APR 信息

- **Endpoint**: `GET https://ydaemon.yearn.fi/{chainID}/vaults/all?limit=200`
- **返回值关键字段**:
  ```json
  {
    "address": "...",
    "name": "...",
    "apr": {
      "netAPR": 0.0329,
      "fees": { "performance": 0.1, "management": 0.0 },
      "points": {
        "weekAgo": 0.034,
        "monthAgo": 0.031,
        "inception": 0.028
      }
    }
  }
  ```

---

### 链上写操作表

#### 4. deposit — 存入资产到 yVault

**流程**: ERC-20 approve → ERC-4626 deposit (3s 延迟)

**Step 4a: ERC-20 approve**
- **合约**: 底层 token 合约 (从 vault.token.address 获取)
- **函数**: `approve(address spender, uint256 amount)` 
- **Selector**: `0x095ea7b3` ✅ (verified: `cast sig "approve(address,uint256)"`)
- **ABI 编码**:
  ```
  0x095ea7b3
  + pad32(vault_address)  // spender = vault contract
  + pad32(amount)         // amount in token units (e.g. 10000 for 0.01 USDT with 6 decimals)
  ```
- **onchainos 命令**:
  ```bash
  onchainos wallet contract-call --chain 1 --to <token_address> \
    --input-data 0x095ea7b3<spender_padded><amount_padded>
  ```

**Step 4b: ERC-4626 deposit (after 3s delay)**
- **合约**: yVault 合约地址
- **函数**: `deposit(uint256 assets, address receiver)`
- **Selector**: `0x6e553f65` ✅ (verified: `cast sig "deposit(uint256,address)"`)
- **ABI 编码**:
  ```
  0x6e553f65
  + pad32(assets)   // amount in token units
  + pad32(receiver) // user wallet address
  ```
- **onchainos 命令**:
  ```bash
  onchainos wallet contract-call --chain 1 --to <vault_address> \
    --input-data 0x6e553f65<assets_padded><receiver_padded>
  ```
- **注意**: USDT (decimals=6) → 0.01 USDT = 10000 raw units

#### 5. withdraw / redeem — 赎回资产

**函数**: `redeem(uint256 shares, address receiver, address owner)`
- **Selector**: `0xba087652` ✅ (verified: `cast sig "redeem(uint256,address,address)"`)
- **合约**: yVault 合约地址
- **ABI 编码**:
  ```
  0xba087652
  + pad32(shares)   // vault token shares to redeem
  + pad32(receiver) // destination address (user wallet)
  + pad32(owner)    // owner of shares (user wallet)
  ```
- **onchainos 命令**:
  ```bash
  onchainos wallet contract-call --chain 1 --to <vault_address> \
    --input-data 0xba087652<shares_padded><receiver_padded><owner_padded>
  ```
- **注意**: 用 `balanceOf(user)` 读取 shares 余额，部分赎回按比例

---

### Function Selector 核对清单

| 函数签名 | cast sig 结果 | 用途 |
|---------|-------------|------|
| `deposit(uint256,address)` | `0x6e553f65` | ERC-4626 deposit |
| `redeem(uint256,address,address)` | `0xba087652` | ERC-4626 redeem |
| `approve(address,uint256)` | `0x095ea7b3` | ERC-20 approve |
| `balanceOf(address)` | `0x70a08231` | 查用户 shares 余额 |
| `pricePerShare()` | `0x99530b06` | 查每 share 价值 |
| `totalAssets()` | `0x01e1d114` | 查 vault 总资产 |
| `asset()` | `0x38d52e0f` | 查 vault 底层 token |

---

## §3 用户场景

### 场景 1: 查看所有 Yearn vault 和收益率

**用户对 Agent 说**: "Show me all Yearn vaults on Ethereum with their APR"

**Agent 动作序列**:
1. 链下查询: `GET https://ydaemon.yearn.fi/1/vaults/all?limit=200` 获取所有 vault
2. 过滤活跃 vault (isRetired=false, isHidden=false)
3. 提取每个 vault 的 name, symbol, token.symbol, apr.netAPR, tvl.tvl
4. 按 APR 降序排列
5. 输出 JSON 表格: vault 名称、底层代币、净 APR、TVL

**预期输出**:
```json
{
  "ok": true,
  "data": {
    "chain": 1,
    "vaults": [
      {
        "address": "0x310B7...",
        "name": "USDT-1 yVault",
        "token": "USDT",
        "net_apr": "3.29%",
        "tvl_usd": 7604530.73
      }
    ]
  }
}
```

---

### 场景 2: 查询用户在 Yearn 的持仓

**用户对 Agent 说**: "What are my Yearn positions on Ethereum?"

**Agent 动作序列**:
1. 解析钱包: `onchainos wallet addresses` → 提取 chainIndex "1" 对应地址
2. 链下查询: `GET https://ydaemon.yearn.fi/1/vaults/all?limit=200` 获取所有 vault
3. 对每个 vault，链下 eth_call:
   - `balanceOf(user_address)` (selector `0x70a08231`) → shares 余额
   - `pricePerShare()` (selector `0x99530b06`) → 每 share 价值
4. 换算: `underlying_balance = shares * pricePerShare / 10^decimals`
5. 过滤 shares > 0，输出持仓
6. 附加 APR 信息（从 yDaemon 获取）

---

### 场景 3: 存入 USDT 到 yvUSDT-1 vault

**用户对 Agent 说**: "Deposit 0.01 USDT into Yearn USDT vault on Ethereum"

**Agent 动作序列**:
1. 解析钱包: `onchainos wallet addresses` → chainIndex "1" → user_wallet
2. 链下查询: `GET https://ydaemon.yearn.fi/1/vaults/all` → 找到 yvUSDT-1 vault `0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa`
3. Dry-run 检查 (if --dry-run): 返回模拟响应，展示 calldata
4. 告知用户操作详情，**请求用户确认** (approve + deposit 两步)
5. 链上写操作 Step A: ERC-20 approve USDT:
   ```
   onchainos wallet contract-call --chain 1 \
     --to 0xdAC17F958D2ee523a2206206994597C13D831ec7 \
     --input-data 0x095ea7b3[vault_address_padded][10000_padded]
   ```
6. 等待 3 秒 (approve confirm delay)
7. 链上写操作 Step B: ERC-4626 deposit:
   ```
   onchainos wallet contract-call --chain 1 \
     --to 0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa \
     --input-data 0x6e553f65[10000_padded][user_wallet_padded]
   ```
8. 提取 txHash，输出结果

---

### 场景 4: 从 vault 赎回资产

**用户对 Agent 说**: "Withdraw my USDT from Yearn yvUSDT-1 vault"

**Agent 动作序列**:
1. 解析钱包: `onchainos wallet addresses` → chainIndex "1" → user_wallet
2. 查询持仓: `eth_call balanceOf(user_wallet)` on vault `0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa`
3. 如果 shares == 0: 返回错误"No position in this vault"
4. Dry-run 检查 (if --dry-run): 返回模拟响应
5. **请求用户确认** 赎回数量
6. 链上写操作: ERC-4626 redeem:
   ```
   onchainos wallet contract-call --chain 1 \
     --to 0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa \
     --input-data 0xba087652[shares_padded][user_wallet_padded][user_wallet_padded]
   ```
7. 提取 txHash，输出结果

---

### 场景 5: 查看 USDT vault 的 APR 历史

**用户对 Agent 说**: "Show me the APR for the Yearn USDT vault"

**Agent 动作序列**:
1. 链下查询: `GET https://ydaemon.yearn.fi/1/vaults/0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa`
2. 提取 apr: netAPR, points.weekAgo, points.monthAgo, points.inception
3. 提取 strategies 列表和各自 APR
4. 输出格式化的 APR 报告

---

## §4 外部 API 依赖

| API | 用途 | 认证 |
|-----|------|------|
| `https://ydaemon.yearn.fi` | yVault 元数据、APR、TVL、strategies | 无需认证 |
| `https://ethereum.publicnode.com` | Ethereum mainnet JSON-RPC (eth_call) | 无需认证 |

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `chain` | u64 | 1 | Ethereum chain ID |
| `dry_run` | bool | false | 模拟运行，不发链上交易 |
| `vault` | Option\<String\> | None | 指定 vault 地址或 symbol (e.g. "yvUSDT-1") |
| `amount` | Option\<String\> | None | 金额 (e.g. "0.01") |
| `token` | Option\<String\> | None | 底层代币 symbol (e.g. "USDT") |
| `all` | bool | false | withdraw: 赎回全部 shares |

---

## §6 关键合约地址 (Ethereum Mainnet)

| 合约 | 地址 | 说明 |
|------|------|------|
| yvUSDT-1 vault | `0x310B7Ea7475A0B449Cfd73bE81522F1B88eFAFaa` | USDT yVault v3.0.2 — 主测试 vault |
| USDT | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | Tether USD (6 decimals) |
| WETH | `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` | Wrapped Ether |
| USDC | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | USD Coin (6 decimals) |
| DAI | `0x6B175474E89094C44Da98b954EedeAC495271d0F` | DAI Stablecoin (18 decimals) |
| V3 Registry | `0xd40ecF29e001c76Dcc4cC0D9cd50520CE845B038` | yVaults v3 registry |
| yDaemon API | `https://ydaemon.yearn.fi` | REST API for vault metadata |

**注意**: 所有 vault 地址在运行时通过 yDaemon API 动态解析，不硬编码。仅 yvUSDT-1 地址用于测试目的。
