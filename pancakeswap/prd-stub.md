# PancakeSwap AMM — Plugin Store 接入 PRD

> 通过 onchainos CLI 接入 PancakeSwap AMM，使 AI Agent 能完成 swap 和流动性管理操作

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `pancakeswap` |
| dapp_name | PancakeSwap AMM |
| dapp_repo | https://github.com/pancakeswap/pancake-smart-contracts |
| dapp_alias | pancake, pcs, pancakeswap v3 |
| one_liner | Swap tokens and manage liquidity on PancakeSwap — the leading DEX on BSC and Base |
| category | defi-protocol |
| tags | dex, swap, liquidity, amm, pancakeswap, bsc |
| target_chains | bsc (56), base (8453), ethereum (1) |
| target_protocols | PancakeSwap V3 AMM |

---

## 1. Background

### 这个 DApp 是什么

PancakeSwap is the leading DEX on BNB Chain with $2B+ TVL. It offers V3 concentrated liquidity AMM (similar to Uniswap V3), plus V2 AMM pools. Core operations are token swaps and liquidity provision.

### 接入可行性调研

> Researcher Agent: please complete this table

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | TBD |
| SDK 支持哪些技术栈？ | TBD |
| 有 REST API？ | TBD |
| 有官方 Skill？ | TBD |
| 开源社区有类似 Skill？ | TBD |
| 支持哪些链？ | BSC (56), Base (8453), Ethereum (1), Arbitrum (42161) |
| 是否需要 onchainos 广播？ | Yes |

### 接入路径判定

TBD — Researcher Agent to determine.

---

## 2. Interface Mapping

> Researcher Agent: please complete §2–§5

### Operations to support (at minimum)

| # | Operation | Type | Priority |
|---|-----------|------|----------|
| 1 | Swap tokens (exact input) | On-chain | P0 |
| 2 | Get swap quote / price | Off-chain read | P0 |
| 3 | Get pool info / liquidity | Off-chain read | P0 |
| 4 | Add liquidity (V3 position) | On-chain | P1 |
| 5 | Remove liquidity | On-chain | P1 |
| 6 | View my LP positions | Off-chain read | P1 |

---

## 3. User Scenarios

> Researcher Agent: please write at least 3 complete scenarios

---

## 4. External API Dependencies

> Researcher Agent: to complete

---

## 5. Configuration Parameters

> Researcher Agent: to complete

| Parameter | Default | Description |
|-----------|---------|-------------|
| chain | 56 (BSC) | Target chain ID |
| slippage | 0.5% | Max slippage tolerance |
| dry_run | false | Simulate without broadcasting |
