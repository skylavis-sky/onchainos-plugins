# Compound V3 (Comet) Plugin — design.md

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `compound-v3` |
| `dapp_name` | Compound V3 (Comet) |
| `binary_name` | `compound-v3` |
| `version` | `0.1.0` |
| `category` | `defi-protocol` |
| `tags` | `lending`, `borrowing`, `defi`, `compound`, `comet` |
| `target_chains` | Ethereum (1), Base (8453), Arbitrum (42161), Polygon (137) |
| `target_protocols` | Compound V3 Comet (lending/borrowing) |
| `source_repo` | `skylavis-sky/onchainos-plugins` |
| `source_dir` | `compound-v3` |
| `description` | Compound V3 (Comet) lending plugin: supply collateral, borrow/repay the base asset, and claim COMP rewards across Ethereum, Base, Arbitrum, and Polygon. |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **No.** Compound Finance provides no official Rust SDK. Only a JavaScript SDK exists: [Compound.js](https://docs.compound.finance/compound-js/). No community Rust crate found on crates.io or GitHub. |
| SDK 支持哪些技术栈？ | JavaScript/TypeScript only (Compound.js wraps Ethers.js). No Rust, Python, or Go SDKs. |
| 有 REST API？ | **No dedicated REST API.** All interaction is via on-chain contract calls (EVM `eth_call` for reads, transactions for writes). The Compound Finance API (`api.compound.finance/api/v2`) offers some market data but is not required for core operations. |
| 有官方 Skill？ | **No.** Compound Finance has no published onchainos/MCP skill. |
| 开源社区有类似 Skill？ | **No** compound-v3 skill found in onchainos plugin-store or similar DApp agent frameworks. The existing Aave V3 plugin in this pipeline is the closest reference (same lending pattern: approve → supply → borrow → repay → withdraw → claim). |
| 支持哪些链？ | Ethereum mainnet, Base, Arbitrum, Polygon, Optimism, Linea, Scroll, Mantle, Unichain, Ronin. **OnchainOS scope: Ethereum (1), Base (8453), Arbitrum (42161), Polygon (137).** |
| 是否需要 onchainos 广播？ | **Yes.** All write operations (supply, borrow/withdraw, repay, claim rewards) are on-chain EVM transactions. Must use `onchainos wallet contract-call` for all writes. Reads use direct `eth_call` via RPC. |

### 接入路径

**API（直接 RPC 调用）**

No Rust SDK exists and no relevant community Skill found. All reads are pure `eth_call` over JSON-RPC; all writes go through `onchainos wallet contract-call`. This mirrors the Aave V3 plugin approach exactly.

---

## §2 接口映射

### 2a. 需要接入的操作表

| 操作 | 类型 | 说明 |
|------|------|------|
| `get-markets` | 链下查询 | 列出所有市场（基础资产、APR、利用率） |
| `get-position` | 链下查询 | 查看账户的供应余额、借款余额、可借额度、是否被抵押 |
| `supply` | 链上操作 | 供应抵押品或基础资产（同时用于还款） |
| `borrow` | 链上操作 | 借出基础资产（通过 `withdraw` 合约方法） |
| `repay` | 链上操作 | 偿还借款（通过 `supply` 合约方法传入基础资产） |
| `withdraw` | 链上操作 | 提取已供应的抵押品 |
| `claim-rewards` | 链上操作 | 领取 COMP 奖励 |

---

### 2b. 链下查询表

#### `get-markets`

| 字段 | 值 |
|------|----|
| 方法 | 直接 `eth_call` via JSON-RPC |
| 合约方法 | `getUtilization()` → `getBorrowRate(uint)` → `getSupplyRate(uint)` |
| Comet `getUtilization()` 选择器 | `0xd7a5b8ab` |
| Comet `getSupplyRate(uint)` 选择器 | `0xd955759d` |
| Comet `getBorrowRate(uint)` 选择器 | `0x9fa83b5a` |
| `totalSupply()` 选择器 | `0x18160ddd` |
| `totalBorrow()` 选择器 | `0x8285ef40` |
| 关键参数 | `comet_address: Address`, `rpc_url: &str` |
| 返回值 | `utilization: u128` (1e18 scaled), `supplyRatePerSec: u64`, `borrowRatePerSec: u64`, `totalSupply: u128`, `totalBorrow: u128` |
| APR 换算 | `rate / 1e18 * 31_536_000 * 100` → 百分比 |

**已知 Comet 合约地址（运行时验证，不硬编码为主要来源）：**

| Chain | Market | Comet Proxy | Rewards |
|-------|--------|-------------|---------|
| Ethereum (1) | cUSDCv3 | `0xc3d688B66703497DAA19211EEdff47f25384cdc3` | `0x1B0e765F6224C21223AeA2af16c1C46E38885a40` |
| Base (8453) | cUSDCv3 | `0xb125E6687d4313864e53df431d5425969c15Eb2F` | `0x123964802e6ABabBE1Bc9547D72Ef1B69B00A6b1` |
| Arbitrum (42161) | cUSDCv3 | `0x9c4ec768c28520B50860ea7a15bd7213a9fF58bf` | `0x88730d254A2f7e6AC8388c3198aFd694bA9f7fae` |
| Polygon (137) | cUSDCv3 | `0xF25212E676D1F7F89Cd72fFEe66158f541246445` | `0x45939657d1CA34A8FA39A924B71D28Fe8431e581` |

> Source: [github.com/compound-finance/comet/deployments](https://github.com/compound-finance/comet/tree/main/deployments)
>
> **Runtime note:** Addresses should be loaded from a config table in `config.rs` (not hardcoded inline). The deployments folder is the authoritative source. Additional markets (WETH, USDT on Ethereum; WETH on Base/Arbitrum) can be added by reading the corresponding `roots.json`.

---

#### `get-position`

| 字段 | 值 |
|------|----|
| 方法 | 直接 `eth_call` via JSON-RPC |
| `balanceOf(address)` 选择器 | `0x70a08231` |
| `borrowBalanceOf(address)` 选择器 | `0x374c49b4` |
| `collateralBalanceOf(address,address)` 选择器 | `0x487dd147` |
| `isBorrowCollateralized(address)` 选择器 | `0x0f3bde75` |
| `getAssetInfo(uint8)` 选择器 | `0xc8c7fe6b` |
| 关键参数 | `comet_address: Address`, `wallet: Address`, `asset: Address` |
| 返回值 | `supply_balance: u128` (base asset, scaled by base decimals), `borrow_balance: u128`, `collateral_balance: u128`, `is_collateralized: bool` |

> `balanceOf()` returns supply balance if positive. `borrowBalanceOf()` returns the current debt including accrued interest. If `borrowBalanceOf > 0`, the account is a borrower.

---

### 2c. 链上写操作表

Compound V3 (Comet) 使用**单合约架构**。同一个 Comet 代理合约处理所有操作（supply、withdraw、borrow、repay）。

#### `supply`（供应抵押品 或 偿还借款）

**Comet 合约调用：**

```
function supply(address asset, uint256 amount)
selector: 0xf2b9fdb8
calldata: 0xf2b9fdb8
          000000000000000000000000<ASSET_ADDR_NO_0X>  (32 bytes)
          <AMOUNT_HEX_PADDED_32_BYTES>
```

**步骤：**
1. ERC-20 approve: 用户向 Comet 合约授权 `amount`
   ```
   onchainos wallet contract-call \
     --chain <CHAIN_ID> \
     --to <TOKEN_CONTRACT> \
     --input-data 0x095ea7b3<COMET_ADDR_PADDED><AMOUNT_PADDED> \
     --from <WALLET>
   ```
2. 3 秒延迟（防止 nonce 碰撞，同 Aave V3 模式）
3. Comet supply 调用:
   ```
   onchainos wallet contract-call \
     --chain <CHAIN_ID> \
     --to <COMET_PROXY> \
     --input-data 0xf2b9fdb8<ASSET_PADDED><AMOUNT_PADDED> \
     --from <WALLET>
   ```

> **注意：** supply 基础资产（如 USDC）同时用于**偿还借款**。如果账户有 borrow 余额，supply 将先偿还债务。

---

#### `borrow`（借款基础资产）

在 Compound V3 中，借款通过 **`withdraw` 合约方法**实现——提取基础资产时，若没有足够的 supply 余额，协议自动创建借款头寸。

```
function withdraw(address asset, uint256 amount)
selector: 0xf3fef3a3
calldata: 0xf3fef3a3
          000000000000000000000000<BASE_ASSET_ADDR_NO_0X>  (32 bytes)
          <AMOUNT_HEX_PADDED_32_BYTES>
```

```
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <COMET_PROXY> \
  --input-data 0xf3fef3a3<BASE_ASSET_PADDED><AMOUNT_PADDED> \
  --from <WALLET>
```

> **无需 ERC-20 approve** — Comet 直接铸造债务并发送基础资产给调用者。
> **前置检查：** 调用 `isBorrowCollateralized` 确认抵押品充足；检查 `baseBorrowMin`（最小借款量）。

---

#### `repay`（偿还借款）

偿还借款与 supply 基础资产使用**相同的 `supply()` 合约方法**。

步骤同 supply，仅资产为基础资产（如 USDC），amount 为借款余额。

**"全额还款"注意事项：**
- **不要使用 `type(uint256).max`**。与 Aave V3 相同，Comet 尝试从钱包拉取精确的债务金额，而利息持续累积，可能导致 revert。
- 正确做法：使用 `borrowBalanceOf(wallet)` 读取当前债务，再用钱包实际余额（取两者较小值）作为 repay amount。

```rust
let borrow_bal = rpc::call_borrow_balance_of(comet, &wallet, &rpc_url).await?;
let wallet_bal = rpc::get_erc20_balance(base_asset, &wallet, &rpc_url).await?;
let repay_amount = borrow_bal.min(wallet_bal);
```

---

#### `withdraw`（提取抵押品）

```
function withdraw(address asset, uint256 amount)
selector: 0xf3fef3a3
calldata: 0xf3fef3a3
          000000000000000000000000<COLLATERAL_ASSET_ADDR_NO_0X>
          <AMOUNT_HEX_PADDED_32_BYTES>
```

```
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <COMET_PROXY> \
  --input-data 0xf3fef3a3<COLLATERAL_ASSET_PADDED><AMOUNT_PADDED> \
  --from <WALLET>
```

> **无需 ERC-20 approve** — 直接提取。
> **前置检查：** 若账户有未偿债务，提取抵押品可能触发 `isBorrowCollateralized` 失败（抵押不足）。插件应在提取前检查 `borrowBalanceOf` — 若有债务，先还清后再提取全部抵押品。

---

#### `claim-rewards`（领取 COMP 奖励）

奖励通过独立的 **CometRewards 合约**领取（非 Comet 代理）。

```
function claimTo(address comet, address src, address to, bool shouldAccrue)
selector: 0x52a4ef2e
calldata: 0x52a4ef2e
          000000000000000000000000<COMET_PROXY_ADDR_NO_0X>   (32 bytes)
          000000000000000000000000<SRC_WALLET_NO_0X>          (32 bytes)
          000000000000000000000000<TO_WALLET_NO_0X>           (32 bytes)
          0000000000000000000000000000000000000000000000000000000000000001  (bool true)
```

```
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <REWARDS_CONTRACT> \
  --input-data 0x52a4ef2e<COMET_PADDED><SRC_PADDED><TO_PADDED><BOOL_TRUE> \
  --from <WALLET>
```

> **前置读取：** 先调用 `getRewardOwed(comet, account)` 在 CometRewards 合约上确认有奖励可领。若 `amtOwed == 0`，返回友好提示"无可领取奖励"，不发送交易。

---

## §3 用户场景

### 场景 1：查询市场和账户仓位（链下查询）

**用户说：** "查一下 Compound V3 在 Base 上的 USDC 市场情况，还有我的持仓。"

**Agent 动作序列：**

1. **[链下查询]** 调用 `getUtilization()` on Base cUSDCv3 (`0xb125E6687d4313864e53df431d5425969c15Eb2F`) via `eth_call`
2. **[链下查询]** 调用 `getSupplyRate(utilization)` 和 `getBorrowRate(utilization)` 计算年化利率
3. **[链下查询]** 调用 `totalSupply()` 和 `totalBorrow()` 获取市场规模
4. **[链下查询]** `onchainos wallet addresses` 获取用户 EVM 钱包地址
5. **[链下查询]** 调用 `balanceOf(wallet)` — 供应余额
6. **[链下查询]** 调用 `borrowBalanceOf(wallet)` — 借款余额
7. **[链下查询]** 调用 `collateralBalanceOf(wallet, weth_addr)` — 如有抵押品
8. **[展示]** 显示：供应 APR、借款 APR、利用率、我的供应量、我的借款量、我的抵押品

---

### 场景 2：供应 USDC 赚取利息（Happy Path）

**用户说：** "帮我在 Base 的 Compound V3 供应 500 USDC。"

**Agent 动作序列：**

1. **[链下查询]** `onchainos wallet addresses` — 获取钱包地址
2. **[链下查询]** `onchainos wallet balance --chain 8453 --token-address 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` — 确认 USDC 余额 ≥ 500
3. **[链下查询]** 调用 `getSupplyRate` 获取当前供应 APR，展示给用户
4. **[确认]** 向用户展示操作详情（500 USDC → Compound V3 Base，预期 APR），**请求用户确认**
5. **[链上操作]** ERC-20 approve: USDC.approve(cUSDCv3, 500_000000)
   ```
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
     --input-data 0x095ea7b3000000000000000000000000b125e6687d4313864e53df431d5425969c15eb2f000000000000000000000000000000000000000000000000000000001dcd6500 \
     --from <WALLET>
   ```
6. **[等待]** 3 秒延迟
7. **[链上操作]** Comet.supply(USDC, 500_000000)
   ```
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0xb125E6687d4313864e53df431d5425969c15Eb2F \
     --input-data 0xf2b9fdb8000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913000000000000000000000000000000000000000000000000000000001dcd6500 \
     --from <WALLET>
   ```
8. **[展示]** 返回 txHash，展示新的供应余额确认

---

### 场景 3：借款流程（供应抵押品后借款，含健康检查）

**用户说：** "我想在 Compound V3 Base 供应 0.1 WETH 作为抵押品，然后借 100 USDC。"

**Agent 动作序列：**

1. **[链下查询]** `onchainos wallet addresses` — 获取钱包地址
2. **[链下查询]** 检查 WETH 余额 ≥ 0.1 WETH (`0x4200000000000000000000000000000000000006`)
3. **[链下查询]** 调用 `getAssetInfoByAddress(WETH)` 获取 `borrowCollateralFactor`（WETH 抵押因子）
4. **[链下查询]** 计算最大可借额度：`0.1 WETH * WETH_price * borrowCollateralFactor`，确认 100 USDC 在安全范围内
5. **[链下查询]** 调用 `baseBorrowMin()` 确认 100 USDC ≥ 最小借款量
6. **[确认]** 展示：供应 0.1 WETH 抵押，借 100 USDC，当前借款 APR；**请求用户确认**
7. **[链上操作]** ERC-20 approve: WETH.approve(cUSDCv3, 100000000000000000)
8. **[等待]** 3 秒延迟
9. **[链上操作]** Comet.supply(WETH, 0.1e18) — 存入抵押品
10. **[链下查询]** 调用 `isBorrowCollateralized(wallet)` 确认抵押品已计入
11. **[链上操作]** Comet.withdraw(USDC, 100_000000) — 触发借款
    ```
    onchainos wallet contract-call \
      --chain 8453 \
      --to 0xb125E6687d4313864e53df431d5425969c15Eb2F \
      --input-data 0xf3fef3a3000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda029130000000000000000000000000000000000000000000000000000000005f5e100 \
      --from <WALLET>
    ```
12. **[展示]** 返回 txHash，展示当前借款余额

---

### 场景 4：还款（含防 overflow 保护）

**用户说：** "帮我还清 Compound V3 Base 上所有的 USDC 借款。"

**Agent 动作序列：**

1. **[链下查询]** `onchainos wallet addresses` — 获取钱包地址
2. **[链下查询]** 调用 `borrowBalanceOf(wallet)` on cUSDCv3 — 读取当前债务（含利息）
3. **[链下查询]** 检查钱包 USDC 余额
4. **[风控]** 若钱包余额 < 债务，提示用户余额不足，显示缺口，建议获取更多 USDC；终止流程
5. **[逻辑]** `repay_amount = min(borrow_balance, wallet_usdc_balance)` — 防止 revert（同 Aave V3 模式）
6. **[确认]** 展示还款金额，**请求用户确认**
7. **[链上操作]** ERC-20 approve: USDC.approve(cUSDCv3, repay_amount)
8. **[等待]** 3 秒延迟
9. **[链上操作]** Comet.supply(USDC, repay_amount) — supply 基础资产即为还款
10. **[链下查询]** 调用 `borrowBalanceOf(wallet)` 确认债务清零（允许 dust）
11. **[展示]** 还款成功，txHash，剩余债务（通常为 0 或极小 dust）

---

### 场景 5：领取 COMP 奖励

**用户说：** "帮我领取 Compound V3 在 Ethereum 上的 COMP 奖励。"

**Agent 动作序列：**

1. **[链下查询]** `onchainos wallet addresses` — 获取钱包地址
2. **[链下查询]** 调用 CometRewards `getRewardOwed(cUSDCv3_eth, wallet)` — 查询应计奖励
3. **[判断]** 若 `amtOwed == 0`，返回"暂无可领取的 COMP 奖励"，不发送交易
4. **[展示]** 显示可领取 COMP 数量，**请求用户确认**
5. **[链上操作]** CometRewards.claimTo(cUSDCv3, wallet, wallet, true)
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x1B0e765F6224C21223AeA2af16c1C46E38885a40 \
     --input-data 0x52a4ef2e \
                  000000000000000000000000c3d688b66703497daa19211eedff47f25384cdc3 \
                  000000000000000000000000<WALLET_NO_0X> \
                  000000000000000000000000<WALLET_NO_0X> \
                  0000000000000000000000000000000000000000000000000000000000000001 \
     --from <WALLET>
   ```
6. **[展示]** 返回 txHash，确认 COMP 已转入钱包

---

## §4 外部 API 依赖

| API | 用途 | Endpoint 示例 |
|-----|------|---------------|
| Ethereum RPC (`eth_call`) | 链下读操作：余额、利率、抵押因子 | `https://eth.llamarpc.com` |
| Base RPC (`eth_call`) | 链下读操作（Base） | `https://mainnet.base.org` 或 `https://base-rpc.publicnode.com` |
| Arbitrum RPC (`eth_call`) | 链下读操作（Arbitrum） | `https://arb1.arbitrum.io/rpc` |
| Polygon RPC (`eth_call`) | 链下读操作（Polygon） | `https://polygon-rpc.com` |
| onchainos CLI | 所有链上写操作 | 本地 CLI |

**RPC 选择注意事项：**
- Base 大量 `eth_call` 场景优先用 `https://base-rpc.publicnode.com`（避免 `-32016 over rate limit`）
- BSC 不在目标链范围内；若未来扩展，使用 `https://bsc-rpc.publicnode.com`

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `chain_id` | `u64` | `8453` (Base) | 目标链 ID |
| `market` | `String` | `"usdc"` | 市场名称（usdc、weth、usdt 等） |
| `dry_run` | `bool` | `false` | 若为 true，跳过所有链上交易，仅显示预期操作 |
| `rpc_url` | `String` | 每链默认值（见 §4） | 覆盖默认 RPC endpoint |
| `slippage_tolerance` | — | N/A | 无需（Compound V3 无 AMM 滑点） |

**Dry-run 处理方式：**
与其他插件一致，`dry_run` 在 wrapper 层处理（提前返回，不调用 onchainos CLI）。**绝不**将 `--dry-run` 标志传递给 `onchainos wallet contract-call`（该命令不支持此标志）。

---

## §6 合约接口速查

### Comet (cToken 代理) 核心选择器

| 方法 | 选择器 | 类型 |
|------|--------|------|
| `supply(address,uint256)` | `0xf2b9fdb8` | 写 |
| `withdraw(address,uint256)` | `0xf3fef3a3` | 写（借款 + 提取） |
| `balanceOf(address)` | `0x70a08231` | 读 |
| `borrowBalanceOf(address)` | `0x374c49b4` | 读 |
| `collateralBalanceOf(address,address)` | `0x487dd147` | 读 |
| `isBorrowCollateralized(address)` | `0x0f3bde75` | 读 |
| `getUtilization()` | `0xd7a5b8ab` | 读 |
| `getSupplyRate(uint256)` | `0xd955759d` | 读 |
| `getBorrowRate(uint256)` | `0x9fa83b5a` | 读 |
| `totalSupply()` | `0x18160ddd` | 读 |
| `totalBorrow()` | `0x8285ef40` | 读 |
| `getAssetInfo(uint8)` | `0xc8c7fe6b` | 读 |
| `getAssetInfoByAddress(address)` | `0x741a61a3` | 读 |
| `baseBorrowMin()` | `0x29f2a836` | 读 |

### ERC-20 approve（用于 supply/repay 前授权）

| 方法 | 选择器 |
|------|--------|
| `approve(address,uint256)` | `0x095ea7b3` |

### CometRewards 选择器

| 方法 | 选择器 |
|------|--------|
| `claimTo(address,address,address,bool)` | `0x52a4ef2e` |
| `getRewardOwed(address,address)` | `0xfd27b525` |

---

## §7 关键注意事项 & 风险

1. **supply 兼具还款语义：** Compound V3 中，向市场 supply 基础资产会自动偿还借款。插件应根据上下文（当前借款余额）向用户清晰解释这一行为。

2. **borrow = withdraw 基础资产：** 借款和提取基础资产使用相同的合约方法（`withdraw`）。插件通过检查 `borrowBalanceOf` 是否为 0 来区分两种场景并给用户清晰说明。

3. **repay overflow 防护：** 不使用 `uint256.max`；读取 `borrowBalanceOf` 后取 `min(borrow, wallet_balance)` 作为还款额（与 Aave V3 相同处理）。

4. **withdraw 前检查债务：** 提取抵押品前必须确认 `borrowBalanceOf == 0`；否则会导致抵押不足 revert。提示用户先还清债务。

5. **ERC-20 approve 后 3 秒延迟：** approve 和 supply/repay 在同一秒提交会导致 nonce 碰撞（与 Aave V3 相同模式）。

6. **baseBorrowMin 检查：** 借款前需确认借款额 ≥ `baseBorrowMin()`（通常为 100 USDC 等值），否则交易 revert。

7. **多市场支持：** 每条链上可能有多个 Comet 实例（cUSDCv3、cWETHv3 等）。插件通过 `market` 配置参数选择正确的 Comet 代理地址。

8. **E106 合规：** SKILL.md 中每个 `wallet contract-call` 使用处均需包含用户确认提示，符合 lint 规则。
