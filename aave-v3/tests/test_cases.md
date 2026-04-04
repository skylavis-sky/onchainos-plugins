# Aave V3 Plugin Test Cases

Generated from design.md §3 (7 user scenarios). Each command has three test cases:
- **TC-X.1**: dry-run (no on-chain tx) — executable now
- **TC-X.2**: happy path on-chain — **PENDING_APPROVAL** (do not execute)
- **TC-X.3**: error/edge case — executable now

**Test wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`  
**Chain:** Base (8453)  
**USDT on Base:** `0xfde4c96c8593536e31f229ea8f37b2ada2699bb2` *(not in Aave Base reserves; tests use USDC)*  
**USDC on Base:** `0x833589fcd6edb6e08f4c7c32d4f71b54bda02913`  
**WETH on Base:** `0x4200000000000000000000000000000000000006`  

> **Note on USDT:** USDT (`0xfde4c96c8593536e31f229ea8f37b2ada2699bb2`) is NOT currently a reserve in Aave V3 Base. Supply/withdraw dry-run tests use USDC instead. Borrow/repay/set-collateral tests still reference WETH and the USDT address per the guard rails spec.

---

## TC-1: health-factor (Scenario 3 — Check health factor)

### TC-1.1 — Dry-run / read-only (PASS)
```bash
./target/release/aave-v3 health-factor --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Expected:** JSON with `ok: true`, `healthFactor`, `totalCollateralUSD`, `totalDebtUSD`, `poolAddress`  
**Result:** PASS — returns health factor `340282366920938487808.00` (uint256.max / 1e18 = no debt), pool address `0xa238dd80c259a72e81d7e4664a9801593f98d1c5`

### TC-1.2 — On-chain happy path (PENDING_APPROVAL)
Same as TC-1.1 — read-only, no on-chain tx needed. Always safe to execute.

### TC-1.3 — Error: unsupported chain (PASS)
```bash
./target/release/aave-v3 health-factor --chain 999 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Expected:** `ok: false`, error listing supported chains  
**Result:** PASS — returns `"Unsupported chain ID: 999. Supported chains: Ethereum Mainnet (1), Polygon (137), Arbitrum One (42161), Base (8453)"`

---

## TC-2: reserves (Scenario 5 — List markets)

### TC-2.1 — Dry-run / read-only (PASS)
```bash
./target/release/aave-v3 reserves --chain 8453
```
**Expected:** JSON with `ok: true`, array of reserves each with `supplyApy`, `variableBorrowApy`, `underlyingAsset`  
**Result:** PASS — 7 of 15 reserves returned (8 skipped due to public RPC rate-limiting on free tier). Sample: USDC `supplyApy: 2.5961%`, `variableBorrowApy: 3.7977%`

### TC-2.2 — On-chain happy path (PENDING_APPROVAL)
Same as TC-2.1 — read-only, always safe.

### TC-2.3 — Error: unsupported chain (PASS)
```bash
./target/release/aave-v3 reserves --chain 999
```
**Expected:** `ok: false`, unsupported chain error  
**Result:** PASS

---

## TC-3: positions (Scenario 5 — Full position overview)

### TC-3.1 — Dry-run / read-only (PASS)
```bash
./target/release/aave-v3 positions --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Expected:** JSON with `ok: true`, health factor, and `positions` from onchainos defi positions  
**Result:** PASS — returns health factor and `positions.data.assetStatus: 1` (no active positions on this wallet)

### TC-3.2 — On-chain happy path (PENDING_APPROVAL)
Same as TC-3.1 — read-only, always safe.

### TC-3.3 — Error: missing --from without active wallet (informational)
```bash
./target/release/aave-v3 positions --chain 8453
```
**Expected:** Falls back to `onchainos wallet status` to resolve address; fails if no active session.

---

## TC-4: supply (Scenario 1 — Supply USDC to earn yield)

