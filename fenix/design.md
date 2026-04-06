# Fenix Finance — Plugin Store 接入 PRD

> 通过 onchainos CLI 接入 Fenix Finance，使 AI Agent 能完成该 DApp 的核心链上操作

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `fenix` |
| dapp_name | Fenix Finance V3 |
| dapp_repo | https://github.com/fenixfinance |
| dapp_alias | Fenix, Fenix DEX, Fenix Finance |
| one_liner | Algebra V3-based concentrated liquidity MetaDEX on Blast with Liquidity Hub aggregation and ve(3,3) incentive flywheel |
| category | defi-protocol |
| tags | DEX, EVM, Blast, concentrated-liquidity, swap, Algebra, CLMM, ve33 |
| target_chains | EVM (Blast/81457) |
| target_protocols | Fenix Finance |

---

## 1. Background

### 这个 DApp 是什么

Fenix Finance is a MetaDEX on Blast (L2) that combines a concentrated liquidity AMM (powered by Algebra Integral V4), classic V2-style pools, and a ve(3,3) voting/incentive system inspired by Uniswap V3 + Curve + Convex. It offers an optional Liquidity Hub (Orbs L3 technology) that aggregates on-chain and off-chain market maker quotes for optimal pricing. Target users are traders seeking deep spot liquidity on Blast and LPs seeking emissions via weekly veFNX voting.

### 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | No |
| SDK 支持哪些技术栈？ | TypeScript (no official SDK for Rust) |
| 有 REST API？ | No REST API; GraphQL subgraph via Goldsky for pool data |
| 有官方 Skill？ | No |
| 开源社区有类似 Skill？ | No |
| 支持哪些链？ | Blast (81457) only |
| 是否需要 onchainos 广播？ | Yes |

### 接入路径判定

接入路径：**API (direct contract calls via onchainos `wallet contract-call` + GraphQL subgraph for read-only pool data)**

- 链下查询：GraphQL subgraph (Goldsky) for pool list; `eth_call` via RPC for quotes (QuoterV2), allowance, token metadata
- 链上写操作：`onchainos wallet contract-call` for ERC-20 approve → SwapRouter `exactInputSingle` (swap); NFPM `mint` (add-liquidity)
- 参考项目结构：https://github.com/ganlinux/plugin-store/tree/main/official/hyperliquid

---

## 2. DApp 核心能力 & 接口映射

### 需要接入的操作

| # | 操作 | 说明 | 链上/链下 |
|---|------|------|-----------|
| 1 | get-quote | 查询两种代币的兑换报价（预估输出量、价格影响） | 链下（QuoterV2 eth_call） |
| 2 | swap | ERC-20 授权 + 单跳精确输入兑换 | 链上（approve + exactInputSingle） |
| 3 | get-pools | 获取 V3 流动性池列表（TVL、APR、fee tier） | 链下（GraphQL subgraph） |
| 4 | add-liquidity | 向 V3 集中流动性池注入流动性（mint NFT position） | 链上（approve × 2 + NFPM mint） |

### 链下查询（SDK / API 直接调用）

| 操作 | API Endpoint / 方法 | 关键参数 | 返回值 |
|------|---------------------|---------|--------|
| get-quote | `eth_call` QuoterV2 `quoteExactInputSingle` | tokenIn, tokenOut, amountIn, limitSqrtPrice=0 | amountOut (uint256) |
| get-pools | GraphQL POST `https://api.goldsky.com/api/public/project_clxadvm41bujy01ui2qalezdn/subgraphs/fenix-finance-v3/latest/gn` | query pools with token0, token1, totalValueLockedUSD, feeTier | pool array |
| check-allowance | `eth_call` ERC-20 `allowance(owner, spender)` | token address, wallet address, router address | allowance (uint128) |
| get-pool-address | `eth_call` Factory `poolByPair(token0, token1)` | token0 address, token1 address | pool address |
| token-metadata | `eth_call` ERC-20 `symbol()`, `decimals()` | token address | symbol string, decimals uint8 |

