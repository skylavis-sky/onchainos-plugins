# Orca DEX (Whirlpools) — Plugin Store 接入 PRD

> 通过 onchainos CLI 接入 Orca Whirlpools CLMM，使 AI Agent 能在 Solana 上执行代币兑换、查询流动性池和获取报价

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `orca` |
| dapp_name | Orca DEX (Whirlpools) |
| dapp_repo | https://github.com/orca-so/whirlpools |
| dapp_alias | orca, orca-whirlpools, whirlpools, orca-dex |
| one_liner | Concentrated liquidity AMM on Solana — swap tokens and query pools via the Whirlpools CLMM program |
| category | trading-strategy |
| tags | dex, swap, clmm, concentrated-liquidity, solana, amm |
| target_chains | solana (chain ID 501) |
| target_protocols | Orca Whirlpools |

---

## 1. Background

### 这个 DApp 是什么

Orca 是 Solana 上最大的原生去中心化交易所之一，其核心产品 Whirlpools 是一个集中流动性自动做市商（CLMM）。用户可在指定价格区间提供流动性，获得交易手续费收益；同时任何用户都可以通过 Whirlpool Program 以极低滑点进行代币兑换。Orca 支持 Solana 主网和 Eclipse 网络，TVL 超 $1B，日均交易量数千万美元。

### 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **Yes** — `orca_whirlpools` crate (v7.0.2)，https://docs.rs/orca_whirlpools / https://github.com/orca-so/whirlpools/tree/main/rust-sdk |
| SDK 支持哪些技术栈？ | **Rust**（主要，High/Core/Client 三层）、**TypeScript**（@orca-so/whirlpools）、**Wasm**（编译自 Rust Core） |
| 有 REST API？ | **Yes（只读）** — https://api.orca.so/docs — 提供池子列表、Token 信息、协议统计；无 swap 路由/报价端点 |
| 有官方 Skill？ | No |
| 开源社区有类似 Skill？ | **Yes** — https://github.com/demcp/demcp-orca-mcp（TypeScript MCP，8 个 tools：configure_orca、get_balance、swap_tokens、get_swap_quote、compare_pools、get_pools、add_liquidity、list_common_tokens） |
| 支持哪些链？ | Solana Mainnet、Solana Devnet、Eclipse Mainnet、Eclipse Testnet — **本插件只接入 Solana Mainnet（chain 501）** |
| 是否需要 onchainos 广播？ | **Yes** — swap 是链上写操作，需要通过 `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>` 或 `onchainos dex swap execute` 广播 |

### 接入路径判定

```
有参考 skills github? → Yes (demcp/demcp-orca-mcp)
  但其为 TypeScript；本插件用 Rust SDK 实现（符合 plugin-store 要求）
  
接入路径：SDK（Rust SDK 主路径）+ 参考社区 Skill 实现逻辑
```

**接入路径：SDK（Rust `orca_whirlpools` crate）**

核心 SDK 层次：
- **High-Level** (`orca_whirlpools`): `swap_instructions()`, `fetch_whirlpools_by_token_pair()`, `fetch_concentrated_liquidity_pool()` — 主要使用层
- **Core** (`orca_whirlpools_core`): `swap_quote_by_input_token()`, `swap_quote_by_output_token()` — 报价计算
- **Client** (`orca_whirlpools_client`): 由 IDL 自动生成的低级指令，仅在需要自定义时使用

Whirlpool Program 地址（主网与 Devnet 相同）：`whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc`

---

## 2. DApp 核心能力 & 接口映射

### 需要接入的操作

| # | 操作 | 说明 | 链上/链下 |
|---|------|------|-----------|
| 1 | `get-pools` | 查询两个 Token 之间所有 Whirlpool 池（已初始化/未初始化），返回价格、流动性、手续费率 | 链下 |
| 2 | `get-quote` | 计算 swap 报价（精确输入或精确输出），返回预期输出数量、最小输出（含滑点）、价格影响 | 链下 |
| 3 | `swap` | 执行代币兑换，构建 Solana 交易并通过 onchainos 广播 | 链上 |

### 链下查询（SDK / API 直接调用）