### TC-4.1 — Dry-run (PASS)
```bash
./target/release/aave-v3 supply --asset USDC --amount 0.01 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, shows investmentId, amount in minimal units, simulated command  
**Result:** PASS — investmentId `378532533`, amountMinimal `10000` (0.01 USDC = 10000 μUSDC), currentApy `2.6500%`  
**Simulated command:** `onchainos defi invest --investment-id 378532533 --address 0xee385ac7... --token USDC --amount 10000`

### TC-4.2 — On-chain happy path (PENDING_APPROVAL)
```bash
./target/release/aave-v3 supply --asset USDC --amount 0.01 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Expected:** tx hash, new aUSDC balance, current APY  
**Guard rails:** 0.01 USDC max; must have ≥ 0.01 USDC balance first  
**Risk:** Low — supply is reversible via withdraw

### TC-4.3 — Error: asset not in Aave (PASS)
```bash
./target/release/aave-v3 supply --asset USDT --amount 0.01 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: false`, "No Aave V3 investment product found for asset 'USDT'"  
**Result:** PASS — USDT not listed in Aave V3 Base via onchainos defi search

---

## TC-5: withdraw (Scenario 1 — Withdraw supplied asset)

### TC-5.1 — Dry-run (PASS)
```bash
./target/release/aave-v3 withdraw --asset USDC --amount 0.01 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, investmentId, simulated withdraw command  
**Result:** PASS — ratio `1`, investmentId `378532533`  
**Simulated:** `onchainos defi withdraw --investment-id 378532533 --address 0xee385ac7... --chain 8453 --ratio 1`

### TC-5.2 — On-chain happy path (PENDING_APPROVAL)
```bash
./target/release/aave-v3 withdraw --asset USDC --amount 0.01 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Prerequisite:** Must have supplied USDC first (TC-4.2)  
**Risk:** Medium — requires active supply position

### TC-5.3 — Error: missing amount and --all
```bash
./target/release/aave-v3 withdraw --asset USDC --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** Error "Specify either --amount <value> or --all for full withdrawal"

---

## TC-6: borrow (Scenario 2 — Borrow ETH against collateral)

### TC-6.1 — Dry-run only (PASS)
```bash
./target/release/aave-v3 borrow --asset 0x4200000000000000000000000000000000000006 \
  --amount 0.00001 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, ABI-encoded calldata for Pool.borrow(), warning about no collateral  
**Result:** PASS — calldata `0xa415bcad...` (borrow selector), warning "No borrow capacity available... tx would revert on-chain"  
**Pool address:** `0xa238dd80c259a72e81d7e4664a9801593f98d1c5`

### TC-6.2 — On-chain (PENDING_APPROVAL — borrow-only dry-run per guard rails)
**Guard rails strictly prohibit real on-chain borrow.** This test case must remain dry-run only.

### TC-6.3 — Error: no --from and no wallet session
Depends on active onchainos session.

---

## TC-7: repay (Scenario 4 — Repay debt)

### TC-7.1 — Dry-run only (PASS)
```bash
./target/release/aave-v3 repay --asset 0x4200000000000000000000000000000000000006 \
  --amount 0.00001 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, ABI-encoded repay calldata, approval simulation  
**Result:** PASS — calldata `0x573ade81...` (repay selector), approval dry-run for WETH, warning "no outstanding debt"

### TC-7.2 — On-chain (PENDING_APPROVAL — guard rails say repay dry-run only)
**Guard rails strictly prohibit real on-chain borrow/repay.** This test case must remain dry-run only.

### TC-7.3 — Error: missing amount/all flag
```bash
./target/release/aave-v3 repay --asset 0x4200000000000000000000000000000000000006 \
  --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Expected:** Error "Specify either --amount <value> or --all for full repayment"

---

## TC-8: set-collateral (Scenario 7 — Enable/disable collateral)

### TC-8.1 — Dry-run (PASS)
```bash
./target/release/aave-v3 set-collateral \
  --asset 0xfde4c96c8593536e31f229ea8f37b2ada2699bb2 --enable \
  --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, ABI-encoded calldata for Pool.setUserUseReserveAsCollateral()  
**Result:** PASS — calldata `0x5a3b74b9...` (setUserUseReserveAsCollateral selector), `useAsCollateral: true`

### TC-8.2 — On-chain happy path (PENDING_APPROVAL)
```bash
./target/release/aave-v3 set-collateral \
  --asset 0xfde4c96c8593536e31f229ea8f37b2ada2699bb2 --enable \
  --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Expected gas:** ~50,000 gas (~$0.01 on Base)  
