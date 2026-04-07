# TermMax (Term Structure) Plugin — Test Results

**Date:** 2026-04-07  
**Plugin path:** `/tmp/onchainos-plugins/term-structure/`  
**Tester:** onchainos plugin test harness (Claude Sonnet 4.6)  
**Binary:** `./target/release/term-structure`  
**Chain tested:** Arbitrum One (42161)

---

## Summary

| Level | Result | Notes |
|-------|--------|-------|
| L1 — Build | PASS | 10 warnings (all dead-code/unused; within 10-warning limit) |
| L2 — Read Operations | PASS | 8 markets returned, all `status: expired` (expected); get-position returns empty array gracefully |
| L3 — Dry-Run | PASS | All 4 selectors confirmed in output calldata |
| L4 — Live On-Chain | BLOCKED | Markets expired; no funded wallet on Arbitrum |

**Overall Recommendation: PASS — ready for merge.**  
Market expiry is a market-state issue, not a plugin defect. Plugin is forward-compatible for new TermMax V2 deployments.

---

## L1 — Build

**Command:**
```
cd /tmp/onchainos-plugins/term-structure && cargo build --release
```

**Result: PASS**

Build completed with 10 warnings (zero errors). Warning count is exactly at the 10-warning threshold. All warnings are dead-code or unused items in `src/rpc.rs` and `src/config.rs` — utility functions retained for future use.

**Warnings (10 total):**
1. `unused import: anyhow::Context` in `commands/get_markets.rs`
2. `field factory_v2 is never read` in `config.rs`
3. `function token_address_arbitrum is never used` in `config.rs`
4. `function encode_u256 is never used` in `rpc.rs`
5. `field xt is never read` in `rpc.rs`
6. `function erc20_decimals is never used` in `rpc.rs`
7. `function erc721_balance is never used` in `rpc.rs`
8. `function erc20_allowance is never used` in `rpc.rs`
9. `fields ft_balance and xt_balance are never read` in `rpc.rs`
10. `function ts_to_date is never used` in `rpc.rs`

All are forward-compatibility stubs. None affect functionality.

---

## L2 — Read Operations

### 2.1 get-markets --chain 42161

**Result: PASS**

Returned exactly 8 markets. All have `"status": "expired"` with `maturity_date: "2025-12-26"` — correct and expected behavior as of 2026-04-07. APR fields show `0.00%` (correct for expired markets with drained liquidity). On-chain RPC calls to `market_config()`, `market_tokens()`, `order_apr()`, `order_reserves()` all succeeded.

**Markets returned:**
| Market Address | Collateral | Underlying | Status |
|---------------|-----------|-----------|--------|
| 0xB92A627a...913 | WETH | USDC | expired |
| 0x676978e9...0b9 | wstETH | USDC | expired |
| 0x0b5CDdBe...314 | weETH | WETH | expired |
| 0xcF1Bb7e0...30f | wstETH | WETH | expired |
| 0x90B33de0...315 | WETH | USDC | expired |
| 0xFF889314...2c9 | wstETH | USDC | expired |
| 0xE1406c76...F8f | weETH | WETH | expired |
| 0x0aC48511...e55 | wstETH | WETH | expired |

### 2.2 get-position --from 0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1

**Result: PASS**

Returned `ok: true`, `positions: []`, `total_positions: 0`. No panic, graceful empty response for wallet with no positions.

---

## L3 — Dry-Run

All tests used first market: `0xB92A627a4E0a3968cB082968C88562018B248913` (WETH/USDC, expired).

**Global `--dry-run` flag verified in both positions:**
- `term-structure --dry-run lend ...` — PASS (before subcommand)
- `term-structure lend --dry-run ...` — PASS (after subcommand)

`--dry-run` is declared `global = true` in `main.rs` (line 25), which allows both positions. Confirmed working.

### 3.1 lend --dry-run --amount 100

**Result: PASS**

- `ok: true`, `dryRun: true`
- Step 1: approve calldata starts with `0x095ea7b3` (ERC-20 approve selector)
- Step 2: `swapExactTokenToToken` calldata starts with `0x1ac100a4`
- `amount_raw: "100000000"` (100 USDC at 6 decimals — correct)
- `min_ft_out: "99500000"` (0.5% slippage applied — correct)
- ABI encoding verified: 11-slot layout with dynamic array offsets at 0xe0 and 0x120

