# Gearbox V3 Plugin — Test Results

**Date:** 2026-04-07  
**Plugin:** `gearbox-v3` v0.1.0  
**Chain tested:** Arbitrum One (42161)  
**Tester:** Claude Code automated test runner

---

## Summary

| Level | Status | Notes |
|-------|--------|-------|
| L1 — Build | **PASS** | Zero errors, already compiled |
| L2 — Read Operations | **PARTIAL** | `get-pools` PASS; `get-account` FAIL |
| L3 — Dry-Run | **PASS** (with correct flags) | All selectors verified; spec flags differ from actual CLI |
| L4 — Live On-Chain | **SKIPPED** | No funded credit account available |
| Static Analysis | **PASS** | All 7 selectors correct; `extract_tx_hash_or_err` pattern sound; no CJK |

**Overall Recommendation:** CONDITIONAL PASS — The core ABI encoding and dry-run logic is correct and all selector checks pass. Two issues must be fixed before production: (1) `get-account` error handling, (2) flag-name documentation misalignment.

---

## L1 — Build

**Result: PASS**

```
cargo build --release
Finished `release` profile [optimized] target(s) in 0.07s
```

Zero errors. Binary located at `target/release/gearbox-v3`.

---

## L2 — Read Operations

### L2.1 — `get-pools --chain 42161`

**Result: PASS**

Returned 6 Credit Managers with live `debtLimits()` RPC data (not hardcoded). All USDC, USDC.e, and WETH tiers present:

| Name | CreditFacade | CreditManager | minDebt | maxDebt |
|------|-------------|---------------|---------|---------|
| Trade USDC Tier 2 | 0x39748885... | 0xb780dd9c... | 1000 USDC | 20000 USDC |
| Trade USDC Tier 1 | 0xbe0715ec... | 0xe5e2d4bb... | 20000 USDC | 400000 USDC |
| Trade USDC.e Tier 2 | 0x8d5d92d4... | 0xb4bc02c0... | 5000 USDC.e | 25000 USDC.e |
| Trade USDC.e Tier 1 | 0x026329e9... | 0x75bc0fef... | 5000 USDC.e | 100000 USDC.e |
| Trade WETH Tier 2 | 0xf1fada02... | 0x3ab1d355... | 0.35 WETH | 7 WETH |
| Trade WETH Tier 1 | 0x7d4a58b2... | 0xcedaa4b4... | 7 WETH | 150 WETH |

Confirmed: values sourced via live `debtLimits()` RPC calls (the SKILL.md table values match actuals). ✓

### L2.2 — `get-account --chain 42161 --from 0x742d35Cc...`

**Result: FAIL (P1 Bug)**

```json
{
  "error": "getCreditAccountsByBorrower eth_call failed",
  "ok": false
}
```

**Root cause:** The DataCompressor `getCreditAccountsByBorrower` RPC call returns `execution reverted` for any address with no open Credit Accounts (and/or due to Chainlink price feed validation requirements in the DataCompressor). The plugin propagates the revert as a hard error instead of returning an empty accounts list.

**Expected behavior:** Return `{ "ok": true, "creditAccounts": [], "message": "No open Credit Accounts found." }`

**Confirmed:** The selector `0x16e5b9f1` is correct (`getCreditAccountsByBorrower(address,(address,uint256,bytes)[])`). The issue is error handling, not ABI encoding.