| 操作 | SDK 方法 / API Endpoint | 关键参数 | 返回值 |
|------|------------------------|---------|--------|
| get-pools（SDK） | `orca_whirlpools::fetch_whirlpools_by_token_pair(rpc, token_1_pubkey, token_2_pubkey)` | `rpc: &RpcClient`, `token_1: Pubkey`, `token_2: Pubkey` | `Vec<PoolInfo>` — 每个含 `PoolInfo::Initialized { address, data, price }` 或 `PoolInfo::Uninitialized { config }` |
| get-pools（API 补充） | `GET https://api.orca.so/pools/search?q=<token_symbol>&minTvl=<n>&stats=true` | `q`: token symbol 或 address；`minTvl`: 最小 TVL 过滤 | `PublicWhirlpool[]`：含 tokenA/B 信息、TVL、24h 交易量、当前价格 |
| get-quote | `orca_whirlpools_core::swap_quote_by_input_token(token_in, specified_token_a, slippage_bps, whirlpool, oracle, tick_arrays, timestamp, transfer_fee_a, transfer_fee_b)` | `token_in: u64`（最小单位），`specified_token_a: bool`，`slippage_tolerance_bps: u16`（如 50 = 0.5%）| `ExactInSwapQuote { token_in, token_estimated_out, token_min_out, trade_fee }` |
| get-quote（精确输出） | `orca_whirlpools_core::swap_quote_by_output_token(token_out, specified_token_a, slippage_bps, whirlpool, oracle, tick_arrays, timestamp, ...)` | `token_out: u64`（目标输出量） | `ExactOutSwapQuote { token_max_in, token_estimated_in, token_out }` |
| token-info（辅助） | `GET https://api.orca.so/tokens/{mint_address}` 或 `onchainos token search --query <symbol> --chain 501` | `mint_address`: SPL Token mint 地址 | `PublicToken { symbol, name, decimals, price }` |
| pool-detail（辅助） | `GET https://api.orca.so/pools/{pool_address}` | Whirlpool 账户地址（base58） | 含 TVL、volume 24h、fee tier、tick spacing |

> **注意：** `get-quote` 使用 Core SDK 的纯数学函数（无 RPC 调用），需先通过 `fetch_concentrated_liquidity_pool` + Solana RPC 获取 whirlpool 账户数据和 tick arrays，然后传入 Core SDK 计算报价。High-Level SDK 的 `swap_instructions()` 已封装了这个完整流程（内部调用报价 + 组装指令）。

### 链上写操作（必须走 onchainos CLI）

> Solana 没有 calldata。链上写操作通过两种路径之一执行：
> 1. **优先路径**：`onchainos dex swap execute`（onchainos 自动处理 Orca 路由、签名、广播）
> 2. **备用路径**：Rust SDK 构建未签名交易 → base64 序列化 → `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>`

| 操作 | onchainos 命令 | 关键参数 | 说明 |
|------|---------------|---------|------|
| swap（优先路径） | `onchainos dex swap execute` | `--chain 501 --from <FROM_MINT> --to <TO_MINT> --amount <UI_AMOUNT> --slippage <PCT>` | onchainos 内置支持 Solana DEX，自动路由（可能走 Orca）；无需 `--force`；CLI 自动处理签名和广播 |
| swap（SDK 路径） | `onchainos wallet contract-call` | `--chain 501 --to whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc --unsigned-tx <BASE64_SERIALIZED_TX>` | Rust SDK `swap_instructions()` 返回 `SwapInstructions { instructions, quote, additional_signers }`；将指令组装为 `solana_sdk::transaction::Transaction`，序列化为 base64 后传入；blockhash 60s 过期，必须立即提交 |

#### swap_instructions() 构建交易流程（SDK 路径详解）

