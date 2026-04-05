# §0 Plugin Meta

```yaml
plugin_name: solv-solvbtc
dapp_name: Solv SolvBTC
version: 0.1.0
target_chains: [evm]
chains_supported:
  - Arbitrum (42161)  # PRIMARY — test wallet has ETH/ARB; WBTC->SolvBTC pool active
  - Ethereum (1)      # SECONDARY — xSolvBTC wrapping only here; requires ETH for gas
category: Yield/Liquid BTC
rpc:
  42161: https://arb1.arbitrum.io/rpc
  1: https://ethereum.publicnode.com
```

---

## Protocol Overview

Solv Protocol issues **SolvBTC** — a 1:1 BTC-backed ERC-20 token across multiple EVM chains. Users deposit WBTC (or other BTC assets) via the `SolvBTCRouterV2` contract to receive SolvBTC. SolvBTC is the base "liquid BTC" token; yield is earned by optionally wrapping SolvBTC into **xSolvBTC** via the `XSolvBTCPool` contract (Ethereum mainnet only).

**Token hierarchy:**
```
WBTC  →[deposit via RouterV2]→  SolvBTC  →[deposit via XSolvBTCPool]→  xSolvBTC
                                         ←[withdraw via XSolvBTCPool]←
       ←[withdrawRequest via RouterV2]←
       (pending queue, NOT instant)
```

**Key insight:** SolvBTC redemption back to WBTC is NOT instant — it goes through an OpenFundMarket redemption queue (returns an ERC-3525 SFT claim ticket). xSolvBTC↔SolvBTC swaps are instant with a 0.05% withdraw fee.

---

# §1 接入可行性调研表

| 维度 | 评估 |
|------|------|
| 接入路径 | 直接合约调用 (SolvBTCRouterV2 + XSolvBTCPool) |
| Mint/Deposit | ✅ RouterV2.deposit() — WBTC approve + deposit call |
| Redeem/Withdraw | ⚠️ RouterV2.withdrawRequest() — 非即时，返回 SFT claim ticket；cancelWithdrawRequest() 可撤销 |
| xSolvBTC wrap | ✅ XSolvBTCPool.deposit() — 即时，仅 Ethereum mainnet |
| xSolvBTC unwrap | ✅ XSolvBTCPool.withdraw() — 即时，0.05% fee，仅 Ethereum mainnet |
| NAV/Price | ✅ DeFiLlama Coins API (实时价格) |
| APY | ⚠️ 无官方 API；使用 DeFiLlama Yields API (project: solv-basis-trading) |
| TVL | ✅ DeFiLlama TVL API (`/tvl/solv-protocol`) |
| 合约升级风险 | 中 — BeaconProxy 模式，可升级；核心逻辑稳定 |
| 赎回延迟 | ⚠️ WBTC 赎回非即时，插件需明确告知用户 |

**可行性结论:** L1-L3 全部可行。L4 优先在 Arbitrum (42161) 测试 WBTC→SolvBTC mint；xSolvBTC wrap/unwrap 在 Ethereum (1) 测试（需 ETH gas + SolvBTC）。

---

# §2 接口映射表

## 操作列表

| 命令 | 描述 | 链 |
|------|------|----|
| `mint` | WBTC → SolvBTC | Arbitrum (42161) 或 Ethereum (1) |
| `redeem` | SolvBTC → WBTC 赎回请求 (非即时) | Arbitrum (42161) 或 Ethereum (1) |
| `cancel-redeem` | 撤销赎回请求 | Arbitrum (42161) 或 Ethereum (1) |
| `wrap` | SolvBTC → xSolvBTC (yield token) | Ethereum (1) only |
| `unwrap` | xSolvBTC → SolvBTC | Ethereum (1) only |
| `get-price` | SolvBTC / xSolvBTC 当前价格 (USD) | 链下查询 |
| `get-tvl` | Solv Protocol TVL | 链下查询 |
| `get-balance` | 查询 SolvBTC / xSolvBTC 余额 | Arbitrum 或 Ethereum |

---

## 链下查询

### 1. 价格查询 — DeFiLlama Coins API

