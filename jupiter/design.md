# Jupiter — Plugin Store 接入 PRD

> 通过 onchainos CLI 接入 Jupiter，使 AI Agent 能在 Solana 上完成代币兑换、报价查询和价格查询等核心操作。

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `jupiter` |
| dapp_name | Jupiter |
| dapp_repo | https://github.com/jup-ag/jup-lock |
| dapp_alias | `jup, jupiter aggregator, jup.ag` |
| one_liner | Solana DEX aggregator — swap any SPL token at best price via multi-router aggregation |
| category | defi-protocol |
| tags | `swap, dex-aggregator, solana, routing, spl-token` |
| target_chains | Solana (chain 501) |
| target_protocols | Jupiter Swap API v2 |

---

## §1 Background

### 这个 DApp 是什么

Jupiter is the leading DEX aggregator on Solana with $1B+ daily swap volume. It routes trades across all major Solana DEXes (Raydium, Orca, Meteora, etc.) via four routing engines (Metis, JupiterZ, Dflow, OKX) to find the best price for any SPL token pair. The platform is non-custodial and requires no on-chain account registration.

### 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | No — official SDK is TypeScript only (`@jup-ag/api`). No official Rust SDK. |
| SDK 支持哪些技术栈？ | TypeScript / JavaScript (`@jup-ag/api`, `@jup-ag/cli`) |
| 有 REST API？ | **Yes** — `https://api.jup.ag` — fully documented, no auth required for basic access (0.5 RPS keyless; API key for higher limits). Quote + Swap + Price + Tokens endpoints. Docs: https://developers.jup.ag/docs |
| 有官方 Skill？ | Yes — Jupiter CLI (`npm i -g @jup-ag/cli`) and Agent Skills (`npx skills add`) exist, but are JS-based. Not usable in Rust plugin context. |
| 开源社区有类似 Skill？ | Raydium Solana plugin already in plugin-store (use as structural reference for Solana `--unsigned-tx` flow) |
| 支持哪些链？ | Solana only (chain 501 in onchainos) |
| 是否需要 onchainos 广播？ | **Yes** — swap returns a serialized unsigned transaction (base64); must convert to base58 and broadcast via `onchainos wallet contract-call --unsigned-tx` |

### 接入路径判定

```
有参考 skills github 链接？ → No (no Rust Skill)
  → 有 Rust SDK？ → No
  → 有其他语言 SDK？ → Yes (TypeScript), but prefer REST API for Rust
  → 仅有 API？ → Yes, clean REST JSON API
```

接入路径：**API** — 直接调用 `https://api.jup.ag` REST endpoints，用 Rust `reqwest` 调用，`base64` + `bs58` crate 处理 tx 编码转换。

参考项目结构：https://github.com/ganlinux/plugin-store/tree/main/official/hyperliquid (general structure) + Raydium plugin (Solana --unsigned-tx pattern).

---

## §2 DApp 核心能力 & 接口映射

### 需要接入的操作

| # | 操作 | 说明 | 链上/链下 |
|---|------|------|-----------|
| 1 | `get-quote` | 获取代币兑换报价（输入→输出数量估算，含手续费、滑点、路由路径） | 链下查询 |
| 2 | `swap` | 执行代币兑换（构建 tx → base64→base58 → onchainos 广播） | **链上** |
| 3 | `get-price` | 查询单个或多个代币的实时 USD 价格 | 链下查询 |
| 4 | `get-tokens` | 按名称/符号/mint 地址搜索代币，获取 mint 地址 | 链下查询 |

---

### 链下查询（API 直接调用）

| 操作 | API Endpoint | 关键参数 | 返回值 |
|------|-------------|---------|--------|
| `get-quote` | `GET https://api.jup.ag/swap/v2/order` | `inputMint`, `outputMint`, `amount` (lamports/atomic units), `taker` (wallet pubkey), `slippageBps` (optional) | `outAmount` (raw), `otherAmountThreshold`, `priceImpactPct`, `routePlan`, `transaction` (base64 unsigned tx), `requestId` |
| `get-price` | `GET https://api.jup.ag/price/v3?ids=<mint1>,<mint2>` | `ids` = comma-separated mint addresses (max 50) | Per mint: `usdPrice`, `decimals`, `priceChange24h`, `blockId` |
| `get-tokens` | `GET https://api.jup.ag/tokens/v2/search?query=<symbol_or_name>` | `query` (symbol, name, or mint address) | Array of token objects: `name`, `symbol`, `address` (mint), `decimals`, `verified`, `organicScore` |

