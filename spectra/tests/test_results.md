# Spectra Finance Plugin — Test Results

**Date:** 2026-04-07  
**Plugin path:** `/tmp/onchainos-plugins/spectra/`  
**Tester:** Claude (automated)  
**Binary:** `./target/release/spectra`  
**Primary chain:** Base (8453)

---

## Summary

| Level | Result | Notes |
|-------|--------|-------|
| L1 — Build | **PASS** | 12 dead_code warnings, zero errors |
| L2 — Read Operations | **PASS** | All 3 read commands return valid JSON; Registry confirmed pTCount=69 |
| L3 — Dry-Run | **PASS** | All 5 write commands produce correct calldata with correct selectors |
| L4 — Live On-Chain | **BLOCKED** | No test wallet funds on Base |
| Static Analysis | **PASS** (with P2 notes) | extract_tx_hash_or_err correct; --force present; CURVE_SWAP_SNG=0x1E; no CJK |

**Overall Recommendation: APPROVE for production merge**

---

## L1 — Build

```
cargo build --release
Finished `release` profile [optimized] target(s) in 0.15s
```

**Result: PASS**

Warnings (all acceptable dead_code): `DEFAULT_SLIPPAGE`, `SWAP_SLIPPAGE`, `MAX_PRICE_IMPACT_WARN`, `MAX_PRICE_IMPACT_BLOCK`, `CMD_CURVE_SWAP`, `CMD_DEPOSIT_ASSET_IN_IBT`, `CMD_DEPOSIT_ASSET_IN_PT`, `CMD_DEPOSIT_IBT_IN_PT`, `CMD_REDEEM_IBT_FOR_ASSET`, `CMD_REDEEM_PT_FOR_ASSET`, `CMD_REDEEM_PT_FOR_IBT`, `curve_pool_type` field. These constants are defined for completeness/future use; not a defect.

---

## L2 — Read Operations

### L2.1 — `get-pools --chain 8453`

**Result: PASS**

```json
{
  "chain_id": 8453,
  "count": 3,
  "ok": true,
  "pools": [
    { "name": "weETH (Ether.fi)", "active": true, "days_to_maturity": 98, "total_registered_pts": 69, ... },
    { "name": "sjEUR (Jarvis)", "active": true, ... },
    { "name": "wsuperOETHb (Origin)", "active": true, ... }
  ]
}
```

- API endpoint unavailable (Next.js build ID scraping failed) — fell back to on-chain Registry correctly
- Registry confirmed `pTCount()` = **69** as expected
- All 3 known pools returned with live on-chain maturity timestamps

### L2.2 — `get-pools --chain 8453 --active-only --limit 5`

**Result: PASS** — Identical to L2.1 (all 3 known pools are active; no expired pools in KNOWN_BASE_POOLS)

### L2.3 — `get-position --chain 8453 --user 0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1`

**Result: PASS** (correct behavior for zero-balance wallet)

Note: `--from` flag is not accepted by `get-position` (uses `--user`). The test spec listed `--from` — this is a spec documentation error, not a plugin defect. The `--user` flag is correct per the CLI design.

```json
{
  "chain_id": 8453,
  "ok": true,
  "position_count": 0,
  "positions": [],
  "wallet": "0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1"
}
```

Registry enumeration with `getPTAt(uint256)` (`0x6c40a4f0`) executed without error across all 69 PT slots. Zero-balance wallet returns empty positions (expected).

---

## L3 — Dry-Run Operations

PT used: `0x07f58450a39d07f9583c188a2a4a441fac358100` (weETH, active, 98 days to maturity)  
Wallet: `0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1`  
Amount: `10000000000000000` (0.01 WETH in wei)

### L3.1 — `--dry-run deposit --pt ... --amount 10000000000000000`

**Result: PASS**

- Selector in calldata: `0xe4cca4b0` ✓ matches `deposit(uint256,address,address,uint256)`
- `previewDeposit` returned 9999999999999999 estimated shares (near 1:1 weETH)
- minShares with 0.5% slippage: 9950000000000000 ✓
- Underlying token approved: `0x4200000000000000000000000000000000000006` (WETH) ✓
- Both `approve_tx` and `tx_hash` = zero hash (correct dry-run behavior) ✓

### L3.2 — `--dry-run deposit --pt ... --amount 10000000000000000 --use-ibt`

**Result: PASS**