```
GET https://coins.llama.fi/prices/current/{coin_key}

coin_key 格式:
  - SolvBTC on Arbitrum: arbitrum:0x3647c54c4c2c65bc7a2d63c0da2809b399dbbdc0
  - SolvBTC on Ethereum: ethereum:0x7a56e1c57c7475ccf742a1832b028f0456652f97
  - xSolvBTC on Ethereum: ethereum:0xd9d920aa40f578ab794426f5c90f6c731d159def

Response: { "coins": { "<key>": { "price": 66821.12, "symbol": "SolvBTC", "decimals": 18 } } }
```

示例:
```bash
curl -s "https://coins.llama.fi/prices/current/arbitrum:0x3647c54c4c2c65bc7a2d63c0da2809b399dbbdc0,ethereum:0xd9d920aa40f578ab794426f5c90f6c731d159def"
```

xSolvBTC NAV (BTC per xSolvBTC) = `xSolvBTC.price / SolvBTC.price`
- 当前约: $69,105 / $66,821 ≈ 1.034 BTC per xSolvBTC

### 2. TVL — DeFiLlama Protocol API

```
GET https://api.llama.fi/tvl/solv-protocol
Response: <number>  (USD TVL, e.g. 575460702.51)
```

### 3. APY — DeFiLlama Yields API

```
GET https://yields.llama.fi/pools
Response: { "data": [ { "pool": "...", "project": "solv-basis-trading", "chain": "Arbitrum", "symbol": "WBTC", "apy": <number>, "tvlUsd": <number> }, ... ] }

过滤条件: project == "solv-basis-trading"
```

注意: DeFiLlama 上 `solv-basis-trading` APY 显示为 0（链外收益策略无实时 APY 数据）。实际 yield 来源于 Babylon staking、GMX LP 等策略，非链上可读。推荐展示 "variable yield, strategy-dependent"。

### 4. ERC-20 余额查询 (on-chain read)

```
Function: balanceOf(address)
Selector: 0x70a08231
Calldata: 0x70a08231 + abi_encode(wallet_address)
```

---

## 链上写操作 — EVM

### Step 0: 获取钱包地址

```bash
onchainos wallet balance --chain <chain_id> --output json
# path: data.details[0].tokenAssets[0].address
```

### Operation A: Mint SolvBTC (WBTC → SolvBTC)

**Step A1: ERC-20 Approve WBTC → RouterV2**

| 字段 | Arbitrum (42161) | Ethereum (1) |
|------|-----------------|--------------|
| `--chain` | 42161 | 1 |
| `--to` (WBTC token) | `0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f` | `0x2260fac5e5542a773aa44fbcfedf7c193bc2c599` |
| `--to` (RouterV2) | `0x92E8A4407FD1ae7a53a32f1f832184edF071080A` | `0x3d93B9e8F0886358570646dAd9421564C5fE6334` |

```
Function: approve(address spender, uint256 amount)
Selector: 0x095ea7b3
Calldata: 0x095ea7b3
          + abi_encode(router_address)    # 32 bytes
          + abi_encode(amount_in_wbtc)    # 32 bytes (WBTC has 8 decimals)
```

```bash
onchainos wallet contract-call \
  --chain 42161 \
  --to 0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f \
  --input-data 0x095ea7b3\
<router_addr_padded_32>\
<amount_padded_32> \
  --force
```

**Step A2: deposit() via RouterV2**

```
Function: deposit(address targetToken_, address currency_, uint256 currencyAmount_, uint256 minimumTargetTokenAmount_, uint64 expireTime_)
Selector: 0x672262e5
```

| 参数 | 说明 |
|------|------|
| `targetToken_` | SolvBTC 合约地址 |
| `currency_` | WBTC 合约地址 |
| `currencyAmount_` | WBTC 数量 (8 decimals) |
| `minimumTargetTokenAmount_` | 最小收到的 SolvBTC (18 decimals)，设为 0 跳过滑点保护 |
| `expireTime_` | Unix timestamp (uint64)，建议 block.timestamp + 300 |

Calldata 编码:
```
0x672262e5
+ pad32(targetToken)       # SolvBTC address
+ pad32(currency)          # WBTC address
+ pad32(currencyAmount)    # WBTC amount (8 decimals)
+ pad32(minTargetAmount)   # min SolvBTC out (18 decimals), can be 0
+ pad32(expireTime)        # uint64, e.g. current_timestamp + 300
```

Router 地址和 Pool ID (内部使用):

