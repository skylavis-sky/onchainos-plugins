# design.md — PancakeSwap CLMM (V3 Concentrated Liquidity)

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `pancakeswap-clmm` |
| `dapp_name` | PancakeSwap V3 CLMM |
| `version` | 0.1.0 |
| `target_chains` | BSC (56), Ethereum (1), Base (8453), Arbitrum (42161) |
| `target_protocols` | NonfungiblePositionManager, MasterChefV3, PancakeV3Factory, QuoterV2 |
| `binary_name` | pancakeswap-clmm |
| `source_repo` | skylavis-sky/onchainos-plugins |
| `source_dir` | pancakeswap-clmm |
| `category` | defi-protocol |
| `tags` | dex, liquidity, clmm, farming, v3, pancakeswap |

---

## Scope Clarification vs. Existing PancakeSwap Plugin (PR #82)

The existing `pancakeswap` plugin (PR #82) already implements:
- `swap` — token swap via SmartRouter
- `quote` — QuoterV2 price quote
- `pools` — factory pool discovery
- `positions` — view LP NFT positions (BSC + Base only)
- `add-liquidity` — mint new CLMM position via NonfungiblePositionManager
- `remove-liquidity` — decreaseLiquidity + collect

**This plugin is distinct in two ways:**

1. **MasterChefV3 Farming** — The existing plugin has zero coverage of LP NFT staking for CAKE rewards. MasterChefV3 is the dedicated farming contract where users stake their position NFTs to earn CAKE emissions. This is a major CLMM-specific workflow not present in any current plugin.

2. **Multi-chain expansion** — The existing plugin only targets BSC (56) and Base (8453). This plugin adds Ethereum (1) and Arbitrum (42161), completing the four primary CLMM deployments. The chain-specific contract addresses differ only for MasterChefV3 and SmartRouter.

**Recommended scope for this plugin:**
- `farm` — stake LP NFT into MasterChefV3
- `unfarm` — withdraw LP NFT from MasterChefV3 (also harvests pending CAKE)
- `harvest` — collect CAKE rewards without withdrawing position
- `pending-rewards` — view pending CAKE for a staked position
- `farm-pools` — list active farming pools with allocation points and APR
- `positions` — list positions (staked and unstaked) across all 4 chains
- `collect-fees` — collect accumulated swap fees from unstaked positions (standalone, without removing liquidity)

This scope is non-overlapping with PR #82 and covers the entire lifecycle of CLMM farming. The `positions` command in this plugin includes cross-chain support (4 chains) and shows staking status (staked in MasterChefV3 vs. held in wallet), which is richer than the existing plugin's BSC+Base only version.

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无。PancakeSwap 无官方 Rust SDK。官方 SDK 为 TypeScript：`@pancakeswap/v3-sdk`, `@pancakeswap/smart-router`。 |
| SDK 支持哪些技术栈？ | TypeScript/JavaScript only。来源：https://github.com/pancakeswap/pancake-frontend/tree/develop/packages/v3-sdk |
| 有 REST API？ | 无官方 REST API for position/farming data。有 The Graph subgraph（GraphQL）用于历史数据，但生产质量欠佳；链上 `eth_call` 可直接查询合约状态，是更可靠的数据源。 |
| 有官方 Skill？ | 无。官方未发布 MCP/Skill。 |
| 开源社区有类似 Skill？ | 有参考 Skill：`skylavis-sky/onchainos-plugins` 中的 `pancakeswap` 目录（PR #82）已实现 swap/LP 核心逻辑。本插件可复用其 `rpc.rs`、`onchainos.rs`、`config.rs` 基础结构，并扩展 MasterChefV3 相关命令。 |
| 支持哪些链？ | V3 CLMM 已部署于：BSC (56), Ethereum (1), Base (8453), Arbitrum (42161), opBNB, Linea, zkSync, Monad。本插件覆盖前四条主网链（OnchainOS 支持的 EVM 链）。 |
| 是否需要 onchainos 广播？ | **Yes**。`farm`（safeTransferFrom NFT 到 MasterChefV3）、`unfarm`（withdraw）、`harvest`（harvest）、`collect-fees`（collect from NonfungiblePositionManager）均为链上写操作，必须通过 `onchainos wallet contract-call` 提交。 |

**接入路径：** 参考已有 Skill (`pancakeswap` PR #82) — 复用 rpc/onchainos 基础结构，扩展 MasterChefV3 farming 命令并增加 Ethereum + Arbitrum 链支持。

---

## §2 接口映射

### 2a. 操作列表

| 操作 | 类型 | 优先级 |
|------|------|--------|
| `farm` — 将 LP NFT 质押到 MasterChefV3 | 链上写 | 核心 |
| `unfarm` — 从 MasterChefV3 取回 NFT（同时 harvest CAKE） | 链上写 | 核心 |
| `harvest` — 只领取 CAKE 奖励，不取回 NFT | 链上写 | 核心 |
| `pending-rewards` — 查看待领 CAKE 数量 | 链下读 | 核心 |
| `farm-pools` — 列出 MasterChefV3 活跃农场池 | 链下读 | 核心 |
| `positions` — 查看用户所有 V3 LP 持仓（含质押状态）| 链下读 | 核心 |
| `collect-fees` — 从未质押持仓收取累计手续费 | 链上写 | 辅助 |

---

### 2b. 链下查询接口

#### 操作：`pending-rewards`

**合约：** MasterChefV3（各链地址见 §2e）

**方法：** `pendingCake(uint256 tokenId) → (uint256 reward)`
- selector: `0xce5f39c6`（来源：eth_utils keccak256 验证）
- ABI 编码：`0xce5f39c6` + tokenId (uint256, 32字节)

**参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `token_id` | `u256` | LP NFT 的 ERC-721 token ID |
| `chain` | `u64` | 链 ID（默认 56） |

**返回值：** `uint256 reward`（CAKE wei，除以 1e18 得 CAKE 数量）

---

#### 操作：`farm-pools`

**合约：** MasterChefV3（各链地址见 §2e）

**方法 1：** `poolLength() → (uint256)`
- selector: `0x081e3eda`（来源：eth_utils keccak256 验证）
- 无参数

**方法 2：** `poolInfo(uint256 pid) → (uint256 allocPoint, address v3Pool, address token0, address token1, uint24 fee, uint256 totalLiquidity, uint256 totalBoostLiquidity)`
- selector: `0x1526fe27`（来源：eth_utils keccak256 验证）
- 参数：pid (uint256, 32字节)

**参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `chain` | `u64` | 链 ID（默认 56） |

**返回值：** 每个 pid 的池信息，包含代币地址、fee tier、分配权重

---

#### 操作：`positions`

**合约 1：** NonfungiblePositionManager（各链地址见 §2e）

**方法 1：** `balanceOf(address owner) → (uint256)`
- selector: `0x70a08231`（来源：eth_utils keccak256 验证）

**方法 2：** `tokenOfOwnerByIndex(address owner, uint256 index) → (uint256 tokenId)`
- selector: `0x2f745c59`（来源：eth_utils keccak256 验证）

**方法 3：** `positions(uint256 tokenId) → (uint96 nonce, address operator, address token0, address token1, uint24 fee, int24 tickLower, int24 tickUpper, uint128 liquidity, uint256 feeGrowthInside0LastX128, uint256 feeGrowthInside1LastX128, uint128 tokensOwed0, uint128 tokensOwed1)`
- selector: `0x99fbab88`（来源：eth_utils keccak256 验证）

**合约 2：** MasterChefV3（查询已质押 NFT）

**方法：** `userPositionInfos(uint256 tokenId) → (uint128 liquidity, uint128 boostLiquidity, int24 tickLower, int24 tickUpper, uint256 rewardGrowthInside, uint128 reward, address user, uint256 pid, uint256 boostMultiplier)`
- selector: `0x3b1acf74`（来源：eth_utils keccak256 验证）

**参数：**

| 参数 | 类型 | 说明 |
|------|------|------|
| `owner` | `String` | 钱包地址（可选，默认使用当前登录钱包） |
| `chain` | `u64` | 链 ID（默认 56） |

**返回值：** token0/token1 地址、fee、tick 范围、流动性、未收手续费、是否在农场中质押

---

### 2c. 链上写操作接口

#### 操作：`farm`（质押 LP NFT 到 MasterChefV3）

**原理：** MasterChefV3 通过 ERC-721 的 `onERC721Received` hook 接收 NFT。用户直接调用 NonfungiblePositionManager 的 `safeTransferFrom`，将 NFT 转移到 MasterChefV3 地址即完成质押。无需单独的 `deposit()` 函数。

**步骤 1：检查 NFT 所有权**
- `eth_call` → `ownerOf(uint256 tokenId)` on NonfungiblePositionManager
- selector: `0x6352211e`（来源：eth_utils keccak256）
- 确认 NFT 属于当前钱包

**步骤 2：safeTransferFrom — 质押 NFT**

```
合约：NonfungiblePositionManager (各链地址见 §2e)
方法：safeTransferFrom(address from, address to, uint256 tokenId)
selector: 0x42842e0e  (来源: eth_utils keccak256 验证)

ABI 编码 calldata:
0x42842e0e
<from: 32字节, 左填充0>      # 用户钱包地址 (resolve_wallet())
<to: 32字节, 左填充0>        # MasterChefV3 合约地址
<tokenId: 32字节>            # LP NFT token ID

onchainos 命令:
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <NONFUNGIBLE_POSITION_MANAGER> \
  --input-data 0x42842e0e<from_padded><masterchef_padded><tokenId_padded> \
  --from <WALLET_ADDR> \
  --force
```

**注意：** 用户在执行前必须确认。dry_run 模式跳过广播，仅展示 calldata。

---

#### 操作：`unfarm`（从 MasterChefV3 取回 NFT）

**步骤：** `withdraw(uint256 tokenId, address to)` on MasterChefV3

```
合约：MasterChefV3 (各链地址见 §2e)
方法：withdraw(uint256 tokenId, address to)
selector: 0x00f714ce  (来源: eth_utils keccak256 验证)

ABI 编码 calldata:
0x00f714ce
<tokenId: 32字节>           # LP NFT token ID
<to: 32字节, 左填充0>       # 接收 NFT 的地址 (resolve_wallet())

onchainos 命令:
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <MASTERCHEF_V3> \
  --input-data 0x00f714ce<tokenId_padded><to_padded> \
  --from <WALLET_ADDR> \
  --force
```

**注意：** `withdraw` 自动 harvest 所有待领 CAKE 并归还 NFT。用户执行前必须确认。

---

#### 操作：`harvest`（只领取 CAKE，不取回 NFT）

**步骤：** `harvest(uint256 tokenId, address to)` on MasterChefV3

```
合约：MasterChefV3 (各链地址见 §2e)
方法：harvest(uint256 tokenId, address to)
selector: 0x18fccc76  (来源: eth_utils keccak256 验证)

ABI 编码 calldata:
0x18fccc76
<tokenId: 32字节>           # LP NFT token ID
<to: 32字节, 左填充0>       # CAKE 接收地址 (resolve_wallet())

onchainos 命令:
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <MASTERCHEF_V3> \
  --input-data 0x18fccc76<tokenId_padded><to_padded> \
  --from <WALLET_ADDR> \
  --force
```

**注意：** 用户确认后执行。如果 pending CAKE 为 0，应友好提示"无待领奖励"而不是报错。

---

#### 操作：`collect-fees`（收取 NonfungiblePositionManager 中的累计手续费）

**适用场景：** 持仓未质押在 MasterChefV3（在钱包中），收取 swap 手续费。已质押的持仓须先 `unfarm` 取回。

**步骤：** `collect((uint256 tokenId, address recipient, uint128 amount0Max, uint128 amount1Max))` on NonfungiblePositionManager

```
合约：NonfungiblePositionManager (各链地址见 §2e)
方法：collect((uint256,address,uint128,uint128))
selector: 0xfc6f7865  (来源: eth_utils keccak256 验证)

ABI 编码 calldata:
0xfc6f7865
<tokenId: 32字节>           # LP NFT token ID
<recipient: 32字节, 左填充0> # 手续费接收地址 (resolve_wallet())
<amount0Max: 32字节>        # type(uint128).max = 0xffffffffffffffffffffffffffffffff (右对齐到32字节)
<amount1Max: 32字节>        # type(uint128).max = 同上

onchainos 命令:
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <NONFUNGIBLE_POSITION_MANAGER> \
  --input-data 0xfc6f7865<tokenId_padded><recipient_padded><amount0Max_padded><amount1Max_padded> \
  --from <WALLET_ADDR> \
  --force
```

**注意：** `amount0Max` 和 `amount1Max` 设为 `uint128::MAX`（`0x0000...ffffffff...` 32字节）以收取全部手续费。用户执行前必须确认。

---

### 2d. Tick 解码注意事项

从 `positions()` 和 `userPositionInfos()` 返回的 ABI 响应中，`tickLower` 和 `tickUpper` 为 `int24` 类型，编码为 32 字节 big-endian 有符号整数。解码时必须：

```rust
// 取最后 6 个 hex chars（3字节 = int24 范围）
// 但 int24 在 ABI 中以 int256 形式 sign-extended，
// 安全做法是取最后 8 hex chars，cast 为 i32：
fn decode_tick(hex_str: &str) -> i32 {
    let clean = hex_str.trim_start_matches("0x");
    let last8 = &clean[clean.len().saturating_sub(8)..];
    u32::from_str_radix(last8, 16).unwrap_or(0) as i32
}
```

参见 `kb/protocols/dex.md#tick-decoding`。

---

### 2e. 合约地址表

所有地址在运行时从 `config.rs` 中按 chain_id 查找，**不得硬编码**在命令逻辑中。

#### NonfungiblePositionManager

| 链 | Chain ID | 地址 | 来源验证 |
|----|----------|------|----------|
| BSC | 56 | `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364` | BscScan 已验证（PancakeSwap V3: Positions NFT-V1） |
| Ethereum | 1 | `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364` | Etherscan 已验证 |
| Base | 8453 | `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364` | BaseScan 已验证 |
| Arbitrum | 42161 | `0x46A15B0b27311cedF172AB29E4f4766fbE7F4364` | 来源：PancakeSwap frontend v3-sdk constants.ts |

#### PancakeV3Factory

| 链 | Chain ID | 地址 | 来源验证 |
|----|----------|------|----------|
| BSC | 56 | `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` | BscScan 已验证（PancakeSwap V3: Factory） |
| Ethereum | 1 | `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` | 来源：v3-sdk constants.ts |
| Base | 8453 | `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` | 来源：v3-sdk constants.ts |
| Arbitrum | 42161 | `0x0BFbCF9fa4f9C56B0F40a671Ad40E0805A091865` | 来源：v3-sdk constants.ts |

#### MasterChefV3

| 链 | Chain ID | 地址 | 来源验证 |
|----|----------|------|----------|
| BSC | 56 | `0x556B9306565093C855AEA9AE92A594704c2Cd59e` | BscScan 已验证（PancakeSwap V3: Masterchef）；Etherscan 二次验证 |
| Ethereum | 1 | `0x556B9306565093C855AEA9AE92A594704c2Cd59e` | Etherscan 已验证（PancakeSwap V3: Masterchef） |
| Base | 8453 | `0xC6A2Db661D5a5690172d8eB0a7DEA2d3008665A3` | BaseScan 已验证；来源：pancake-frontend farms const.ts |
| Arbitrum | 42161 | `0x5e09ACf80C0296740eC5d6F643005a4ef8DaA694` | 来源：pancake-frontend farms const.ts |

#### QuoterV2

| 链 | Chain ID | 地址 | 来源验证 |
|----|----------|------|----------|
| BSC | 56 | `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` | BscScan 已验证（PancakeSwap: Quoter v2） |
| Ethereum | 1 | `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` | Etherscan 已验证 |
| Base | 8453 | `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` | BaseScan 已验证 |
| Arbitrum | 42161 | `0xB048Bbc1Ee6b733FFfCFb9e9CeF7375518e25997` | 同地址跨链部署，与 BSC/Ethereum/Base 一致 |

#### SmartRouter

| 链 | Chain ID | 地址 | 来源验证 |
|----|----------|------|----------|
| BSC | 56 | `0x13f4EA83D0bd40E75C8222255bc855a974568Dd4` | BscScan 已验证（PancakeSwap V3: Smart Router） |
| Ethereum | 1 | `0x13f4EA83D0bd40E75C8222255bc855a974568Dd4` | Etherscan 已验证 |
| Base | 8453 | `0x678Aa4bF4E210cf2166753e054d5b7c31cc7fa86` | BaseScan 已验证（PancakeSwap V3: Smart Router） |
| Arbitrum | 42161 | 需运行时从 PancakeSwap 官方来源动态解析（SmartRouter 不在本插件核心功能中使用） |

> **注意：** SmartRouter 在本插件中不直接用于 swap（swap 由 PR #82 的 `pancakeswap` 插件负责）。此处记录以供参考。

---

### 2f. 已验证 Function Selectors 汇总

所有 selector 均通过 `eth_utils.keccak` (Python, Keccak-256) 计算并验证，符合 EVM ABI 规范。

| Selector | 函数签名 | 用于 |
|----------|----------|------|
| `0x42842e0e` | `safeTransferFrom(address,address,uint256)` | `farm` — 质押 NFT |
| `0x00f714ce` | `withdraw(uint256,address)` | `unfarm` — 取回 NFT + harvest |
| `0x18fccc76` | `harvest(uint256,address)` | `harvest` — 只领 CAKE |
| `0xce5f39c6` | `pendingCake(uint256)` | `pending-rewards` — 查询待领 |
| `0xfc6f7865` | `collect((uint256,address,uint128,uint128))` | `collect-fees` — 收手续费 |
| `0x99fbab88` | `positions(uint256)` | `positions` — 查询持仓详情 |
| `0x70a08231` | `balanceOf(address)` | `positions` — 持仓数量 |
| `0x2f745c59` | `tokenOfOwnerByIndex(address,uint256)` | `positions` — 枚举 NFT |
| `0x3b1acf74` | `userPositionInfos(uint256)` | `positions` — 质押状态 |
| `0x1526fe27` | `poolInfo(uint256)` | `farm-pools` — 池信息 |
| `0x081e3eda` | `poolLength()` | `farm-pools` — 池数量 |
| `0x6352211e` | `ownerOf(uint256)` | `farm` pre-check — NFT 所有权验证 |
| `0x1698ee82` | `getPool(address,address,uint24)` | 池存在性验证 |

---

## §3 用户场景

### 场景 1：质押 LP NFT 到农场赚取 CAKE（Happy Path）

**用户说：** "我想把我在 BSC 上的 PancakeSwap V3 LP 头寸质押到农场里赚 CAKE"

**Agent 动作序列：**

1. **链下查询 — 获取钱包地址**
   `onchainos wallet balance --chain 56 --output json` → 提取 `data.address`

2. **链下查询 — 查看未质押 LP 持仓**
   调用 NonfungiblePositionManager `balanceOf(walletAddr)` via `eth_call` (BSC, chain 56)
   如 balance = 0，提示"当前无 V3 LP 持仓，请先添加流动性"并退出

3. **链下查询 — 枚举 NFT token IDs**
   遍历 `tokenOfOwnerByIndex(walletAddr, index)` for index in 0..balance
   收集所有 token IDs，调用 `positions(tokenId)` 展示 token0/token1/fee/流动性

4. **链下查询 — 检查 MasterChefV3 支持哪些池**
   调用 `poolLength()` on MasterChefV3，遍历 `poolInfo(pid)` 找到匹配的 (token0, token1, fee) 池
   如目标持仓对应的池不在活跃农场中，提示"此持仓对应的池暂无 CAKE 激励"

5. **Agent 展示**
   显示可质押的 NFT 列表（token ID、代币对、流动性大小）及对应农场预估 APR

6. **Agent 询问用户确认**
   "您确认要将 Token ID #12345 (WBNB/USDC 0.05%) 质押到 MasterChefV3 赚取 CAKE 吗？"

7. **链上写操作 — 质押 NFT（safeTransferFrom）**
   ```
   onchainos wallet contract-call \
     --chain 56 \
     --to 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364 \
     --input-data 0x42842e0e<wallet_padded><masterchef_padded><tokenId_padded> \
     --from <wallet> \
     --force
   ```
   等待 5 秒，提取 `data.txHash`

8. **链下验证 — 确认质押成功**
   调用 `userPositionInfos(tokenId)` on MasterChefV3，验证 `user` 字段 = 钱包地址
   显示"质押成功！Token ID #12345 已在农场中，开始赚取 CAKE"

---

### 场景 2：查看所有持仓及待领 CAKE 奖励

**用户说：** "查看我在 PancakeSwap V3 上的所有持仓，包括已质押的，以及我有多少 CAKE 可以领"

**Agent 动作序列：**

1. **链下查询 — 获取钱包地址**
   `onchainos wallet balance --chain 56 --output json`

2. **链下查询 — 未质押持仓（NonfungiblePositionManager）**
   `balanceOf(walletAddr)` → 枚举 token IDs → 对每个调用 `positions(tokenId)`
   收集：token0/token1/fee/tickLower/tickUpper/liquidity/tokensOwed0/tokensOwed1

3. **链下查询 — 已质押持仓（MasterChefV3）**
   已质押的 NFT 不在用户钱包，通过扫描 MasterChefV3 的 `userPositionInfos` 查询
   策略：检查 MasterChefV3 的已知 token ID 范围（通过链上事件索引或已知 pid 列表），或在 positions 命令中接受 `--token-ids` 参数直接查询

4. **链下查询 — 待领 CAKE**
   对每个已质押 token ID 调用 `pendingCake(tokenId)` on MasterChefV3
   汇总总待领 CAKE（wei → 除以 1e18）

5. **Agent 展示**
   ```
   === 未质押持仓 (2个) ===
   #11111 WBNB/USDT 0.25%  流动性: 1,234,567  累计手续费: 0.05 WBNB + 15 USDT
   #22222 CAKE/BNB 1%      流动性: 500,000    累计手续费: 2 CAKE + 0.001 BNB

   === 已质押持仓 (1个) ===
   #33333 BUSD/USDC 0.01%  流动性: 9,999,999  待领 CAKE: 12.45 CAKE

   总待领 CAKE: 12.45 CAKE (~$35.80 USD)
   ```

---

### 场景 3：领取 CAKE 奖励（风控场景）

**用户说：** "领取我 #33333 头寸的 CAKE 奖励"

**Agent 动作序列：**

1. **链下查询 — 验证持仓存在且在农场中**
   调用 `userPositionInfos(33333)` on MasterChefV3 (BSC)
   如 `user != walletAddr`，提示"Token ID #33333 未在 MasterChefV3 质押，无法 harvest；如需领取手续费请使用 collect-fees 命令"

2. **链下查询 — 查看待领 CAKE**
   调用 `pendingCake(33333)` on MasterChefV3
   如 pending = 0，提示"当前无待领 CAKE 奖励"并退出（不发送 tx）

3. **风控检查 — CAKE 代币安全扫描**
   `onchainos security token-scan --address 0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82 --chain 56`
   (CAKE 合约地址，BSC)
   如扫描结果为 block，中止；warn 则展示警告，继续由用户决定

4. **Agent 询问用户确认**
   "您确认要领取 Token ID #33333 的 12.45 CAKE 奖励（约 $35.80）吗？"

5. **链上写操作 — harvest**
   ```
   onchainos wallet contract-call \
     --chain 56 \
     --to 0x556B9306565093C855AEA9AE92A594704c2Cd59e \
     --input-data 0x18fccc76<tokenId_padded><recipient_padded> \
     --from <wallet> \
     --force
   ```

6. **链下验证 — 确认 CAKE 余额变化**
   `onchainos wallet balance --chain 56` 确认 CAKE 余额增加
   展示"成功领取 12.45 CAKE，交易: 0x..."

---

### 场景 4：解除质押并收回 NFT（unfarm）

**用户说：** "把我 #33333 的头寸从农场里取出来"

**Agent 动作序列：**

1. **链下查询 — 验证质押状态**
   `userPositionInfos(33333)` on MasterChefV3，确认 `user == walletAddr`

2. **链下查询 — 显示待领 CAKE**
   `pendingCake(33333)`，展示"取回同时将自动领取 X.XX CAKE 奖励"

3. **Agent 询问用户确认**
   "您确认要从农场取回 Token ID #33333 (BUSD/USDC 0.01%) 吗？取回后将自动领取 12.45 CAKE 奖励，但不再产生新的 CAKE 激励。"

4. **链上写操作 — withdraw（NFT 取回 + harvest）**
   ```
   onchainos wallet contract-call \
     --chain 56 \
     --to 0x556B9306565093C855AEA9AE92A594704c2Cd59e \
     --input-data 0x00f714ce<tokenId_padded><to_padded> \
     --from <wallet> \
     --force
   ```
   等待 5 秒

5. **链下验证 — 确认 NFT 已返回钱包**
   `ownerOf(33333)` on NonfungiblePositionManager，确认 owner = walletAddr
   展示"NFT #33333 已取回到钱包，同时领取了 12.45 CAKE"

---

### 场景 5：收取累计手续费（collect-fees）

**用户说：** "帮我收取 #11111 头寸赚到的手续费"

**Agent 动作序列：**

1. **链下查询 — 检查持仓未在农场质押**
   `ownerOf(11111)` on NonfungiblePositionManager，确认 owner = walletAddr
   如 owner = MasterChefV3，提示"此持仓已质押在农场，请先 unfarm 取回再收取手续费"

2. **链下查询 — 查看累计手续费**
   `positions(11111)` → 提取 `tokensOwed0` 和 `tokensOwed1`
   如两者均为 0，提示"当前无累计手续费可收取"并退出

3. **Agent 询问用户确认**
   "Token #11111 累计手续费：0.05 WBNB + 15 USDT。确认收取？"

4. **链上写操作 — collect**
   ```
   onchainos wallet contract-call \
     --chain 56 \
     --to 0x46A15B0b27311cedF172AB29E4f4766fbE7F4364 \
     --input-data 0xfc6f7865<tokenId_padded><recipient_padded><amount0Max><amount1Max> \
     --from <wallet> \
     --force
   ```
   其中 amount0Max = amount1Max = `0x00000000000000000000000000000000ffffffffffffffffffffffffffffffff` (uint128::MAX, 32字节)

5. **链下验证**
   展示"成功收取手续费，交易: 0x..."

---

## §4 外部 API 依赖

| API | 用途 | 端点 | 认证 |
|-----|------|------|------|
| BSC RPC | eth_call、所有链上读取 | `https://bsc-rpc.publicnode.com` | 无 |
| Ethereum RPC | eth_call | `https://ethereum.publicnode.com` | 无 |
| Base RPC | eth_call | `https://base-rpc.publicnode.com` | 无 |
| Arbitrum RPC | eth_call | `https://arb1.arbitrum.io/rpc` | 无 |
| onchainos wallet | 地址解析、链上写操作、余额查询 | onchainos CLI | 登录状态 |
| onchainos security | CAKE token 安全扫描（harvest 前） | onchainos CLI | 无 |

> **RPC 选择依据：**
> - BSC: `bsc-dataseed.binance.org` 在 sandbox 有 TLS 问题，使用 publicnode.com（见 `kb/onchainos/gotchas.md#bsc-rpc-tls`）
> - Base: `mainnet.base.org` 在多次 eth_call 下触发限流，使用 publicnode.com（见 `kb/onchainos/gotchas.md#base-rpc-rate-limit`）
> - Ethereum: `cloudflare-eth.com` 不可靠，使用 publicnode.com（见 `kb/onchainos/gotchas.md#cloudflare-eth-bad`）

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `chain` | `u64` | `56` | 默认链 ID（BSC）；支持 56/1/8453/42161 |
| `dry_run` | `bool` | `false` | true 时跳过广播，仅打印 calldata 和模拟结果 |
| `rpc_url` | `Option<String>` | 按 chain 自动选择（见 §4） | 可覆盖默认 RPC |
| `gas_limit` | `Option<u64>` | 自动估算（`onchainos gateway gas-limit`） | farm/harvest 操作 gas 上限 |
| `slippage_bps` | `u64` | `100` (1%) | 仅 collect-fees 中无需此参数；farm/unfarm 不涉及 slippage |

> **dry_run 说明：** `--dry-run` flag 不能直接传给 `onchainos wallet contract-call`（见 `kb/onchainos/gotchas.md#dry-run-flag`）。dry_run 必须在 plugin wrapper 层处理，提前返回 calldata 展示，不调用 onchainos。

---

## §6 已知限制与开发注意事项

### 6a. 已质押 NFT 的枚举问题

已质押到 MasterChefV3 的 NFT 不在用户钱包，`balanceOf(walletAddr)` 无法枚举。解决策略：

1. `positions` 命令增加可选参数 `--token-ids <id1,id2,...>` 让用户指定已质押的 token ID
2. 或通过 The Graph subgraph 查询历史 `Deposit` 事件（需要 subgraph URL，稳定性有限）
3. **推荐实现：** 默认展示钱包中的未质押持仓，加 `--include-staked <tokenId1,tokenId2>` 可选参数查询质押持仓。在 SKILL.md 中说明此限制。

### 6b. NFT 质押通过 safeTransferFrom 而非 deposit()

PancakeSwap MasterChefV3 的质押机制是通过 ERC-721 的 `onERC721Received` 回调实现的，不存在单独的 `deposit()` 函数。`safeTransferFrom` 将 NFT 转入合约地址即完成质押。这与 ERC-20 farming 的常见模式不同，需在 SKILL.md 中解释清楚。

### 6c. 多步骤操作间的 nonce 延迟

`farm` 操作如需先 approve（ERC-721 approve for MasterChefV3），需在 approve 和 safeTransferFrom 之间等待 5 秒（见 `kb/protocols/dex.md#lp-nonce-delay`）。实际上 ERC-721 的 `safeTransferFrom` 不需要预先 approve（`from` 就是调用者），可以直接调用，无需额外 approve 步骤。

### 6d. 与 PR #82 pancakeswap 插件的关系

- 本插件 `pancakeswap-clmm` 不重新实现 `swap`、`quote`、`add-liquidity`、`remove-liquidity`
- 用户若需要先添加流动性再质押，应先使用 `pancakeswap add-liquidity`，获得 token ID 后再使用本插件 `pancakeswap-clmm farm`
- SKILL.md 中应说明两个插件的分工关系

### 6e. CAKE Token 地址

| 链 | CAKE 合约地址 |
|----|--------------|
| BSC (56) | `0x0E09FaBB73Bd3Ade0a17ECC321fD13a19e81cE82` |
| Ethereum (1) | `0x152649eA73beAb28c5b49B26eb48f7EAD6d4c898` |
| Base (8453) | 待确认（CAKE 在 Base 的 bridged 版本） |
| Arbitrum (42161) | 待确认（CAKE 在 Arbitrum 的 bridged 版本） |

> 在 harvest/unfarm 后通过 `onchainos wallet balance` 验证 CAKE 余额增加，避免依赖 CAKE 地址进行核心逻辑。