> **Amount unit note for get-quote**: `amount` must be in **raw atomic units** (lamports for SOL, no decimals).
> Example: 0.1 SOL = `100000000` (0.1 × 10^9 lamports).
> Example: 1 USDC = `1000000` (1 × 10^6, USDC has 6 decimals).

---

### 链上写操作（必须走 onchainos CLI）

**Solana 链上操作：**

| 操作 | Program ID | API Endpoint | 请求 Body 关键字段 | amount 单位 | tx 编码 |
|------|-----------|-------------|-----------------|-----------|--------|
| `swap` | `JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4` | `GET https://api.jup.ag/swap/v2/order` | `inputMint`, `outputMint`, `amount` (raw atomic units), `taker` (wallet pubkey), `slippageBps` | **Raw atomic units** — lamports for SOL (10^9), USDC × 10^6, etc. | `transaction` field in response = **base64** → **must convert to base58** before `--unsigned-tx` |

**onchainos 命令（swap）：**

```bash
# Step 1: Get wallet address (Solana — no --output json)
onchainos wallet balance --chain 501
# → parse json["data"]["details"][0]["tokenAssets"][0]["address"]

# Step 2: GET /swap/v2/order → extract base64 `transaction` field

# Step 3: Convert base64 → bytes → base58 (in Rust code)
# Cargo.toml: base64 = "0.22", bs58 = "0.5"
# let bytes = base64::engine::general_purpose::STANDARD.decode(&base64_tx)?;
# let base58_tx = bs58::encode(bytes).into_string();

# Step 4: Broadcast unsigned tx
onchainos wallet contract-call \
  --unsigned-tx <base58_tx> \
  --to JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 \
  --chain 501 \
  --force
```

> **Critical encoding note**: Jupiter API v2 (`/swap/v2/order`) returns the transaction in the `transaction` field as **base64**. The `onchainos wallet contract-call --unsigned-tx` flag expects **base58**. Code MUST convert: `base64 decode → raw bytes → base58 encode`. Cargo.toml must include `base64 = "0.22"` and `bs58 = "0.5"`.

> **No separate execute step needed**: With `--unsigned-tx`, onchainos handles signing and broadcasting internally. Do NOT call `/swap/v2/execute` — that endpoint expects a pre-signed transaction from a client-side wallet, which conflicts with the onchainos signing model.

---

## §3 用户场景

**场景 1：查询 0.1 SOL → USDC 兑换报价**

- 用户说：「Quote me a swap of 0.1 SOL to USDC on Solana」
- Agent 动作序列：
  1. [链下查询] 确认 SOL mint = `So11111111111111111111111111111111111111112`, USDC mint = `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
  2. [链下查询] 获取钱包地址：`onchainos wallet balance --chain 501` → 解析 `json["data"]["details"][0]["tokenAssets"][0]["address"]`
  3. [链下查询] `GET https://api.jup.ag/swap/v2/order?inputMint=So11...112&outputMint=EPjF...1v&amount=100000000&taker=<wallet_pubkey>`
  4. 解析响应：`outAmount`（转换为 USDC UI 单位除以 10^6）、`priceImpactPct`、路由引擎
  5. 返回给用户：预计输出 USDC 数量、价格影响、最优路由

**场景 2：执行 0.01 SOL → USDC 兑换**

- 用户说：「Swap 0.01 SOL for USDC using Jupiter」
- Agent 动作序列：
  1. [链下查询] 获取钱包地址：`onchainos wallet balance --chain 501` → 解析 wallet pubkey
  2. [链下查询] 确认余额充足：SOL balance ≥ 0.01 + gas (~0.001 SOL)
  3. [链下查询] `GET https://api.jup.ag/swap/v2/order?inputMint=So11...112&outputMint=EPjF...1v&amount=10000000&taker=<wallet_pubkey>&slippageBps=50`
  4. 提取响应中的 `transaction` 字段（base64 编码的 unsigned versioned tx）
  5. [代码转换] base64 → bytes → base58
  6. [链上操作] `onchainos wallet contract-call --unsigned-tx <base58_tx> --to JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 --chain 501 --force`
  7. 等待 tx 确认，返回 signature 和实际兑换数量

**场景 3：查询 SOL 的实时 USD 价格**

- 用户说：「What is the current price of SOL in USD?」
- Agent 动作序列：
  1. [链下查询] `GET https://api.jup.ag/price/v3?ids=So11111111111111111111111111111111111111112`
  2. 解析响应：`data["So11...112"]["usdPrice"]`
  3. 返回给用户：SOL 当前 USD 价格、24h 涨跌幅 (`priceChange24h`)

**场景 4：搜索代币 mint 地址后执行兑换**