| Chain | RouterV2 Proxy | Pool ID (WBTC→SolvBTC) |
|-------|---------------|------------------------|
| Arbitrum (42161) | `0x92E8A4407FD1ae7a53a32f1f832184edF071080A` | `0x488def4a346b409d5d57985a160cd216d29d4f555e1b716df4e04e2374d2d9f6` |
| Ethereum (1) | `0x3d93B9e8F0886358570646dAd9421564C5fE6334` | `0x716db7dc196abe78d5349c7166896f674ab978af26ada3e5b3ea74c5a1b48307` |

注意: Pool ID 由 `RouterV2.poolIds(targetToken, currency)` 读取，无需用户提供，由 Router 合约内部路由。

```bash
onchainos wallet contract-call \
  --chain 42161 \
  --to 0x92E8A4407FD1ae7a53a32f1f832184edF071080A \
  --input-data <calldata> \
  --force
```

**成功条件:** `data.txHash` 存在且非空；用户 SolvBTC 余额增加。

---

### Operation B: Redeem SolvBTC (SolvBTC → WBTC 请求)

⚠️ **非即时赎回** — 返回 ERC-3525 SFT (redemption claim ticket)，需等待 OpenFundMarket 队列处理后才能领取 WBTC。

**Step B1: ERC-20 Approve SolvBTC → RouterV2**

```
Function: approve(address,uint256)
Selector: 0x095ea7b3
--to: SolvBTC token address
spender: RouterV2 address
```

**Step B2: withdrawRequest() via RouterV2**

```
Function: withdrawRequest(address targetToken_, address currency_, uint256 withdrawAmount_)
Selector: 0xd2cfd97d
```

| 参数 | 说明 |
|------|------|
| `targetToken_` | SolvBTC 合约地址 |
| `currency_` | WBTC 合约地址 |
| `withdrawAmount_` | 赎回的 SolvBTC 数量 (18 decimals) |

返回值: `(address redemption, uint256 redemptionId)` — ERC-3525 SFT claim ticket。

```bash
onchainos wallet contract-call \
  --chain 42161 \
  --to 0x92E8A4407FD1ae7a53a32f1f832184edF071080A \
  --input-data <calldata> \
  --force
```

**Step B3 (可选): cancelWithdrawRequest() — 撤销赎回**

```
Function: cancelWithdrawRequest(address targetToken_, address redemption_, uint256 redemptionId_)
Selector: 0x42c7774b
```

---

### Operation C: Wrap SolvBTC → xSolvBTC (Ethereum only)

xSolvBTC = yield-bearing SolvBTC，NAV > 1 BTC per token (当前约 1.034 BTC).

**Step C1: ERC-20 Approve SolvBTC → XSolvBTCPool**

```
--to: 0x7a56e1c57c7475ccf742a1832b028f0456652f97  (SolvBTC on Ethereum)
spender: 0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86  (XSolvBTCPool)
Selector: 0x095ea7b3
```

**Step C2: deposit() via XSolvBTCPool**

```
Function: deposit(uint256 solvBtcAmount_) → uint256 xSolvBtcAmount
Selector: 0xb6b55f25
Contract: XSolvBTCPool = 0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86
Chain: Ethereum (1) only
```

Calldata: `0xb6b55f25 + pad32(solvBtcAmount)`

```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86 \
  --input-data <calldata> \
  --force
```

---

### Operation D: Unwrap xSolvBTC → SolvBTC (Ethereum only)

withdraw fee = 0.05% (withdrawFeeRate = 5 / 10000)

**Step D1: ERC-20 Approve xSolvBTC → XSolvBTCPool**

```
--to: 0xd9d920aa40f578ab794426f5c90f6c731d159def  (xSolvBTC on Ethereum)
spender: 0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86  (XSolvBTCPool)
Selector: 0x095ea7b3
```

**Step D2: withdraw() via XSolvBTCPool**

```
Function: withdraw(uint256 xSolvBtcAmount_) → uint256 solvBtcAmount
Selector: 0x2e1a7d4d
Contract: XSolvBTCPool = 0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86
Chain: Ethereum (1) only
```

Calldata: `0x2e1a7d4d + pad32(xSolvBtcAmount)`

---

## Selector 验证表

