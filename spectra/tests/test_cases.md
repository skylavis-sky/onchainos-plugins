# Spectra Plugin Test Cases

All tests use Base (chain 8453). Test wallet: `0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045` (vitalik.eth — no real funds needed for dry-run).
weETH PT: `0x07f58450a39d07f9583c188a2a4a441fac358100`

---

## TC-001: get-pools — Basic Pool List

**Command:**
```bash
spectra --chain 8453 get-pools --active-only --limit 5
```

**Expected:**
- `ok: true`
- `chain_id: 8453`
- At least 1 pool in `pools[]`
- `total_registered_pts` is 69 (or current live value, verified from Registry `pTCount()`)
- weETH pool present with `active: true`, `days_to_maturity > 0`
- Pool has `pt`, `yt`, `ibt`, `curve_pool` addresses

**Pass criteria:** `ok == true`, `pools.length >= 1`, registry call succeeds (not empty)

---

## TC-002: get-pools — All Pools (No Filter)

**Command:**
```bash
spectra --chain 8453 get-pools --limit 20
```

**Expected:**
- Returns all known pools including any expired ones
- `active` field correctly reflects whether `maturity_ts < now`

---

## TC-003: get-position — Empty Wallet

**Command:**
```bash
spectra --chain 8453 get-position --user 0x000000000000000000000000000000000000dead
```

**Expected:**
- `ok: true`
- `position_count: 0`
- `positions: []`

---

## TC-004: deposit — Dry-Run (weETH, 0.01 WETH)

**Command:**
```bash
spectra --chain 8453 --dry-run deposit \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --amount 10000000000000000 \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `ok: true`, `dry_run: true`
- `operation: "deposit"`
- `token_approved: "0x4200000000000000000000000000000000000006"` (WETH)
- `calldata` starts with `0xe4cca4b0` (deposit selector)
- `estimated_pt_shares` ≈ `9999999999999999` (near 1:1 for WETH pool)
- `min_pt_shares` = estimated * 0.995 (0.5% slippage)
- `approve_tx` and `tx_hash` are zero hash (dry-run)

---

## TC-005: deposit — IBT Mode Dry-Run

**Command:**
```bash
spectra --chain 8453 --dry-run deposit \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --amount 10000000000000000 \
  --use-ibt \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `operation: "depositIBT"`
- `token_approved` = IBT address `0x22f757c0b434d93c93d9653f26c9441d8d06c8ec` (sw-weETH)
- `calldata` starts with `0x2a412806` (depositIBT selector)

---

## TC-006: deposit — Post-Expiry Error

**Setup:** Use an expired PT (maturity < now). If no expired PT available, mock by passing a PT whose `maturity()` returns a past timestamp.

**Expected:**
- Returns error: `"PT has expired"` with exit code 1

---

## TC-007: redeem — Pre-Expiry Dry-Run (withdraw)

**Command:**
```bash
spectra --chain 8453 --dry-run redeem \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --shares 9999999999999999 \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `ok: true`, `dry_run: true`
- `expired: false` (weETH PT matures Jul 2026, currently active)
- `operation` contains `"pre-expiry"` and `"withdraw"`
- `calldata` starts with `0xb460af94` (withdraw selector)
- Warning note: requires equal YT balance

---

## TC-008: redeem — Post-Expiry Calldata Verification

**Setup:** Manually set current time past maturity (or use a PT that has already expired).

**Expected:**
- `expired: true`
- `calldata` starts with `0x9f40a7b3` (redeem selector)
- Output includes `estimated_underlying_out`

---

## TC-009: claim-yield — Dry-Run (No Pending Yield)

**Command:**
```bash
spectra --chain 8453 --dry-run claim-yield \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `ok: true`
- `pending_yield_ibt_raw: "0"` (vitalik.eth holds no YT for this pool)
- `calldata` starts with `0x999927df` (claimYield selector)
- `operation: "claimYield"`

---

## TC-010: claim-yield — In-IBT Mode

**Command:**
```bash
spectra --chain 8453 --dry-run claim-yield \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --in-ibt \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `operation: "claimYieldInIBT"`
- `calldata` starts with `0x0fba731e` (claimYieldInIBT selector)

---

## TC-011: swap — Sell PT Dry-Run

**Command:**
```bash
spectra --chain 8453 --dry-run swap \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --amount-in 9999999999999999 \
  --sell-pt \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `ok: true`, `dry_run: true`
- `operation: "swap (sell PT -> IBT)"`
- `token_in` = PT address
- `router` = `0xc03309de321a4d3df734f5609b80cc731ae28e6d`
- `calldata` starts with `0x24856bc3` (execute selector)
- `calldata` contains `001e` bytes (TRANSFER_FROM=0x00, CURVE_SWAP_SNG=0x1e)
- `min_amount_out_raw` ≈ amount * 0.99 (1% slippage)

---

## TC-012: swap — Buy PT Dry-Run

**Command:**
```bash
spectra --chain 8453 --dry-run swap \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --amount-in 10000000000000000 \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- `operation: "swap (buy PT, sell IBT)"`
- `token_in` = IBT address (`0x22f757c0b434d93c93d9653f26c9441d8d06c8ec`)

---

## TC-013: swap — Unknown Pool Error

**Command:**
```bash
spectra --chain 8453 --dry-run swap \
  --pt 0x1111111111111111111111111111111111111111 \
  --amount-in 1000000 \
  --sell-pt \
  --from 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045
```

**Expected:**
- Error: `"No known Curve pool for PT 0x1111..."` (exit 1)

---

## TC-014: Registry Smoke Test (pTCount)

Verified live during development via direct RPC call:

```
curl https://base-rpc.publicnode.com \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"eth_call","params":[{"to":"0x786da12e9836a9ff9b7d92e8bac1c849e2ace378","data":"0x704bdadc"},"latest"],"id":1}'
```

**Result:** `0x0000...0045` → 69 PTs registered on Base. Registry is live.

---

## TC-015: Selector Correctness

All selectors verified against 4byte.directory during development:

| Function | Selector | Verified |
|----------|----------|---------|
| `deposit(uint256,address,address,uint256)` | `0xe4cca4b0` | OK |
| `depositIBT(uint256,address,address,uint256)` | `0x2a412806` | OK |
| `redeem(uint256,address,address,uint256)` | `0x9f40a7b3` | OK |
| `withdraw(uint256,address,address)` | `0xb460af94` | OK |
| `claimYield(address)` | `0x999927df` | OK |
| `claimYieldInIBT(address)` | `0x0fba731e` | OK |
| `execute(bytes,bytes[])` | `0x24856bc3` | OK |
| `previewDeposit(uint256)` | `0xef8b30f7` | OK |
| `previewRedeem(uint256)` | `0x4cdad506` | OK |
| `maturity()` | `0x204f83f9` | OK |
| `getIBT()` | `0xc644fe94` | OK |
| `getYT()` | `0x04aa50ad` | OK |
| `underlying()` | `0x6f307dc3` | OK |
| `pTCount()` | `0x704bdadc` | OK |
| `getPTAt(uint256)` | `0x6c40a4f0` | OK |
| `getCurrentYieldOfUserInIBT(address)` | `0x0e1b6d89` | OK |

Router Command bytes (from Commands.sol):
- `TRANSFER_FROM` = `0x00`
- `CURVE_SWAP_SNG` = `0x1E` (Curve StableSwap NG pool — weETH pool on Base is v7.0.0 SNG)