### 链上写操作（必须走 onchainos CLI）

> 硬性要求：所有链上交易（签名、广播、合约调用）必须通过 onchainos 执行。
> `onchainos dex approve`、`onchainos tx send` 命令**不存在**，所有链上操作统一通过 `wallet contract-call`。

**EVM 链上操作（每列必须填写，Researcher 验证后才能交给 Developer）：**

| 操作 | 合约地址（来源） | 函数签名（canonical 格式） | Selector（pycryptodome keccak256 ✅） | ABI 参数顺序 |
|------|---------------|--------------------------|--------------------------------------|------------|
| ERC-20 授权（swap） | tokenIn 地址（用户输入，运行时解析） | `approve(address,uint256)` | `0x095ea7b3` | spender=SwapRouter, amount=uint256_max |
| Swap exactInputSingle | `0x2df37Cb897fdffc6B4b03d8252d85BE7C6dA9d00`（SwapRouter 固定） | `exactInputSingle((address,address,address,address,uint256,uint256,uint256,uint160))` | `0x1679c792` | tokenIn, tokenOut, deployer=0x0, recipient=wallet, deadline, amountIn, amountOutMinimum, limitSqrtPrice=0 |
| ERC-20 授权 token0（add-liq） | token0 地址（运行时解析） | `approve(address,uint256)` | `0x095ea7b3` | spender=NFPM, amount=uint256_max |
| ERC-20 授权 token1（add-liq） | token1 地址（运行时解析） | `approve(address,uint256)` | `0x095ea7b3` | spender=NFPM, amount=uint256_max |
| NFPM mint（add-liq） | `0x8881b3Fb762d1D50e6172f621F107E24299AA1Cd`（NFPM 固定） | `mint((address,address,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` | `0x9cc1a283` | token0, token1, tickLower, tickUpper, amount0Desired, amount1Desired, amount0Min, amount1Min, recipient=wallet, deadline |

#### Selector 验证方法说明

selectors 使用 `pycryptodome` (`Crypto.Hash.keccak`) 计算，采用标准 Keccak-256（非 NIST SHA3-256）。验证命令：

```python
from Crypto.Hash import keccak
k = keccak.new(digest_bits=256)
k.update(b"exactInputSingle((address,address,address,address,uint256,uint256,uint256,uint160))")
print("0x" + k.hexdigest()[:8])  # → 0x1679c792
```

#### Algebra V3 与 Uniswap V3 / Algebra V1 的关键差异

Fenix 使用 **Algebra Integral V4**（≠ Uniswap V3，≠ Algebra V1/Camelot）：

1. **单池无 fee tier 参数** — Factory 用 `poolByPair(token0, token1)` (selector `0xd9a641e1`) 查池地址，不需要 `fee` 参数
2. **ExactInputSingleParams 有 `deployer` 字段**（第3个参数）：
   ```
   struct ExactInputSingleParams {
       address tokenIn;       // [0]
       address tokenOut;      // [1]
       address deployer;      // [2] — 对 Fenix 自己的池传 address(0)
       address recipient;     // [3] — 必须是真实钱包地址，不能是 address(0)
       uint256 deadline;      // [4]
       uint256 amountIn;      // [5]
       uint256 amountOutMinimum; // [6]
       uint160 limitSqrtPrice;   // [7] — 传 0 表示不限制价格
   }
   ```
3. **QuoterV2 struct 4字段**（无 `deployer`）：
   ```
   struct QuoteExactInputSingleParams {
       address tokenIn;
       address tokenOut;
       uint256 amountIn;
       uint160 limitSqrtPrice;
   }
   ```
   Selector: `0x5e5e6e0f`

---

## 3. 用户场景