| Function | Signature | Selector | 验证方式 |
|----------|-----------|----------|----------|
| approve | `approve(address,uint256)` | `0x095ea7b3` | 标准 ERC-20 |
| RouterV2 deposit | `deposit(address,address,uint256,uint256,uint64)` | `0x672262e5` | keccak256 计算 |
| RouterV2 withdrawRequest | `withdrawRequest(address,address,uint256)` | `0xd2cfd97d` | keccak256 计算 |
| RouterV2 cancelWithdrawRequest | `cancelWithdrawRequest(address,address,uint256)` | `0x42c7774b` | keccak256 计算 |
| XSolvBTCPool deposit | `deposit(uint256)` | `0xb6b55f25` | keccak256 计算 |
| XSolvBTCPool withdraw | `withdraw(uint256)` | `0x2e1a7d4d` | keccak256 计算 |
| balanceOf | `balanceOf(address)` | `0x70a08231` | 标准 ERC-20 |

> 验证命令: `python3 -c "from eth_hash.auto import keccak; print('0x'+keccak(b'<sig>').hex()[:8])"`

---

# §3 用户场景

## 场景 1: 用户在 Arbitrum 将 WBTC 兑换为 SolvBTC (Mint)

**前提:** 用户有 WBTC on Arbitrum，想持有 BTC 资产同时获取 Solv Protocol 生态收益。

**流程:**
1. 查询用户钱包地址: `onchainos wallet balance --chain 42161 --output json`
2. 查询用户 WBTC 余额 (on-chain read, balanceOf)
3. 查询 SolvBTC 当前价格: DeFiLlama Coins API
4. Step A1: approve WBTC → RouterV2 (0x095ea7b3)
5. 等待 3 秒 (nonce 安全间隔)
6. Step A2: RouterV2.deposit() (0x672262e5)
7. 返回: txHash + 预计收到的 SolvBTC 数量

**参数说明:**
- `amount`: WBTC 数量，8 decimals (e.g. 0.001 BTC = 100000 raw)
- `minimumTargetTokenAmount`: 0 (无滑点保护) 或基于 price 计算的最小值
- `expireTime`: current_unix_timestamp + 300

**示例:**
```
用户: "mint 0.001 WBTC worth of SolvBTC on Arbitrum"
→ approve 100000 (8 dec WBTC) to RouterV2
→ deposit(SolvBTC_addr, WBTC_addr, 100000, 0, now+300)
```

---

## 场景 2: 用户在 Ethereum 将 SolvBTC 包装为 xSolvBTC (Wrap for Yield)

**前提:** 用户已有 SolvBTC on Ethereum，想赚取 Solv Protocol 的 basis trading / staking 收益。

**流程:**
1. 查询用户 SolvBTC 余额 (balanceOf on chain 1)
2. 查询当前 xSolvBTC NAV = xSolvBTC.price / SolvBTC.price
3. 查询 XSolvBTCPool.depositAllowed() — 确认 deposit 开放
4. Step C1: approve SolvBTC → XSolvBTCPool (0x095ea7b3)
5. 等待 3 秒
6. Step C2: XSolvBTCPool.deposit(amount) (0xb6b55f25)
7. 返回: txHash + 预计收到的 xSolvBTC 数量 (基于当前 NAV 换算)

**NAV 换算:** `xSolvBTC_received ≈ solvBTC_amount / NAV`
- 当前 NAV ≈ 1.034，则 1 SolvBTC → ≈ 0.967 xSolvBTC

**示例:**
```
用户: "wrap 0.05 SolvBTC into xSolvBTC for yield"
→ approve 0.05e18 SolvBTC to XSolvBTCPool
→ XSolvBTCPool.deposit(0.05e18)
```

---

## 场景 3: 用户查询 SolvBTC 价格和 Solv Protocol TVL

**前提:** 用户想了解当前 SolvBTC 价值和协议规模。

**流程:**
1. 查询 SolvBTC 和 xSolvBTC 价格: DeFiLlama Coins API (单次请求同时查两个)
2. 计算 xSolvBTC NAV (BTC per xSolvBTC)
3. 查询 Solv Protocol TVL: `GET https://api.llama.fi/tvl/solv-protocol`
4. 格式化并返回

**示例输出:**
```
SolvBTC price: $66,821.13 (≈ 1.000 BTC)
xSolvBTC price: $69,105.83 (≈ 1.034 BTC per xSolvBTC)
xSolvBTC accumulated yield: +3.4% over SolvBTC
Solv Protocol TVL: $575.5M
```

