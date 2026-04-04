# Pendle Finance — Test Cases

**Date:** 2026-04-05
**Test Chain:** Base (8453) for L4; Arbitrum (42161) for L2/L3

---

## L1 — Compile + Lint

| # | Test | Expected |
|---|------|---------|
| L1-1 | `cargo build --release` | Exit 0, binary at `target/release/pendle` |
| L1-2 | `cargo clean && plugin-store lint .` | 0 errors, 0 warnings |

---

## L2 — Read Operations (no wallet, no gas)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L2-1 | User asks "show me Pendle markets on Arbitrum" | `list-markets --chain-id 42161 --active-only --limit 3` | JSON `results[]` with markets, `name`, `address`, `pt`, `yt`, `chainId` |
| L2-2 | User asks "show all Pendle markets" | `list-markets --limit 5` | JSON `results[]` cross-chain |
| L2-3 | User asks "APY history for gUSDC market" | `--chain 42161 get-market --market 0x0934e592cee932b04b3967162b3cd6c85748c470` | JSON `results[]` with `impliedApy`, `tvl`, `timestamp` |
| L2-4 | User asks "what Pendle positions do I hold" | `get-positions --user 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | JSON with `positions` array (may be empty) |
| L2-5 | User asks "PT prices on Arbitrum" | `get-asset-price --chain-id 42161 --asset-type PT` | JSON `prices{}` map with many PT addresses |
| L2-6 | User asks "price of specific PT token" | `get-asset-price --ids 42161-0x97c1a4ae3e0da8009aff13e3e3ee7ea5ee4afe84 --chain-id 42161` | JSON `prices{}` with one entry, non-zero value |

---

## L3 — Dry-run / Simulate (write ops, no broadcast)

All L3 tests use `--dry-run` flag and `--from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`.
Market data: gUSDC on Arbitrum (chain 42161), market `0x0934e592cee932b04b3967162b3cd6c85748c470`.

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L3-1 | User previews buying PT with 100 USDC (Arbitrum) | `--chain 42161 --dry-run buy-pt --token-in 0xaf88d...5831 --amount-in 100000000 --pt-address 0x97c1a4...fe84 --from ... --slippage 0.01` | `ok:true`, `calldata` starts with `0x`, non-empty, `dry_run:true`, `router:0x8888...` |
| L3-2 | User previews selling 100 USDC worth of PT (Arbitrum) | `--chain 42161 --dry-run sell-pt --pt-address 0x97c1a4...fe84 --amount-in 100000000 --token-out 0xaf88d...5831 --from ...` | `ok:true`, calldata `0x5...` non-empty, `dry_run:true` |
| L3-3 | User previews adding liquidity to gUSDC pool (Arbitrum) | `--chain 42161 --dry-run add-liquidity --token-in 0xaf88d...5831 --amount-in 100000000 --lp-address 0x0934e5...470 --from ...` | `ok:true`, calldata `0x1...` non-empty, `dry_run:true` |
| L3-4 | User previews removing LP tokens from gUSDC pool (Arbitrum) | `--chain 42161 --dry-run remove-liquidity --lp-address 0x0934e5...470 --lp-amount-in 10000 --token-out 0xaf88d...5831 --from ...` | `ok:true`, calldata `0x6...` non-empty, `dry_run:true` |

---

## L4 — On-chain Write Operations (real broadcast, min amounts)

**Chain:** Base (8453)
**Wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`
**Per GUARDRAILS:** max 0.00005 ETH, max 0.01 USDC per test (Pendle SDK minimum is 0.02 USDC; noted in results)

| # | Scenario (user view) | Command | Token | Amount |
|---|---------------------|---------|-------|-------|
| L4-1 | User buys PT-wsuperOETHb fixed yield with 0.00005 WETH on Base | `--chain 8453 buy-pt --token-in 0x4200...0006 --amount-in 50000000000000 --pt-address 0x5fab...c0cc --from ... --slippage 0.01` | WETH | 0.00005 ETH |
| L4-2 | User adds liquidity to wsuperOETHb pool with 0.02 USDC on Base | `--chain 8453 add-liquidity --token-in 0x8335...2913 --amount-in 20000 --lp-address 0x9621...b67 --from ... --slippage 0.01` | USDC | 0.02 USDC (Pendle minimum > 0.01) |
