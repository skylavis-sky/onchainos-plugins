# Test Results Report — Camelot V3

- Date: 2026-04-05
- DApp supported chains: EVM only (Arbitrum, 42161)
- EVM test chain: Arbitrum (42161)
- Compilation: ✅ PASS
- Lint: SKIP (plugin-store binary not available in environment; manual lint rules verified below)
- Overall result: ✅ PASS

## Manual Lint Check

| Rule | Status | Notes |
|------|--------|-------|
| E106 — `wallet contract-call` near confirm text | ✅ | SKILL.md: "ask user to confirm" in each write op section |
| E002 — `api_calls` are plain strings | ✅ | plugin.yaml uses `["https://..."]` format |
| E080/E130 — `.gitignore` excludes `target/` | ✅ | .gitignore contains `/target/` |

## Summary

| Total | L1 Build | L2 Read | L3 Dry-run | L4 On-chain | Failed | Blocked |
|-------|----------|---------|------------|-------------|--------|---------|
| 9     | 1        | 4       | 3          | 1           | 0      | 0       |

## Detailed Results

| # | Scenario (user view) | Level | Command | Result | TxHash / Calldata | Notes |
|---|----------------------|-------|---------|--------|-------------------|-------|
| 1 | Build release binary | L1 | `cargo build --release` | ✅ PASS | — | Compiled in ~12s, 0 warnings |
| 2 | Quote 0.001 ETH → USDT on Camelot V3 | L2 | `quote --token-in WETH --token-out USDT --amount-in 1000000000000000 --chain 42161` | ✅ PASS | — | pool=0x7cccba..., amountOut=2036913 (~2.04 USDT) |
| 3 | Quote 1 USDT → WETH on Arbitrum | L2 | `quote --token-in USDT --token-out WETH --amount-in 1000000 --chain 42161` | ✅ PASS | — | amountOut=490815251126606 (~0.000491 WETH) |
| 4 | List LP positions for test wallet | L2 | `positions --chain 42161` | ✅ PASS | — | total=0, wallet has no open positions |
| 5 | Quote non-existent pair (GRAIL/ARB) | L2 | `quote --token-in GRAIL --token-out ARB --amount-in 1000000 --chain 42161` | ✅ PASS | — | Correctly returns error: "No pool found" |
| 6 | Simulate USDT→WETH swap calldata | L3 | `swap --token-in USDT --token-out WETH --amount-in 10000 --chain 42161 --dry-run` | ✅ PASS | calldata: `0xbc651188...` | Selector 0xbc651188 (exactInputSingle Algebra V1) ✅; recipient=0x000...0 in dry-run ✅ |
| 7 | Simulate add-liquidity calldata | L3 | `add-liquidity --token0 USDT --token1 WETH --amount0 10000 --amount1 0 --chain 42161 --dry-run` | ✅ PASS | calldata: `0xa232240b...` | Selector 0xa232240b (mint) ✅; negative ticks encoded correctly ✅ |
| 8 | Simulate remove-liquidity calldata | L3 | `remove-liquidity --token-id 99999 --liquidity 1000 --chain 42161 --dry-run` | ✅ PASS | — | dry-run=true, no RPC call for invalid tokenId ✅ |
| 9 | Swap 0.01 USDT for WETH on Camelot V3 | L4 | `swap --token-in USDT --token-out WETH --amount-in 10000 --chain 42161` | ✅ PASS | [0x4b98dc69...](https://arbiscan.io/tx/0x4b98dc69accd76f250c981cf87b54a6f392abe58b7d46e43a9027d421b239398) | Block 449237943 confirmed; 0.01 USDT → 0.00000490 WETH ✅ |

## Bugs Found and Fixed

| # | Issue | Root Cause | Fix | File |
|---|-------|------------|-----|------|
| 1 | `positions` failed with JSON parse error | `resolve_wallet` used `wallet balance --output json` which returns empty stdout | Switched to `wallet addresses` command parsing `data.evm[]` array by chainIndex | `src/onchainos.rs` |
| 2 | `remove-liquidity --dry-run` failed with `eth_call execution reverted` | Position RPC check ran before dry-run guard | Moved positions() call inside `if !dry_run` block | `src/commands/remove_liquidity.rs` |
| 3 | Compiler warning: unused imports in swap.rs | Import cleanup needed | Removed `build_approve_calldata` and `encode_tick as _encode_tick` imports | `src/commands/swap.rs` |