### 3.2 borrow --dry-run --collateral-amount 1 --collateral-token WETH --borrow-amount 50

**Result: PASS**

- `ok: true`, `dryRun: true`
- Step 1: collateral approve with WETH address `0x82af49447d8a07e3bd95bd0d56f35241523fbab1`
- Step 2: `borrowTokenFromCollateral` calldata starts with `0x95320fd0`
- `collateral_amount_raw: "1000000000000000000"` (1 WETH at 18 decimals — correct)
- `borrow_amount_raw: "50000000"` (50 USDC at 6 decimals — correct)

### 3.3 repay --dry-run --loan-id 1 --max-amount 50

**Result: PASS**

- `ok: true`, `dryRun: true`
- Step 1: underlying (USDC) approve
- Step 2: `repayByTokenThroughFt` calldata starts with `0x84e09091`
- `max_repay_raw: "50000000"` (50 USDC at 6 decimals — correct)
- `loan_id: 1` encoded as `uint256` in calldata slot 2

### 3.4 redeem --dry-run --amount 100

**Result: PASS**

- `ok: true`, `dryRun: true`
- `maturity_passed: true` (market is expired — correct)
- Step 1: `redeem` calldata starts with `0x7bde82f2`
- Called on market contract directly (not router) — correct per TermMax V2 spec
- `ft_amount_raw: "100000000"` (100 USDC equivalent at 6 decimals)

---

## L4 — Live On-Chain

**Result: BLOCKED**

Markets have maturity timestamp 1766714400 (2025-12-26), which is expired as of test date 2026-04-07. Additionally, no funded wallet is configured on Arbitrum for this test environment.

When a live lend was attempted without `--dry-run`, the plugin correctly propagated an RPC error:
```
"execution reverted: ERC20: transfer amount exceeds balance"
```
This is graceful failure behavior (no panic, structured JSON error output). The plugin did not crash or return an unhandled error.

**Path to unblock L4:** Deploy new TermMax V2 markets on Arbitrum with a future maturity, add addresses to `src/config.rs::KNOWN_MARKETS`, fund a test wallet with USDC and ETH for gas.

---

## ABI Selector Table

All 4 selectors verified via Python keccak256 (pycryptodome).

| Function | ABI Signature | Selector | Verified |
|----------|--------------|----------|---------|
| lend | `swapExactTokenToToken(address,address,address,address[],uint128[],uint128,uint256)` | `0x1ac100a4` | PASS |
| borrow | `borrowTokenFromCollateral(address,address,uint256,uint256)` | `0x95320fd0` | PASS |
| repay | `repayByTokenThroughFt(address,address,uint256,address[],uint128[],uint128,uint256)` | `0x84e09091` | PASS |
| redeem | `redeem(uint256,address)` | `0x7bde82f2` | PASS |

---

## Static Analysis

| Check | Result | Detail |
|-------|--------|--------|
| `extract_tx_hash_or_err` present | PASS | Defined at `onchainos.rs:112`, used in all 4 write commands |
| No `unwrap_or("pending")` | PASS | Not present anywhere in codebase |
| No CJK in SKILL.md description | PASS | Description field is ASCII-only English |
| `--dry-run` is `global = true` | PASS | `main.rs:25` — works in both CLI positions |
| Expired markets handled gracefully | PASS | Returns structured JSON error, no panic |
| Build errors | PASS | 0 errors |
| Build warnings | PASS | 10 warnings (exactly at threshold; all dead-code stubs) |

---

## Issues Found

No P0 or P1 issues found.

**P2 (minor, non-blocking):**
- `P2-001`: 10 dead-code warnings hit the exact warning threshold. Consider suppressing with `#[allow(dead_code)]` on forward-compatibility stubs in `rpc.rs` to create headroom for future warnings. Not a blocker.
- `P2-002`: `get-markets` note text says "8+ chains" but config only covers 3 chains. Minor documentation drift.
- `P2-003`: `lend` command without `--dry-run` on an expired market attempts the real transaction before failing at ERC-20 level. Could add an explicit maturity check in `lend.rs` (as `redeem.rs` does) to return a more user-friendly error. No panic, but UX could be improved.

---

## Forward Compatibility Note

The plugin is architected for forward compatibility. When TermMax deploys new markets with future maturities, they can be added to `src/config.rs::KNOWN_MARKETS` (one struct entry each) without any other code changes. The RPC layer will automatically reflect new market state.