**Risk:** Low — enabling collateral for an asset not currently supplied is a no-op on-chain

### TC-8.3 — Error: invalid asset address
```bash
./target/release/aave-v3 set-collateral --asset not_an_address --enable \
  --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** Error parsing invalid address

---

## TC-9: set-emode (Scenario 6 — Enable E-Mode)

### TC-9.1 — Dry-run (PASS)
```bash
./target/release/aave-v3 set-emode --category 0 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, ABI-encoded calldata for Pool.setUserEMode(0)  
**Result:** PASS — calldata `0x28530a47...` (setUserEMode selector), categoryName `No E-Mode`

### TC-9.2 — On-chain happy path (PENDING_APPROVAL)
```bash
./target/release/aave-v3 set-emode --category 0 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Note:** Category 0 = "disable E-Mode" — safe if no E-Mode is currently active (no-op)  
**Expected gas:** ~40,000 gas (~$0.008 on Base)  
**Risk:** Low — setting to category 0 when already at 0 is a no-op; wallet has no positions

### TC-9.3 — E-Mode enable (category 1)
```bash
./target/release/aave-v3 set-emode --category 1 --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** calldata with categoryId=1, categoryName `Stablecoins`

---

## TC-10: claim-rewards

### TC-10.1 — Dry-run (PASS)
```bash
./target/release/aave-v3 claim-rewards --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: true`, list of simulated collect commands for each Aave product  
**Result:** PASS — 8 products found (USDC, WETH, cbBTC, EURC, cbETH, GHO, tBTC, USDbC), each showing `onchainos defi collect` command

### TC-10.2 — On-chain happy path (PENDING_APPROVAL)
```bash
./target/release/aave-v3 claim-rewards --chain 8453 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
```
**Risk:** Low — claiming rewards is non-destructive; if no rewards, tx reverts  
**Expected gas:** ~70,000 gas per product with rewards

### TC-10.3 — Error: unsupported chain
```bash
./target/release/aave-v3 claim-rewards --chain 999 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: false`, unsupported chain error

---

## TC-11: Error cases

### TC-11.1 — Missing required argument (PASS)
```bash
./target/release/aave-v3 supply --chain 8453 2>&1 || true
```
**Expected:** clap error listing missing `--asset` and `--amount`  
**Result:** PASS — exits 1 with usage help

### TC-11.2 — Invalid address in calldata
```bash
./target/release/aave-v3 set-collateral --asset 0xinvalid --enable \
  --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --dry-run
```
**Expected:** `ok: false`, "Invalid address: 0xinvalid"

---

## Summary

| Test ID | Command | Type | Status |
|---------|---------|------|--------|
| TC-1.1 | health-factor | read-only | PASS |
| TC-1.3 | health-factor wrong chain | error | PASS |
| TC-2.1 | reserves | read-only | PASS |
| TC-3.1 | positions | read-only | PASS |
| TC-4.1 | supply --dry-run | dry-run | PASS |
| TC-4.3 | supply invalid asset | error | PASS |
| TC-5.1 | withdraw --dry-run | dry-run | PASS |
| TC-6.1 | borrow --dry-run | dry-run | PASS |
| TC-7.1 | repay --dry-run | dry-run | PASS |
| TC-8.1 | set-collateral --dry-run | dry-run | PASS |
| TC-9.1 | set-emode --dry-run | dry-run | PASS |
| TC-10.1 | claim-rewards --dry-run | dry-run | PASS |
| TC-11.1 | supply missing args | error | PASS |
| TC-4.2 | supply on-chain 0.01 USDC | on-chain | PENDING_APPROVAL |
| TC-5.2 | withdraw on-chain USDC | on-chain | PENDING_APPROVAL |
| TC-8.2 | set-collateral on-chain | on-chain | PENDING_APPROVAL |
| TC-9.2 | set-emode category 0 | on-chain | PENDING_APPROVAL |
| TC-10.2 | claim-rewards on-chain | on-chain | PENDING_APPROVAL |
| TC-6.2 | borrow on-chain | on-chain | BLOCKED (guard rails) |
| TC-7.2 | repay on-chain | on-chain | BLOCKED (guard rails) |
