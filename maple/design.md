# Maple Finance Plugin — Design Document (PRD)

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `maple` |
| dapp_name | Maple Finance |
| version | 0.1.0 |
| target_chains | Ethereum (1) |
| target_protocols | Lending / Yield (ERC-4626 pool vaults) |
| bitable_record | recvfIURyo38kQ |
| dev_dir | /Users/amos/projects/plugin-store-dev/maple |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无 |
| SDK 支持哪些技术栈？ | Solidity contracts only (maple-labs/maple-core-v2) |
| 有 REST API？ | 有 GraphQL API: `https://api.maple.finance/v2/graphql` |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | 无 |
| 支持哪些链？ | Ethereum mainnet (1) — syrupUSDC and syrupUSDT pools |
| 是否需要onchainos广播？ | Yes — deposit, requestRedeem are on-chain writes |

**接入路径**: API (GraphQL for reads) + direct eth_call + onchainos for writes

---

## §2 接口映射

### 2a. 需要接入的操作表

| 操作 | 类型 | 命令 |
|------|------|------|
| 查询所有池子 | 链下 | `pools` |
| 查询用户持仓 | 链下 | `positions` |
| 查询 APY 利率 | 链下 | `rates` |
| 存款 (deposit via SyrupRouter) | 链上写 | `deposit` |
| 申请赎回 (requestRedeem) | 链上写 | `withdraw` |

### 2b. 链下查询表

#### `pools` — 查询池子列表

**方法**: GraphQL + eth_call (totalAssets, totalSupply)

```graphql
query {
  pools(where: { chain: "ethereum" }) {
    id
    name
    asset { symbol decimals address }
    totalAssets
    totalSupply
    apr7d
    apr30d
    poolAddress
  }
}
```

**API**: `https://api.maple.finance/v2/graphql`

**返回值**:
```json
{
  "pools": [
    {
      "name": "syrupUSDC",
      "poolAddress": "0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b",
      "asset": { "symbol": "USDC", "decimals": 6 },
      "apr7d": "0.085",
      "totalAssets": "100000000"
    }
  ]
}
```

**Fallback (no GraphQL)**: eth_call totalAssets() on each known pool address.

---

#### `positions` — 查询用户持仓

**方法**: eth_call `balanceOf(address)` + `convertToExitAssets(uint256)` on each pool

**Pool contracts**:
- syrupUSDC Pool: `0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b`
- syrupUSDT Pool: `0x356B8d89c1e1239Cbbb9dE4815c39A1474d5BA7D`

**Selectors** (verified with `cast sig`):
- `balanceOf(address)` → `0x70a08231`
- `convertToExitAssets(uint256)` → `0x50496cbd`
- `balanceOfAssets(address)` → `0x9159b206`

**Flow**:
1. `balanceOf(wallet)` → shares held by user
2. `convertToExitAssets(shares)` → underlying asset value (accounting for unrealized losses)
3. Return `{ pool, shares, assets_value, underlying_symbol }`

---

#### `rates` — 查询 APY 利率

**方法**: GraphQL query for apr7d / apr30d per pool

**GraphQL query**:
```graphql
query {
  pools(where: { chain: "ethereum" }) {
    name
    poolAddress
    apr7d
    apr30d
    totalAssets
    asset { symbol decimals }
  }
}
```

**Fallback**: eth_call `totalAssets()` (selector `0x01e1d114`) to show TVL.

---

### 2c. 链上写操作表

#### `deposit` — 存款到 Maple 池

**Contract**: SyrupRouter (per pool)
- syrupUSDC SyrupRouter: `0x134cCaaA4F1e4552eC8aEcb9E4A2360dDcF8df76`
- syrupUSDT SyrupRouter: `0xF007476Bb27430795138C511F18F821e8D1e5Ee2`

**Function** (on SyrupRouter): `deposit(uint256 amount, bytes32 depositData)`
- Selector: `0xc9630cb0` (verified: `cast sig "deposit(uint256,bytes32)"`)
- `depositData`: use `bytes32(0)` (32 zero bytes)

**Underlying tokens**:
- USDC: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48`
- USDT: `0xdAC17F958D2ee523a2206206994597C13D831ec7`

**Flow**:
1. ERC-20 `approve(syrupRouter, amount)` via `wallet contract-call`
   - selector: `0x095ea7b3`
   - calldata: `0x095ea7b3` + spender(32 bytes) + amount(32 bytes)
2. Wait 3 seconds
3. SyrupRouter.`deposit(amount, bytes32(0))` via `wallet contract-call`
   - calldata: `0xc9630cb0` + amount(32 bytes) + 0x000...000(32 bytes)

**onchainos commands**:
```bash
# Step 1: ERC-20 approve
onchainos wallet contract-call --chain 1 \
  --to <USDC_OR_USDT> \
  --input-data 0x095ea7b3<spender_padded><amount_hex>

# Step 2: deposit
onchainos wallet contract-call --chain 1 \
  --to <SYRUP_ROUTER> \
  --input-data 0xc9630cb0<amount_hex><bytes32_zero>