- 用户说：「Swap 5 USDC for JUP token on Jupiter」
- Agent 动作序列：
  1. [链下查询] 搜索 JUP token mint: `GET https://api.jup.ag/tokens/v2/search?query=JUP` → 取 `verified=true` 的第一个结果的 `address` 字段
  2. [链下查询] 获取钱包地址：`onchainos wallet balance --chain 501`
  3. [链下查询] `GET https://api.jup.ag/swap/v2/order?inputMint=EPjF...1v&outputMint=<JUP_mint>&amount=5000000&taker=<wallet_pubkey>&slippageBps=50`
     （USDC amount: 5 USDC × 10^6 = `5000000`）
  4. 提取 `transaction` (base64) → convert to base58
  5. [链上操作] `onchainos wallet contract-call --unsigned-tx <base58_tx> --to JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4 --chain 501 --force`
  6. 返回确认结果和实际兑换数量

---

## §4 外部 API 依赖

| API | Base URL | 用途 | 需要 API Key？ |
|-----|----------|------|---------------|
| Jupiter Swap API v2 | `https://api.jup.ag/swap/v2` | 获取兑换报价 + 构建未签名 tx | No（keyless 0.5 RPS）；可选 `x-api-key` header 提升至 1–150 RPS |
| Jupiter Price API v3 | `https://api.jup.ag/price/v3` | 查询代币 USD 实时价格（max 50 tokens/request） | No（同上） |
| Jupiter Tokens API v2 | `https://api.jup.ag/tokens/v2` | 搜索代币 metadata（name, symbol, mint address） | No |

> **Rate limits (keyless)**: 0.5 RPS per IP across all endpoints. For production use, register at https://developers.jup.ag/portal for a free API key (1 RPS) or paid tier (up to 150 RPS).

---

## §5 配置参数

| Parameter | Default | Description |
|-----------|---------|-------------|
| `default_slippage_bps` | `50` | 默认滑点容忍度（单位：bps，50 = 0.5%）。传入 `slippageBps` 查询参数。 |
| `dry_run` | `true` | 模拟模式：构建并显示 tx 内容，不执行 `--force` 广播 |
| `restrict_intermediate_tokens` | `true` | 仅允许通过高流动性中间代币路由（减少失败风险） |
| `max_accounts` | `64` | 内部 swap 指令最大账户数（Jupiter 推荐值）。超出可能导致 tx 失败 |
| `priority_fee_lamports` | `auto` | 优先费用（`auto` = 使用 Jupiter 动态估算；或指定 lamports 整数值） |

---

## §6 关键技术注意事项

### base64 → base58 转换（必做）

`/swap/v2/order` 的响应中 `transaction` 字段是 **base64** 编码的 versioned Solana transaction。`onchainos wallet contract-call --unsigned-tx` 期望 **base58**。

Cargo.toml 依赖（必须添加）：
```toml
base64 = "0.22"
bs58 = "0.5"
```

Rust 转换代码：
```rust
use base64::Engine;
let bytes = base64::engine::general_purpose::STANDARD.decode(&base64_tx)?;
let base58_tx = bs58::encode(&bytes).into_string();
```

### onchainos Solana 特殊规则

- 链 ID：`501`（不是 `"solana"`）
- `onchainos wallet balance --chain 501` — **不加 `--output json`**（Solana 原生返回 JSON，加了会 EOF 失败）
- 钱包地址路径：`json["data"]["details"][0]["tokenAssets"][0]["address"]`
- `--unsigned-tx` 需配合 `--force` 使用（跳过 confirmation prompt）

### Token mint 地址（常用）

| Token | Mint Address |
|-------|-------------|
| SOL (Wrapped) | `So11111111111111111111111111111111111111112` |
| USDC | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| USDT | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| JUP | `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN` |

### Jupiter Program ID

```
JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4
```

This is the `--to` address for `onchainos wallet contract-call`.

---

## §7 Open Questions

| # | Question | Blocking? | Status |
|---|----------|-----------|--------|
| 1 | Does `/swap/v2/order` (Meta-Aggregator path) work without an API key in sandbox/test? Need to verify 0.5 RPS keyless access is sufficient for basic testing. | No | Open |
| 2 | `wrapAndUnwrapSol` parameter needed? v2 `/order` endpoint likely handles SOL wrapping automatically (native SOL → wSOL). Need to confirm with live test. | No | Open — assume auto-handled in v2 |
| 3 | Does `--unsigned-tx` work for versioned (v0) transactions with Address Lookup Tables on onchainos 501? Raydium plugin uses same flow — assume Yes. | No | Open — assume Yes per Raydium precedent |
| 4 | API key requirement for production: keyless rate limit (0.5 RPS) may be insufficient under load. Should plugin accept optional `JUP_API_KEY` env var? | No | Defer to Developer Agent |