**场景 1：Swap — 兑换代币**
- 用户说：「用 100 USDB 换 WETH，最大滑点 0.5%」
- Agent 动作序列：
  1. [链下] 解析 token 符号 → 地址（USDB: `0x4300000000000000000000000000000000000003`, WETH: `0x4300000000000000000000000000000000000004`）
  2. [链下] 调用 QuoterV2 `quoteExactInputSingle` 获取预估输出量 amountOut
  3. [链下] 计算 amountOutMinimum = amountOut × (1 - 0.5%)
  4. [链下] `eth_call` ERC-20 `allowance(wallet, SwapRouter)` 检查当前授权
  5. [链上] 若授权不足：`onchainos wallet contract-call` → USDB `approve(SwapRouter, uint256_max)` (等待 3 秒)
  6. [链下] 获取钱包地址（`onchainos wallet balance --chain 81457` 解析 address 字段）
  7. [链上] `onchainos wallet contract-call` → SwapRouter `exactInputSingle(...)` with recipient=wallet_address
  8. 返回 txHash 和实际获得的 WETH 数量给用户

**场景 2：Get Quote — 查询报价**
- 用户说：「查询用 1 WETH 能换多少 USDB」
- Agent 动作序列：
  1. [链下] 解析 token 符号 → 地址
  2. [链下] 验证池存在：Factory `poolByPair(WETH, USDB)` 返回非零地址
  3. [链下] QuoterV2 `quoteExactInputSingle({tokenIn: WETH, tokenOut: USDB, amountIn: 1e18, limitSqrtPrice: 0})`
  4. 返回 amountOut（格式化为 USDB，18 decimals）及价格影响

**场景 3：Get Pools — 查询流动性池**
- 用户说：「列出 Fenix 上 TVL 最高的 10 个 V3 池」
- Agent 动作序列：
  1. [链下] GraphQL query to Goldsky subgraph: `{ pools(first: 10, orderBy: totalValueLockedUSD, orderDirection: desc) { id token0 { symbol } token1 { symbol } totalValueLockedUSD feesUSD volumeUSD } }`
  2. 格式化返回：池地址、token 对名称、TVL (USD)、24h 交易量

**场景 4：Add Liquidity — 注入集中流动性**
- 用户说：「向 WETH/USDB 池添加流动性，注入 0.1 WETH 和等值 USDB，价格区间 ±10%」
- Agent 动作序列：
  1. [链下] 解析 token 地址；获取当前价格（通过 QuoterV2 或 pool slot0）
  2. [链下] 计算 tickLower / tickUpper（根据 ±10% 价格区间换算 tick）
  3. [链下] 检查 WETH 对 NFPM 的授权；不足则授权
  4. [链上] `onchainos wallet contract-call` → WETH `approve(NFPM, uint256_max)` (等待 5 秒)
  5. [链下] 检查 USDB 对 NFPM 的授权；不足则授权
  6. [链上] `onchainos wallet contract-call` → USDB `approve(NFPM, uint256_max)` (等待 5 秒)
  7. [链下] 获取钱包地址
  8. [链上] `onchainos wallet contract-call` → NFPM `mint(...)` with recipient=wallet
  9. 返回 tokenId（NFT position ID）和实际注入的两种 token 数量

---

## 4. 外部 API 依赖