---

# §4 外部 API 依赖

| API | 用途 | Endpoint | 备注 |
|-----|------|----------|------|
| DeFiLlama Coins | SolvBTC / xSolvBTC 价格 | `https://coins.llama.fi/prices/current/{keys}` | 免费，无需 API key |
| DeFiLlama TVL | Solv Protocol 总 TVL | `https://api.llama.fi/tvl/solv-protocol` | 免费，返回 USD 数字 |
| DeFiLlama Yields | Solv APY (basis trading) | `https://yields.llama.fi/pools` | 过滤 project=solv-basis-trading；APY 可能为 0 |
| Arbitrum RPC | 链上查询 | `https://arb1.arbitrum.io/rpc` | 标准 JSON-RPC |
| Ethereum RPC | 链上查询 | `https://ethereum.publicnode.com` | 勿用 cloudflare-eth.com |

**reqwest 代理注意:** 所有 HTTP 请求需通过 `build_client()` 读取 `HTTPS_PROXY` 环境变量。

---

# §5 配置参数

## 合约地址

### Arbitrum (chain_id: 42161)

```toml
[arbitrum]
chain_id = 42161
rpc_url = "https://arb1.arbitrum.io/rpc"

solvbtc_token = "0x3647c54c4c2c65bc7a2d63c0da2809b399dbbdc0"   # SolvBTC ERC-20, 18 decimals
wbtc_token = "0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f"       # WBTC ERC-20, 8 decimals
router_v2 = "0x92E8A4407FD1ae7a53a32f1f832184edF071080A"        # SolvBTCRouterV2Proxy
multi_asset_pool = "0xf00aa0442bD2abFA2Fe20B12a1f88104A61037c7" # SolvBTCMultiAssetPool

# Pool IDs (read from router.poolIds(targetToken, currency))
pool_id_wbtc_to_solvbtc = "0x488def4a346b409d5d57985a160cd216d29d4f555e1b716df4e04e2374d2d9f6"
```

### Ethereum Mainnet (chain_id: 1)

```toml
[ethereum]
chain_id = 1
rpc_url = "https://ethereum.publicnode.com"

solvbtc_token = "0x7a56e1c57c7475ccf742a1832b028f0456652f97"    # SolvBTC ERC-20, 18 decimals
xsolvbtc_token = "0xd9d920aa40f578ab794426f5c90f6c731d159def"   # xSolvBTC ERC-20, 18 decimals
wbtc_token = "0x2260fac5e5542a773aa44fbcfedf7c193bc2c599"       # WBTC ERC-20, 8 decimals
router_v2 = "0x3d93B9e8F0886358570646dAd9421564C5fE6334"        # SolvBTCRouterV2Proxy
multi_asset_pool = "0x1d5262919c4aab745a8c9dd56b80db9feaef86ba" # SolvBTCMultiAssetPool
xsolvbtc_pool = "0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86"    # XSolvBTCPool (instant swap)
xsolvbtc_oracle = "0xfC8ffd33dA2ba271668B899Ceb74618B465AffBF"  # XSolvBTCOracle

# Pool IDs
pool_id_wbtc_to_solvbtc = "0x716db7dc196abe78d5349c7166896f674ab978af26ada3e5b3ea74c5a1b48307"

# XSolvBTCPool fees
xsolvbtc_withdraw_fee_rate = 5   # 5/10000 = 0.05%
```

## 关键常量

```toml
WBTC_DECIMALS = 8
SOLVBTC_DECIMALS = 18
XSOLVBTC_DECIMALS = 18

# DeFiLlama coin keys
DEFI_LLAMA_SOLVBTC_ARB = "arbitrum:0x3647c54c4c2c65bc7a2d63c0da2809b399dbbdc0"
DEFI_LLAMA_SOLVBTC_ETH = "ethereum:0x7a56e1c57c7475ccf742a1832b028f0456652f97"
DEFI_LLAMA_XSOLVBTC_ETH = "ethereum:0xd9d920aa40f578ab794426f5c90f6c731d159def"
DEFI_LLAMA_PROTOCOL_SLUG = "solv-protocol"
```

---

# §6 实现注意事项

## A. approve + deposit 间隔