```

**Note**: USDT on Ethereum requires a two-step approve: set to 0 first, then set desired amount (USDT "allowance race condition"). Check current allowance before approve.

---

#### `withdraw` — 申请赎回 (requestRedeem)

**Contract**: Pool contract directly
- syrupUSDC Pool: `0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b`
- syrupUSDT Pool: `0x356B8d89c1e1239Cbbb9dE4815c39A1474d5BA7D`

**Function**: `requestRedeem(uint256 shares, address owner)`
- Selector: `0x107703ab` (verified: `cast sig "requestRedeem(uint256,address)"`)
- `shares`: user's share balance (from `balanceOf`)
- `owner`: user's wallet address

**onchainos command**:
```bash
onchainos wallet contract-call --chain 1 \
  --to <POOL_ADDRESS> \
  --input-data 0x107703ab<shares_hex><owner_padded>
```

**Note**: requestRedeem puts shares in the withdrawal queue. There is a queue delay (varies by pool state). This is NOT an immediate redemption — it initiates the withdrawal process. The actual `redeem()` call must wait until the queue processes.

---

## §3 用户场景

### 场景 1: 查询 Maple 池子和利率

**用户说**: "Show me Maple Finance lending pools and their APY"

**动作序列**:
1. [链下] 调用 GraphQL `https://api.maple.finance/v2/graphql` 查询 pools
2. [链下] eth_call `totalAssets()` on syrupUSDC and syrupUSDT pools (fallback)
3. 返回 JSON: 池子名称、APY、TVL、支持的代币

---

### 场景 2: 查询我的持仓

**用户说**: "Show my Maple Finance positions"

**动作序列**:
1. [链下] 解析钱包地址: `onchainos wallet addresses` → `data.evm[].address` chainIndex "1"
2. [链下] eth_call `balanceOf(wallet)` on syrupUSDC pool → shares
3. [链下] eth_call `convertToExitAssets(shares)` → underlying value
4. [链下] 对 syrupUSDT pool 重复
5. 返回 JSON: 池子、份额数量、对应 USDC/USDT 价值

---

### 场景 3: 存入 USDC 到 Maple syrupUSDC 池

**用户说**: "Deposit 0.01 USDC into Maple syrupUSDC pool"

**动作序列**:
1. [链下] 解析钱包地址
2. [链下] `onchainos wallet balance --chain 1` 检查 USDT 余额
3. [链上-approve] **Ask user to confirm** — 提交 ERC-20 approve calldata via `wallet contract-call`
4. 等待 3 秒
5. [链上-deposit] **Ask user to confirm** — 提交 SyrupRouter.deposit calldata via `wallet contract-call`
6. 返回 txHash

---

### 场景 4: 申请赎回持仓

**用户说**: "Request withdrawal of my syrupUSDC shares"

**动作序列**:
1. [链下] 解析钱包地址
2. [链下] eth_call `balanceOf(wallet)` → shares amount
3. [链上] **Ask user to confirm** — 提交 Pool.requestRedeem(shares, wallet) via `wallet contract-call`
4. 返回 txHash + 提示用户等待队列处理

---

## §4 外部 API 依赖

| API | 用途 | URL |
|-----|------|-----|
| Maple GraphQL API | 查询池子信息、APY、TVL | `https://api.maple.finance/v2/graphql` |
| Ethereum RPC | eth_call (balanceOf, totalAssets, convertToExitAssets) | `https://ethereum.publicnode.com` |

---

## §5 配置参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| chain_id | 1 | Ethereum mainnet |
| rpc_url | `https://ethereum.publicnode.com` | EVM RPC endpoint |
| graphql_url | `https://api.maple.finance/v2/graphql` | Maple GraphQL API |
| dry_run | false | 模拟模式，不发链上交易 |
| syrupusdc_pool | `0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b` | syrupUSDC pool contract |
| syrupusdt_pool | `0x356B8d89c1e1239Cbbb9dE4815c39A1474d5BA7D` | syrupUSDT pool contract |
| syrupusdc_router | `0x134cCaaA4F1e4552eC8aEcb9E4A2360dDcF8df76` | syrupUSDC SyrupRouter |
| syrupusdt_router | `0xF007476Bb27430795138C511F18F821e8D1e5Ee2` | syrupUSDT SyrupRouter |
| usdc_address | `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` | USDC on Ethereum |
| usdt_address | `0xdAC17F958D2ee523a2206206994597C13D831ec7` | USDT on Ethereum |

---

## Appendix: Function Selectors (verified via `cast sig`)

| Function | Selector | Verified |
|----------|----------|---------|
| `deposit(uint256,address)` (ERC-4626 Pool) | `0x6e553f65` | ✅ cast sig |
| `deposit(uint256,bytes32)` (SyrupRouter) | `0xc9630cb0` | ✅ cast sig |
| `requestRedeem(uint256,address)` | `0x107703ab` | ✅ cast sig |
| `redeem(uint256,address,address)` | `0xba087652` | ✅ cast sig |
| `balanceOf(address)` | `0x70a08231` | ✅ cast sig |
| `balanceOfAssets(address)` | `0x9159b206` | ✅ cast sig |
| `totalAssets()` | `0x01e1d114` | ✅ cast sig |
| `convertToAssets(uint256)` | `0x07a2d13a` | ✅ cast sig |
| `convertToExitAssets(uint256)` | `0x50496cbd` | ✅ cast sig |
| `totalSupply()` | `0x18160ddd` | ✅ cast sig |
| `approve(address,uint256)` (ERC-20) | `0x095ea7b3` | ✅ standard |