```rust
// 1. 初始化 RPC client，设置 whirlpool config（主网：FcrweFY1G9HJAHG5inkGB6pKg1HZ6x9UC2WioAfWrGkR）
let rpc = RpcClient::new(solana_rpc_url);
set_whirlpools_config_address(WhirlpoolsConfigInput::SolanaMainnet)?;

// 2. 调用 High-Level SDK — 内部自动：
//    a. fetch_concentrated_liquidity_pool() → 获取 whirlpool 账户数据
//    b. 加载 5 个 tick arrays（当前 ±2 位置）
//    c. 获取 oracle 账户（adaptive-fee 池需要）
//    d. 计算 swap_quote_by_input_token() / swap_quote_by_output_token()
//    e. prepare_token_accounts_instructions() → 创建/验证 ATA
//    f. 组装 SwapV2 指令（含 token programs、vaults、tick arrays、oracle）
let swap_ix = swap_instructions(
    &rpc,
    whirlpool_address,  // pool 的 Pubkey
    amount,             // u64，最小单位
    specified_mint,     // 输入 token 的 mint Pubkey
    SwapType::ExactIn,
    Some(slippage_bps), // e.g. 50 for 0.5%
    Some(wallet_pubkey),
).await?;

// 3. 组装 Transaction（注意：必须获取最新 blockhash）
let recent_blockhash = rpc.get_latest_blockhash().await?;
let mut tx = Transaction::new_with_payer(&swap_ix.instructions, Some(&wallet_pubkey));
tx.message.recent_blockhash = recent_blockhash;
// swap_ix.additional_signers 包含 wSOL wrap/unwrap 所需的临时 keypair

// 4. 序列化为 base64（未签名）
let serialized = base64::encode(tx.message_data());

// 5. 通过 onchainos 提交（onchainos 内部用钱包私钥签名并广播）
// onchainos wallet contract-call \
//   --chain 501 \
//   --to whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc \
//   --unsigned-tx <serialized>
```

**SwapInstructions 结构：**
- `instructions: Vec<Instruction>` — 完整指令序列（含 ATA 创建、swap、wSOL wrap/unwrap）
- `quote: SwapQuote` — 包含预期输出、最小输出（含滑点保护）、手续费
- `additional_signers: Vec<Keypair>` — wrapped SOL 操作所需的临时 keypair
- `trade_enable_timestamp: u64` — 自适应费率池的交易开放时间戳

> **⚠️ Solana 特殊注意事项：**
> - Solana blockhash 约 60 秒过期，从获取 blockhash 到 onchainos 广播必须在 60s 内完成
> - 没有 ERC-20 approve 概念；SPL Token 授权通过 ATA（Associated Token Account）机制处理，SDK 自动处理
> - native SOL 地址用 `11111111111111111111111111111111`（系统程序），wSOL 用 `So11111111111111111111111111111111111111112`
> - market price 查询 SOL 价格时，用 wSOL mint 地址，不用 native SOL 地址

---

## 3. 用户场景

**场景 1：查询代币兑换报价（核心查询路径）**

- 用户说：「我想用 10 USDC 换 SOL，能换多少？」

- Agent 动作序列：
  1. **[链下查询]** `onchainos token search --query USDC --chain 501` 解析 USDC mint 地址 → `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`（decimals=6）
  2. **[链下查询]** wSOL mint 为已知常量 `So11111111111111111111111111111111111111112`（decimals=9）
  3. **[链下查询]** 调用 `fetch_whirlpools_by_token_pair(rpc, USDC_mint, wSOL_mint)` 获取所有已初始化的 USDC/SOL 池（通常有多个 tick spacing 的池）
  4. **[链下查询]** 对流动性最高的池调用 `swap_quote_by_input_token(10_000_000, false, 50, whirlpool_data, oracle, tick_arrays, timestamp, None, None)`（USDC=tokenB，false 表示不是 tokenA）
  5. **[返回用户]** 展示：预期获得 X SOL、最少获得 Y SOL（0.5% 滑点保护）、手续费 Z USDC、价格影响 P%

**场景 2：执行代币 Swap（核心 Happy Path）**

- 用户说：「帮我用 0.5 SOL 换 USDC，滑点 1%」