参考 `kb/protocols/lending.md`: approve 和 deposit 调用之间需 **3 秒延迟** 防止 nonce 碰撞。

```rust
erc20_approve(chain_id, wbtc_addr, router_addr, amount, dry_run).await?;
tokio::time::sleep(std::time::Duration::from_secs(3)).await;
router_deposit(chain_id, router_addr, solvbtc, wbtc, amount, min_out, expiry, dry_run).await?;
```

## B. WBTC 精度换算

WBTC = 8 decimals。用户输入 `0.001 BTC` → `100_000` raw units。

```rust
let raw_amount = (human_amount * 1e8) as u64;
```

## C. SolvBTC 赎回非即时

`withdrawRequest()` 不立即返回 WBTC，而是返回一个 ERC-3525 Semi-Fungible Token (redemption ticket)。插件必须：
1. 明确告知用户赎回是**非即时**的（需等待 OpenFundMarket 队列处理）
2. 展示 redemption SFT 的 address 和 tokenId 供追踪
3. 提供 `cancel-redeem` 操作允许撤销

## D. xSolvBTC 仅在 Ethereum Mainnet

`XSolvBTCPool` 部署仅在 Ethereum (chain 1)。Arbitrum 没有 xSolvBTC。
- 若用户在 Arbitrum 持有 SolvBTC 想 wrap to xSolvBTC，需先桥接到 Ethereum。
- 插件应在 `wrap`/`unwrap` 命令中校验 chain_id == 1，否则返回友好错误。

## E. expireTime 计算

`deposit()` 的 `expireTime_` 参数为 `uint64` Unix 时间戳。建议：
```rust
let expire_time: u64 = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)?
    .as_secs() + 300;  // +5 minutes
```

## F. 赎回流程中的 pool ID 自动路由

RouterV2 内部通过 `poolIds[targetToken][currency]` 映射查找对应的 OpenFundMarket pool，不需要用户提供 pool ID。Pool ID 仅在合约内部使用，插件不需要将其暴露给用户。

## G. EVM chain ID vs chain name

参照 `kb/onchainos/gotchas.md`:
- `defi` 命令用 chain name string: `--chain arbitrum`
- `wallet` 命令用 numeric chain ID: `--chain 42161`

## H. 测试建议 (L4)

**Arbitrum (42161) — 优先测试:**
1. `get-price` — DeFiLlama API，纯链下
2. `get-tvl` — DeFiLlama API，纯链下
3. `get-balance` — on-chain balanceOf
4. `mint` — approve WBTC + deposit (需少量 WBTC；可用 0.0001 WBTC = 10000 raw)

**Ethereum (1) — 次要测试:**
5. `wrap` — approve SolvBTC + XSolvBTCPool.deposit (需 SolvBTC + ETH gas)
6. `unwrap` — approve xSolvBTC + XSolvBTCPool.withdraw

**注意:** `redeem` (withdrawRequest) 会创建 pending SFT；建议 dry-run 模式验证 calldata 正确性，L4 实际执行后立即 `cancel-redeem` 取回 SolvBTC。

---

# §7 函数选择器完整参考

```
# 通用 ERC-20
approve(address,uint256)                              → 0x095ea7b3
balanceOf(address)                                    → 0x70a08231
allowance(address,address)                            → 0xdd62ed3e
decimals()                                            → 0x313ce567
totalSupply()                                         → 0x18160ddd

# SolvBTCRouterV2 (同一 ABI，Arbitrum 和 Ethereum)
deposit(address,address,uint256,uint256,uint64)       → 0x672262e5
withdrawRequest(address,address,uint256)              → 0xd2cfd97d
cancelWithdrawRequest(address,address,uint256)        → 0x42c7774b
poolIds(address,address)                              → 0x6534d8dc  [view]
multiAssetPools(address)                              → 0x7448130c  [view]

# XSolvBTCPool (Ethereum only)
deposit(uint256)                                      → 0xb6b55f25
withdraw(uint256)                                     → 0x2e1a7d4d
depositAllowed()                                      → 0xc30ea2a2  [view]
withdrawFeeRate()                                     → 0xea99e689  [view]

# XSolvBTCOracle (Ethereum only)
getNav(address)                                       → 0xfb596008  [view]
navDecimals(address)                                  → 0x(computed) [view]
```