**Note:** The `--from` flag is the correct flag (spec used `--address` which doesn't exist on this CLI — see P2 finding below).

---

## L3 — Dry-Run

Tested with correct CLI flags (spec flags differ — see P2 below).

### L3.1 — `open-account --dry-run`

**Result: PASS**

```bash
./target/release/gearbox-v3 --dry-run open-account \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --token USDC --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --collateral 1000 --borrow 2000
```

Verified:
- Step 1 (approve): selector `0x095ea7b3` ✓; spender = CreditManagerV3 `0xb780dd9c...` ✓ (not facade)
- Step 2 (openCreditAccount): selector `0x92beab1d` ✓
- Inner call order: `increaseDebt` at offset 584 BEFORE `addCollateral` at offset 904 ✓
- `increaseDebt` inner selector `2b7c7b11` ✓
- `addCollateral` inner selector `6d75b9ee` ✓
- USDC amount 1000 → raw `1000000000` (6 decimals) ✓
- USDC borrow 2000 → raw `2000000000` ✓

### L3.2 — `close-account --dry-run`

**Result: PASS**

```bash
./target/release/gearbox-v3 --dry-run close-account \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --account 0x1234567890123456789012345678901234567890 \
  --underlying 0xaf88d065e77c8cC2239327C5EDb3A432268e5831
```

Verified:
- Outer selector `0x36b2ced3` (closeCreditAccount) ✓
- Inner `decreaseDebt` selector `2a7ba1f7` ✓; value = `u128::MAX` (`ffffffffffffffffffffffffffffffff`) ✓
- Inner `withdrawCollateral` selector `1f1088a0` ✓; amount = `u128::MAX` ✓
- recipient defaults to zero address when `--from` not specified (expected in dry-run) ✓

### L3.3 — `add-collateral --dry-run`

**Result: PASS**

```bash
./target/release/gearbox-v3 --dry-run add-collateral \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --account 0x1234567890123456789012345678901234567890 \
  --token USDC --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --amount 500
```

Verified:
- Step 1 (approve): selector `0x095ea7b3` ✓; spender = CreditManagerV3 (not facade) ✓; amount 500 USDC = `1dcd6500` ✓
- Step 2 (multicall): selector `0xebe4107c` ✓
- Inner `addCollateral` selector `6d75b9ee` ✓

### --dry-run Flag Positioning

**Result: PASS**

Both `--dry-run BEFORE subcommand` and `--dry-run AFTER subcommand` produce identical output. Defined as `global=true` in Clap. ✓

---

## L4 — Live On-Chain

**Result: SKIPPED**

No funded Gearbox Credit Account available on Arbitrum during test execution. Additionally, `get-account` is broken (P1 bug above), which would be required to confirm account state after opening.

---

## Static Analysis

### ABI Selector Table

| Function Signature | Expected Selector | Code Selector | Match |
|-------------------|------------------|---------------|-------|
| `openCreditAccount(address,(address,bytes)[],uint256)` | `0x92beab1d` | `0x92beab1d` | ✓ PASS |
| `closeCreditAccount(address,(address,bytes)[])` | `0x36b2ced3` | `0x36b2ced3` | ✓ PASS |
| `multicall(address,(address,bytes)[])` | `0xebe4107c` | `0xebe4107c` | ✓ PASS |
| `increaseDebt(uint256)` | `0x2b7c7b11` | `0x2b7c7b11` | ✓ PASS |
| `addCollateral(address,uint256)` | `0x6d75b9ee` | `0x6d75b9ee` | ✓ PASS |
| `decreaseDebt(uint256)` | `0x2a7ba1f7` | `0x2a7ba1f7` | ✓ PASS |
| `withdrawCollateral(address,uint256,address)` | `0x1f1088a0` | `0x1f1088a0` | ✓ PASS |
| `approve(address,uint256)` (ERC-20) | `0x095ea7b3` | `0x095ea7b3` | ✓ PASS |
| `debtLimits()` | `0x166bf9d9` | `0x166bf9d9` | ✓ PASS |

All 9 selectors verified correct via Python keccak256.

### Key Design Checks

| Check | Result |
|-------|--------|
| Approve target = CreditManagerV3 (not CreditFacadeV3) | ✓ PASS |
| `increaseDebt` BEFORE `addCollateral` in openCreditAccount | ✓ PASS |
| `get-pools` uses live `debtLimits()` RPC calls | ✓ PASS |
| `extract_tx_hash_or_err` checks `ok`, `error`, `message`, `data.txHash`, `txHash` | ✓ PASS |
| No CJK in SKILL.md description | ✓ PASS |
| `--dry-run` global flag works before and after subcommand | ✓ PASS |
| `decreaseDebt(u128::MAX)` for full repayment | ✓ PASS |
| `withdrawCollateral(token, u128::MAX, recipient)` for full withdrawal | ✓ PASS |

---

## Issues Found

### P1 — `get-account` fails with RPC revert instead of returning empty list

**Severity:** P1 (functional failure for any address with no open accounts)  
**File:** `src/commands/get_account.rs`, line 46  
**Description:** `eth_call` to `DataCompressor.getCreditAccountsByBorrower()` returns `execution reverted` for addresses with no open Credit Accounts. The plugin does not catch this revert and instead exits with `ok: false`. The fix is to catch the `eth_call` error in `get_account::run()` and return an empty accounts list.

**Fix:**
```rust
let result_hex = match eth_call(rpc, dc, &calldata_hex).await {
    Ok(hex) => hex,
    Err(_) => {
        // DataCompressor reverts when no accounts exist
        return Ok(json!({
            "ok": true,
            "chain": chain_id,
            "borrower": borrower,
            "creditAccounts": [],
            "message": "No open Credit Accounts found."
        }));
    }
};
```

### P2 — CLI flag names differ from test spec / SKILL.md examples

**Severity:** P2 (documentation/usability)  
**Description:** The L3 test spec and `get-account` L2 spec use flag names that do not exist on this CLI:

| Spec / Docs | Actual CLI |
|------------|-----------|
| `--credit-manager` | `--manager` |
| `--collateral-amount` | `--collateral` |
| `--borrow-amount` | `--borrow` |
| `--address` (get-account) | `--from` |

The spec's `open-account` example uses `--credit-manager`, `--collateral`, `--collateral-amount`, `--borrow-amount` — running the spec commands verbatim produces `error: unexpected argument`. The SKILL.md routing table correctly uses `--facade`, `--manager`, `--collateral`, `--borrow` which do work.

**Impact:** Confusion when test spec and actual CLI differ; users following the spec literally will get parse errors.

### P3 — `get-account` requires `--from` but spec describes it as using `--address`

**Severity:** P3 (minor documentation gap)  
**File:** `src/commands/get_account.rs`, `src/main.rs`  
**Description:** `get-account` uses the global `--from` flag to pass the borrower address, while docs and test spec refer to `--address`. This is a naming inconsistency. The current behavior is consistent with other commands (all use `--from` for the wallet address), but a dedicated `--address` or `--borrower` parameter for `get-account` would be less confusing since it may differ from the active wallet.

---

## Conclusion

The plugin's ABI encoding logic is correct. All 9 function selectors match keccak256 values. The dry-run outputs for all three write operations produce well-formed calldata with correct inner-call ordering and proper approve targets. The `get-pools` command successfully fetches live on-chain debt limits.

The blocking issue is `get-account` (P1): it fails hard on RPC revert instead of gracefully returning an empty account list. This should be fixed before release.

The flag name misalignment (P2) between the test spec and actual CLI is a documentation/interface issue that causes confusion but does not affect functional behavior when using the correct flags.

**Recommendation:** Fix P1 (5-line code change), update spec/README flags to match actual CLI (P2), then re-test `get-account` — at that point the plugin is ready for release.