- Agent 动作序列：
  1. **[链下查询]** `onchainos wallet balance --chain 501` 确认 SOL 余额 ≥ 0.5 + 预估 gas（至少 ~0.001 SOL 用于 tx fee）
  2. **[安全检查]** `onchainos security token-scan --address EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v --chain 501` 确认 USDC 无风险
  3. **[链下查询]** `fetch_whirlpools_by_token_pair(rpc, wSOL_mint, USDC_mint)` 获取最佳流动性池
  4. **[链下查询]** 计算报价：`swap_quote_by_input_token(500_000_000, true, 100, ...)` （0.5 SOL = 500_000_000 lamports，wSOL 为 tokenA，slippage 100 bps = 1%）→ 展示预期 USDC 输出和价格影响
  5. **[请求用户确认]** 展示报价（预期输出、最少输出、手续费）并请求确认
  6. **[链上操作]** 用户确认后，优先使用：
     ```bash
     onchainos dex swap execute \
       --chain 501 \
       --from So11111111111111111111111111111111111111112 \
       --to EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
       --amount 0.5 \
       --slippage 1
     ```
     若 `dex execute` 不支持 Orca，则回退到 SDK 路径：`swap_instructions()` 构建交易 → base64 → `onchainos wallet contract-call --chain 501 --unsigned-tx <base64>`
  7. **[返回结果]** 展示交易签名（base58 tx hash），链接 Solscan

**场景 3：查询可用流动性池（分析场景）**

- 用户说：「Orca 上 ORCA/USDC 有哪些池，TVL 和收益率分别是多少？」

- Agent 动作序列：
  1. **[链下查询]** `GET https://api.orca.so/pools/search?q=ORCA&stats=true&minTvl=10000` 获取所有含 ORCA Token 的池，过滤包含 USDC 的池
  2. **[链下查询]** 对返回的每个池地址调用 `GET https://api.orca.so/pools/{address}` 获取详细统计（TVL、24h volume、7d fees yield）
  3. **[链下查询]** `onchainos market price --address orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE --chain 501` 获取实时 ORCA 价格
  4. **[返回用户]** 格式化展示：每个池的手续费率（tick spacing）、TVL、24h 交易量、年化费用收益率（APR = 24h fees / TVL × 365 × 100%），建议流动性最好的池

**场景 4：获取 Token 在 Orca 上的流动性（风控场景）**

- 用户说：「这个 Token XXX 在 Orca 上流动性够吗？」（用户提供 mint 地址）

- Agent 动作序列：
  1. **[安全检查]** `onchainos security token-scan --address <mint_addr> --chain 501` — 若返回 `block`，立即拒绝并告知用户风险；`warn` 则展示警告后继续
  2. **[链下查询]** `GET https://api.orca.so/pools/search?q=<mint_addr>&stats=true` 列出所有含该 Token 的 Orca 池
  3. **[链下查询]** `onchainos token liquidity --address <mint_addr> --chain 501` 获取 top 5 流动性池总览
  4. **[链下查询]** 对 TVL 前 3 的池计算 $1000 兑换的预估滑点（`swap_quote_by_input_token` 以小额模拟）
  5. **[返回用户]** 展示：最大可用流动性、建议最大单笔交易量（TVL 的 1-2% 以控制滑点 < 1%）；若总 TVL < $10K，警告高滑点风险

---

## 4. 外部 API 依赖

| API | Base URL | 用途 | 需要 API Key？ |
|-----|----------|------|---------------|
| Orca REST API | `https://api.orca.so` | 池子列表搜索、池子统计信息（TVL、volume、APR）、Token 信息 | No（公开 API） |
| Solana RPC | 用户配置（见 §5） | 读取链上账户数据（Whirlpool、tick arrays、oracle、token mints）；High-Level SDK 所有链下操作均需 | No（公开节点）/ Yes（付费节点） |
| onchainos dex swap execute | onchainos CLI | Solana DEX swap 广播（优先路径） | 需要 onchainos 已认证 |
| onchainos wallet contract-call | onchainos CLI | 提交未签名 Solana 交易（SDK 路径） | 需要 onchainos 已认证 |

> **Solana RPC 节点建议（主网）：**
> - 公共节点：`https://api.mainnet-beta.solana.com`（有速率限制，仅低频使用）
> - 推荐：`https://solana-mainnet.rpc.extrnode.com`、`https://mainnet.helius-rpc.com` 或用户自有 RPC（如 Quicknode、Alchemy Solana）
> - 注意：DEX 操作需要较多 RPC 调用（fetch pool + tick arrays + oracle + ATA），高频使用需付费 RPC

---

## 5. 配置参数