| API | Base URL | 用途 | 需要 API Key？ |
|-----|----------|------|---------------|
| Blast RPC | `https://rpc.blast.io` (or user's RPC) | eth_call for quotes, allowance, token metadata | No |
| Fenix GraphQL (Goldsky) V3 | `https://api.goldsky.com/api/public/project_clxadvm41bujy01ui2qalezdn/subgraphs/fenix-finance-v3/latest/gn` | get-pools: pool list with TVL, volume, fees | No |
| Fenix GraphQL (Goldsky) V2 | `https://api.goldsky.com/api/public/project_clxadvm41bujy01ui2qalezdn/subgraphs/fenix-finance-v2/latest/gn` | V2 pair data (optional) | No |

---

## 5. 配置参数

| Parameter | Default | Description |
|-----------|---------|-------------|
| default_chain | `81457` | Blast chain ID（唯一支持链） |
| rpc_url | `https://rpc.blast.io` | Blast RPC endpoint（可覆盖为用户自定义 RPC） |
| max_slippage | `0.5` | 最大滑点 (%)，用于计算 amountOutMinimum |
| swap_deadline_secs | `300` | 交易 deadline 偏移（秒，从当前时间加） |
| dry_run | `true` | 模拟模式，不发真实交易 |

---

## 6. Agent 执行指南

### Phase 1：需求分析（Researcher Agent）✅ 完成

1. 已确认 plugin_name=`fenix`，接入路径=API（直接合约调用）
2. 已 WebFetch Fenix 官方文档、Blastscan 合约代码、Algebra GitHub 源码
3. 已验证所有函数签名（pycryptodome Keccak-256）
4. 已完成 §2 接口映射表

### Phase 2：代码实现（Developer Agent）

1. 读取 Plugin Store 开发文档：https://github.com/okx/plugin-store-community/blob/main/PLUGIN_DEVELOPMENT_GUIDE_ZH.md
2. 读取 onchainos skills 和命令：https://github.com/okx/onchainos-skills/tree/main/skills
3. 在 `/Users/samsee/projects/plugin-store-dev/fenix/` 下创建 Rust 工程
4. 实现以下命令（Skill + Binary 类型）：
   - `fenix get-quote --token-in <addr> --token-out <addr> --amount-in <uint>` → eth_call QuoterV2
   - `fenix swap --token-in <addr> --token-out <addr> --amount-in <uint> --slippage <f64>` → approve + exactInputSingle
   - `fenix get-pools [--limit <n>]` → GraphQL subgraph query
   - `fenix add-liquidity --token0 <addr> --token1 <addr> --amount0 <uint> --amount1 <uint> --tick-lower <i32> --tick-upper <i32>` → approve×2 + NFPM mint
5. 关键实现注意事项：
   - **deployer 字段**：`exactInputSingle` 的第3个 struct 字段 `deployer` 对 Fenix 官方池传 `address(0)`（`0x0000000000000000000000000000000000000000`）
   - **recipient 不能是 address(0)**：通过 `onchainos wallet balance --chain 81457` 解析真实钱包地址后传入
   - **approve 前检查 allowance**：避免重复授权导致 nonce 冲突（参见 dex.md `#allowance-check`）
   - **approve → swap 间隔 3 秒**：防止 nonce 竞争
   - **approve → mint 间隔 5 秒**：LP 多步操作间隔更长（参见 dex.md `#lp-nonce-delay`）
   - **pool 存在性验证**：quote 前先调 Factory `poolByPair` 确认池已部署（参见 dex.md `#quoter-zero-liquidity`）
   - **token 地址解析**：支持符号映射（WETH→`0x4300...0004`, USDB→`0x4300...0003`, FNX→`0x52f8...192`）
   - **EVM chain_id 81457 不能用 `--output json`**：获取钱包地址时解析默认输出

6. 每个命令实际链上验证，确认 txHash 成功

### Phase 3：测试（Tester Agent）

基于 §3 用户场景执行测试用例：
- T1: `get-quote WETH USDB 1e18` → 返回合理 amountOut
- T2: `swap USDB WETH 100e18 --slippage 0.5` → txHash confirmed on Blastscan
- T3: `get-pools --limit 10` → 返回 10 个池，含 TVL 数据
- T4: `add-liquidity WETH USDB ...` → NFT tokenId returned

### Phase 4：提交 PR（Submitter Agent）

按开发文档步骤 6 提交 PR 至 okx/plugin-store-community。

---

## 7. Open Questions

- [ ] **deployer 字段取值**：Algebra V3 `exactInputSingle` 第3字段 `deployer`，对 Fenix Finance 官方池是否始终为 `address(0)`？需在 Blastscan 上解码一笔真实 swap tx 的 calldata 确认（初步研究认为是 `address(0)`）
- [ ] **QuoterV2 是否包含 deployer**：Blastscan 源码显示 4 字段（无 deployer），已使用此版本（selector `0x5e5e6e0f`）；若链上调用失败需尝试 5 字段版本（selector `0xe94764c4`）
- [ ] **USDB decimals**：USDB (Blast 原生稳定币) 是否为 18 decimals？需用 `eth_call` `decimals()` 确认，代码中不硬编码
- [ ] **Blast chain wallet balance 命令**：chain 81457 的 `onchainos wallet balance` 是否返回 JSON（不加 `--output json`）？根据 memory 记录 chain 501 (Solana) 是这样；EVM 链需 Developer 实测
- [ ] **GraphQL subgraph V2 slug 名称**：V2 subgraph URL 中的 `fenix-finance-v2` 未经官方文档确认，Developer 需测试实际可用性
- [ ] **Blast native ETH wrapping**：用户若传入原生 ETH（非 WETH），swap 是否需要先 wrap？SwapRouter 是否支持 `msg.value` payable 路径？

---

## 附录：合约地址速查

| 合约 | 地址 | 备注 |
|------|------|------|
| SwapRouter (Algebra V3) | `0x2df37Cb897fdffc6B4b03d8252d85BE7C6dA9d00` | exactInputSingle 入口 |
| QuoterV2 | `0x94Ca5B835186A37A99776780BF976fAB81D84ED8` | 价格模拟，只读 |
| Algebra Factory | `0x7a44CD060afC1B6F4c80A2B9b37f4473E74E25Df` | poolByPair 查池地址 |
| NFT Position Manager | `0x8881b3Fb762d1D50e6172f621F107E24299AA1Cd` | 集中流动性 LP mint |
| RouterV2 (classic AMM) | `0xbD571125856975DBfC2E9b6d1DE496D614D7BAEE` | V2 pair 入口（本期不接入） |
| Pair Factory V2 | `0xa19C51D91891D3DF7C13Ed22a2f89d328A82950f` | V2 pair 工厂（本期不接入） |
| FNX Token | `0x52f847356b38720B55ee18Cb3e094ca11C85A192` | 18 decimals |
| veFNX | `0xC900C984a3581EfA4Fb56cAF6eF19721aAFbB4f9` | vote-escrow，本期不接入 |
| WETH (Blast) | `0x4300000000000000000000000000000000000004` | 18 decimals |
| USDB (Blast) | `0x4300000000000000000000000000000000000003` | Blast 原生稳定币，decimals 待确认 |

## 附录：函数选择器速查（pycryptodome Keccak-256 验证）

| 操作 | 选择器 | 规范函数签名 |
|------|--------|-------------|
| ERC-20 approve | `0x095ea7b3` | `approve(address,uint256)` |
| ERC-20 allowance | `0xdd62ed3e` | `allowance(address,address)` |
| ERC-20 balanceOf | `0x70a08231` | `balanceOf(address)` |
| ERC-20 decimals | `0x313ce567` | `decimals()` |
| ERC-20 symbol | `0x95d89b41` | `symbol()` |
| SwapRouter exactInputSingle | `0x1679c792` | `exactInputSingle((address,address,address,address,uint256,uint256,uint256,uint160))` |
| SwapRouter exactInput (multi-hop) | `0xc04b8d59` | `exactInput((bytes,address,uint256,uint256,uint256))` |
| QuoterV2 quoteExactInputSingle | `0x5e5e6e0f` | `quoteExactInputSingle((address,address,uint256,uint160))` |
| Factory poolByPair | `0xd9a641e1` | `poolByPair(address,address)` |
| NFPM mint | `0x9cc1a283` | `mint((address,address,int24,int24,uint256,uint256,uint256,uint256,address,uint256))` |
| NFPM collect | `0xfc6f7865` | `collect((uint256,address,uint128,uint128))` |
| NFPM decreaseLiquidity | `0x0c49ccbe` | `decreaseLiquidity((uint256,uint128,uint256,uint256,uint256))` |
| NFPM positions | `0x99fbab88` | `positions(uint256)` |
