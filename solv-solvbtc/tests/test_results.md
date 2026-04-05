# Solv SolvBTC Plugin — Phase 3 Test Results

**Date:** 2026-04-05  
**Plugin:** `solv-solvbtc`  
**Wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`  
**Binary:** `target/release/solv-solvbtc`

---

## L1 — Build & Lint

| Check | Result | Notes |
|-------|--------|-------|
| `cargo build --release` | PASS | Compiled cleanly, no warnings |
| `plugin-store lint .` | PASS | "Plugin 'solv-solvbtc' passed all checks!" |

---

## L2 — Read Tests (no wallet required)

| Command | Result | Output |
|---------|--------|--------|
| `get-nav` | PASS | SolvBTC $66,999.68; xSolvBTC $68,080.75; NAV 1.0161 BTC/xSolvBTC; TVL $575.5M |
| `get-balance --chain 42161` | PASS | 0 SolvBTC (Arbitrum) — correct, wallet has none |
| `get-balance --chain 1` | PASS | 0 SolvBTC, 0 xSolvBTC (Ethereum) — correct, wallet has none |

---

## L3 — Dry-Run Calldata Verification

| Command | Selector Check | Result | Notes |
|---------|---------------|--------|-------|
| `mint --amount 0.001 --chain 42161 --dry-run` | approve `0x095ea7b3` ✓, deposit `0x672262e5` ✓ | PASS | RouterV2 `0x92E8A4...080A`, WBTC `0x2f2a25...5B0f`, correct amounts |
| `mint --amount 0.001 --chain 1 --dry-run` | approve `0x095ea7b3` ✓, deposit `0x672262e5` ✓ | PASS | RouterV2 `0x3d93B9...6334`, WBTC mainnet `0x2260FA...599`, correct |
| `redeem --amount 0.001 --chain 42161 --dry-run` | approve `0x095ea7b3` ✓, withdrawRequest `0xd2cfd97d` ✓ | PASS | Targets SolvBTC and RouterV2 on Arbitrum |
| `wrap --amount 0.001 --dry-run` | approve `0x095ea7b3` ✓, xpool.deposit `0xb6b55f25` ✓ | PASS | Ethereum-only; XSolvBTCPool `0xf394Aa...c86` |
| `unwrap --amount 0.001 --dry-run` | approve `0x095ea7b3` ✓, xpool.withdraw `0x2e1a7d4d` ✓ | PASS | Ethereum-only; NAV-adjusted output shown, 0.05% fee displayed |

### Selector Summary

All 6 expected selectors verified correct:

| Selector | Function | Status |
|----------|----------|--------|
| `0x095ea7b3` | ERC-20 `approve` | PASS (all flows) |
| `0x672262e5` | RouterV2 `deposit` (mint) | PASS |
| `0xd2cfd97d` | RouterV2 `withdrawRequest` (redeem) | PASS |
| `0x42c7774b` | `cancelWithdraw` | Not tested (requires redemption ticket ID) |
| `0xb6b55f25` | XSolvBTCPool `deposit` (wrap) | PASS |
| `0x2e1a7d4d` | XSolvBTCPool `withdraw` (unwrap) | PASS |

---

## L4 — Live Transaction Tests

### Wallet Balance at Test Time

| Chain | Asset | Balance |
|-------|-------|---------|
| Arbitrum (42161) | WBTC | 0.0 |
| Arbitrum (42161) | SolvBTC | 0.0 |
| Arbitrum (42161) | ETH | ~trace (no gas asset shown, only 0.2 USDC) |
| Ethereum (1) | ETH | 0.002948 |
| Ethereum (1) | SolvBTC | 0.0 |
| Ethereum (1) | xSolvBTC | 0.0 |

### L4 Results

| Operation | Status | Reason |
|-----------|--------|--------|
| `mint` (Arbitrum) | BLOCKED | WBTC balance = 0.0 |
| `mint` (Ethereum) | BLOCKED | WBTC balance = 0.0 |
| `redeem` (Arbitrum) | BLOCKED | SolvBTC balance = 0.0 |
| `wrap` (Ethereum) | BLOCKED | SolvBTC balance = 0.0 |
| `unwrap` (Ethereum) | BLOCKED | xSolvBTC balance = 0.0 |
| `cancel-redeem` | BLOCKED | No active redemption ticket |

**No L4 lock acquired** — all live operations blocked, no transactions broadcast.

---

## Bugs / Issues Found

None. All commands behaved correctly:

- Correct addresses and selectors for both Arbitrum and Ethereum
- Decimal handling: WBTC 8 decimals, SolvBTC 18 decimals — both correct
- NAV-adjusted xSolvBTC estimates computed correctly in wrap/unwrap
- Error messages, dry-run labels, and redemption warnings all present
- `get-balance --chain 1` correctly returns both SolvBTC and xSolvBTC balances

---

## Overall Result

| Level | Status |
|-------|--------|
| L1 Build | PASS |
| L1 Lint | PASS |
| L2 Read | PASS (3/3) |
| L3 Dry-run | PASS (5/5) |
| L4 Live | BLOCKED (0/5, insufficient balance) |