- Selector in calldata: `0x2a412806` ✓ matches `depositIBT(uint256,address,address,uint256)`
- IBT token approved: `0x22f757c0b434d93c93d9653f26c9441d8d06c8ec` (weETH IBT) ✓
- Calldata encodes 4 parameters (amount, ptReceiver, ytReceiver, minShares) ✓

### L3.3 — `--dry-run claim-yield --pt ...`

**Result: PASS**

- Selector in calldata: `0x999927df` ✓ matches `claimYield(address)`
- `getCurrentYieldOfUserInIBT` returned 0 for test wallet (expected for zero-balance wallet)
- dry_run=true bypasses the no-yield early-exit guard ✓
- Calldata: `0x999927df` + padded receiver address ✓

### L3.4 — `--dry-run redeem --pt ... --shares 10000000000000000`

**Result: PASS**

- PT is active (not expired), so `withdraw` path selected
- Selector: `0xb460af94` ✓ matches `withdraw(uint256,address,address)`
- Pre-expiry branch note visible in output: "withdraw (pre-expiry, requires equal YT)" ✓
- No approve step (redeem does not require prior approval) ✓

### L3.5 — `--dry-run swap --pt ... --amount-in 10000000000000000 --sell-pt`

**Result: PASS**

- Router selector: `0x24856bc3` ✓ matches `execute(bytes,bytes[])`
- Command bytes decoded from calldata: `[0x00, 0x1e]` = `[TRANSFER_FROM, CURVE_SWAP_SNG]` ✓
- `CURVE_SWAP_SNG = 0x1E` is used (not `CURVE_SWAP = 0x03`) ✓
- sell_pt=true → i=1 (PT), j=0 (IBT) ✓
- Curve pool correctly resolved: `0x3870a9498cd7ced8d134f19b0092931ef83aec1e` ✓
- Router address: `0xc03309de321a4d3df734f5609b80cc731ae28e6d` ✓
- min_amount_out estimated via `get_dy(uint256,uint256,uint256)` (`0x556d6e9f`) — SNG-correct ✓

---

## L4 — Live On-Chain

**Result: BLOCKED** — No test wallet with WETH or USDC on Base available.

---

## Static Analysis

| Check | Result | Detail |
|-------|--------|--------|
| `extract_tx_hash_or_err` present (not `unwrap_or("pending")`) | **PASS** | Function at `onchainos.rs:78` checks `data.txHash`, then `txHash`, then `error` field, only falls back to `"pending"` when no error field exists |
| No `unwrap_or("pending")` raw pattern | **PASS** | `grep` found zero instances |
| `--force` in `erc20_approve` | **PASS** | `wallet_contract_call(..., true, ...)` — force=true passed ✓ |
| `--force` in write operations | **PASS** | `deposit.rs:109`, `redeem.rs:95`, `claim_yield.rs:64`, `swap.rs:162` all pass `force=true` ✓ |
| No CJK in `SKILL.md` description field | **PASS** | Zero CJK characters found |
| `Do NOT use for` section in `SKILL.md` | **PASS** | Section present at line 50–55 |
| `CURVE_SWAP_SNG = 0x1E` used (not `CURVE_SWAP = 0x03`) | **PASS** | `swap.rs` imports `CMD_CURVE_SWAP_SNG`; calldata confirmed `0x1e` |
| Registry address on Base | **PASS** | `0x786da12e9836a9ff9b7d92e8bac1c849e2ace378` ✓ |
| Router address on Base | **PASS** | `0xc03309de321a4d3df734f5609b80cc731ae28e6d` ✓ |

---

## ABI Selector Verification Table

All selectors verified via Python `keccak256` (pycryptodome):