| Parameter | Default | Description |
|-----------|---------|-------------|
| `solana_rpc_url` | `https://api.mainnet-beta.solana.com` | Solana 主网 RPC 端点；高频使用建议替换为付费节点 |
| `default_slippage_bps` | `50` | 默认滑点容忍度（basis points，50 = 0.5%） |
| `min_pool_tvl_usd` | `10000` | 查询池时过滤的最小 TVL（美元），低于此值视为流动性不足 |
| `price_impact_warn_threshold` | `2.0` | 价格影响（%）超过此值时向用户展示警告 |
| `price_impact_block_threshold` | `10.0` | 价格影响（%）超过此值时阻止交易执行 |
| `dry_run` | `true` | 模拟模式：true 时只计算报价、不发送链上交易 |
| `chain_id` | `501` | 目标链（固定为 Solana 主网，不允许用户修改） |

---

## 6. 已知常量与地址

> **重要：地址在运行时验证，不应硬编码在合约调用逻辑中，仅作为参考常量。**

| 常量 | 值 | 说明 |
|------|-----|------|
| Whirlpool Program | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | Solana 主网 & Devnet（Verifiable Build 已验证） |
| WhirlpoolsConfig (主网) | `FcrweFY1G9HJAHG5inkGB6pKg1HZ6x9UC2WioAfWrGkR` | SDK 初始化时用 `WhirlpoolsConfigInput::SolanaMainnet` |
| wSOL Mint | `So11111111111111111111111111111111111111112` | Wrapped SOL（SPL Token），用于 SOL swap |
| Native SOL | `11111111111111111111111111111111` | 系统程序地址（仅用于余额查询，price 查询用 wSOL） |
| USDC Mint | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | Circle USDC on Solana |
| USDT Mint | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` | Tether USDT on Solana |
| ORCA Token | `orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE` | ORCA governance token |

---

## 7. Agent 执行指南

### Phase 1：需求分析（Researcher Agent）— 已完成

### Phase 2：代码实现（Developer Agent）

1. 读取 Plugin Store 开发文档（https://github.com/okx/plugin-store-community/blob/main/PLUGIN_DEVELOPMENT_GUIDE_ZH.md）
2. 在 `~/projects/plugin-store-dev/orca/` 创建 Rust 工程（Skill + Binary 类型）
3. `Cargo.toml` 添加：`orca_whirlpools = "7"`, `orca_whirlpools_core`, `solana-sdk`, `solana-client`, `tokio`, `serde_json`, `base64`, `reqwest`
4. 实现三个命令：`get-pools`、`get-quote`、`swap`
   - `get-pools`：调用 `fetch_whirlpools_by_token_pair` + Orca REST API 补充统计信息
   - `get-quote`：调用 `swap_instructions()` 获取内含 quote，或单独调用 `swap_quote_by_input_token/output_token`（需先 fetch pool + tick arrays）
   - `swap`：优先调用 `onchainos dex swap execute`；若失败则 `swap_instructions()` 构建交易 → base64 → `wallet contract-call --unsigned-tx`
5. 在 `swap` 命令中强制执行风控：price impact > 10% 时拒绝，2-10% 时警告
6. 实现 `dry_run` 模式：只计算报价、不发送 onchainos 命令
7. 安全：swap 前调用 `onchainos security token-scan` 检查 output token

### Phase 3：测试（Tester Agent）

基于 §3 场景执行 L1（编译）→ L2（mock 链下）→ L3（dry-run 链上）→ L4（真实链上）四级测试。

注意：Solana 测试钱包需有少量 SOL（≥ 0.01 SOL）用于 tx fee 和 ATA 创建费用。

---

## 8. Open Questions

- [ ] `onchainos dex swap execute --chain 501` 是否原生支持 Orca 路由？需在 Phase 3 验证；若不支持则回退 SDK 路径
- [ ] `onchainos wallet contract-call --unsigned-tx` 中，`additional_signers`（wSOL wrap keypair）如何传递？CLI 是否自动处理？若不处理，需在 SDK 路径中预签名这些 keypair
- [ ] Orca API `api.orca.so` 是否有速率限制或需要 API Key？文档显示公开，但需实测
- [ ] Eclipse 网络（chain ID 待确认）是否需要接入？本 PRD 仅涵盖 Solana 主网