| Function Signature | Expected | Code Uses | Status |
|-------------------|----------|-----------|--------|
| `deposit(uint256,address,address,uint256)` | `0xe4cca4b0` | `0xe4cca4b0` | **PASS** |
| `depositIBT(uint256,address,address,uint256)` | `0x2a412806` | `0x2a412806` | **PASS** |
| `claimYield(address)` | `0x999927df` | `0x999927df` | **PASS** |
| `claimYieldInIBT(address)` | `0x0fba731e` | `0x0fba731e` | **PASS** |
| `execute(bytes,bytes[])` | `0x24856bc3` | `0x24856bc3` | **PASS** |
| `redeem(uint256,address,address,uint256)` | `0x9f40a7b3` | `0x9f40a7b3` | **PASS** |
| `withdraw(uint256,address,address)` | `0xb460af94` | `0xb460af94` | **PASS** |
| `pTCount()` | `0x704bdadc` | `0x704bdadc` | **PASS** |
| `getPTAt(uint256)` | `0x6c40a4f0` | `0x6c40a4f0` | **PASS** |
| `maturity()` | `0x204f83f9` | `0x204f83f9` | **PASS** |
| `getIBT()` | `0xc644fe94` | `0xc644fe94` | **PASS** |
| `underlying()` | `0x6f307dc3` | `0x6f307dc3` | **PASS** |
| `getYT()` | `0x04aa50ad` | `0x04aa50ad` | **PASS** |
| `previewDeposit(uint256)` | `0xef8b30f7` | `0xef8b30f7` | **PASS** |
| `previewRedeem(uint256)` | `0x4cdad506` | `0x4cdad506` | **PASS** |
| `getCurrentYieldOfUserInIBT(address)` | `0x0e1b6d89` | `0x0e1b6d89` | **PASS** |
| `balanceOf(address)` | `0x70a08231` | `0x70a08231` | **PASS** |
| `approve(address,uint256)` | `0x095ea7b3` | `0x095ea7b3` | **PASS** |
| `decimals()` | `0x313ce567` | `0x313ce567` | **PASS** |
| `get_dy(uint256,uint256,uint256)` (Curve SNG) | `0x556d6e9f` | `0x556d6e9f` | **PASS** |

**All 20 selectors verified correct.**

### Selector Notes

1. **`getPTAt(uint256)` (`0x6c40a4f0`)**: The comment in `get_position.rs:67` says `"getPTAddress"` but the actual Spectra Registry function is `getPTAt`. The selector value `0x6c40a4f0` is correct and confirmed working (Registry returns 69 PTs). This is a misleading code comment only — **P2**.

2. **`get_dy(uint256,uint256,uint256)` (`0x556d6e9f`)**: The comment in `swap.rs:84` incorrectly states the signature is `get_dy(int128,int128,uint256)`. Curve StableSwap NG (v7) uses `uint256` for `i`/`j` params. The code uses `0x556d6e9f` which is correct for SNG. The comment is wrong but the selector is right — **P2**.

3. **`depositIBT` signature in SKILL.md**: `SKILL.md` (and the test spec) document `depositIBT(uint256,address,address)` = `0x2a412806`. This is incorrect — that 3-param signature has selector `0xbadfc074`. The correct 4-param variant `depositIBT(uint256,address,address,uint256)` = `0x2a412806`. The code is correct (encodes 4 params with minShares); only the documentation is wrong — **P2**.

---

## Issues Found

| ID | Severity | Location | Description |
|----|----------|----------|-------------|
| I-1 | **P2** | `get_position.rs:67` | Comment says `"getPTAddress(uint256)"` but actual function is `getPTAt(uint256)`. Selector value is correct. |
| I-2 | **P2** | `swap.rs:84` | Comment says `get_dy(int128,int128,uint256) = 0x556d6e9f` which is wrong. Actual matching sig is `get_dy(uint256,uint256,uint256)`. Selector value is correct for SNG. |
| I-3 | **P2** | `skills/spectra/SKILL.md` | Documents `depositIBT(uint256,address,address)` = `0x2a412806`. Should be `depositIBT(uint256,address,address,uint256)`. Code is correct. |
| I-4 | **P2** | `onchainos.rs:93-94` | Comment says "Always uses --force=false for approve" but code passes `force=true`. Comment is wrong; behavior is correct. |
| I-5 | **P2** | `onchainos.rs:88` | `extract_tx_hash_or_err` returns `"pending"` as final fallback. While the function is correctly implemented (checks error field first), a more descriptive fallback like `"ERROR: no txHash in response"` would be safer. Non-blocking. |

No P0 (critical) or P1 (high) issues found.

---

## Overall Recommendation

**APPROVE** — The plugin is production-ready.

All core logic is correct: selectors match on-chain ABIs, `CURVE_SWAP_SNG=0x1E` is properly used for the weETH StableSwap NG pool, `--force` is applied on all write operations, `extract_tx_hash_or_err` is used throughout (no raw `unwrap_or("pending")`), and dry-run mode correctly simulates without broadcasting. The 5 issues found are all P2 (documentation/comment inaccuracies) with no impact on runtime behavior.

Optional before merge: fix the 4 misleading comments (I-1 through I-4).
