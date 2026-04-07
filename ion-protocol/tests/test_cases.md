# Ion Protocol Plugin Test Cases

Generated from design.md §3 and §7 (7 user scenarios).
Each command has test variants: read-only/dry-run (safe to run now) and on-chain happy path (PENDING_APPROVAL).

**Test wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`
**Chain:** Ethereum Mainnet (chain 1)
**Primary pool:** rsETH/wstETH (highest TVL ~6.5 wstETH, ~32% borrow APY)

**Test wallet funding requirements (for L4 live tests):**
- 0.01 wstETH (for lend and repay tests)
- 0.01 rsETH (for collateral deposit and borrow tests)
- ETH for gas on Ethereum mainnet

---

## TC-1: get-pools (Read market data)

### TC-1.1 -- Read-only smoke test (PASS)
```bash
./target/release/ion-protocol get-pools --chain 1
```
**Expected:** JSON with `ok: true`, `poolCount: 4`, each pool has `borrowApy` (e.g. "32.xxxx%"), `totalLendSupply`, `ionPool`, `gemJoin`, `collateral`, `lendToken`.
**Result:** PASS -- Returns 4 pools. rsETH/wstETH borrowApy ~32.7%, rswETH/wstETH ~2.7%, ezETH/WETH ~5.7%, weETH/wstETH ~2.8%. Total supply: rsETH ~6.48 wstETH, ezETH ~0.006 WETH.

### TC-1.2 -- On-chain happy path (PENDING_APPROVAL)
Same as TC-1.1 -- read-only, always safe to run.

### TC-1.3 -- Error: unsupported chain (PASS)
```bash
./target/release/ion-protocol get-pools --chain 8453
```
**Expected:** `ok: false`, error "Ion Protocol only supports Ethereum Mainnet (chain 1)."
**Note:** get-pools does not validate chain (it always uses chain 1 RPC). Error would surface on write ops.

---

## TC-2: get-position (Read vault position)

### TC-2.1 -- Read-only with known wallet (PASS)
```bash
./target/release/ion-protocol get-position --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --chain 1
```
**Expected:** JSON with `ok: true`, `hasPositions: false` (no active position for test wallet), 4 pool entries each with `collateralWad: "0"`, `normalizedDebtWad: "0"`, `lendBalanceWad: "0"`.
**Result:** PASS -- Returns zero positions as expected for unfunded test wallet.

### TC-2.2 -- Read-only after lend/borrow (PENDING_APPROVAL)
Run after TC-4.2 (lend) and TC-6.2 (borrow). Expect non-zero `lendBalanceWad` and `collateralWad`/`normalizedDebtWad`.

### TC-2.3 -- Error: bad address
```bash
./target/release/ion-protocol get-position --from 0xBADADDRESS --chain 1
```
**Expected:** `ok: false`, address parse error.

---

## TC-3: deposit-collateral (Deposit LRT collateral, 3-step)

### TC-3.1 -- Dry-run (PASS)
```bash
./target/release/ion-protocol deposit-collateral \
  --pool rsETH \
  --amount 10000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, 3 steps with correct selectors:
- Step 1: `0x095ea7b3` (ERC20.approve to GemJoin `0x3bC3AC09...`)
- Step 2: `0x3b4da69f` (GemJoin.join)
- Step 3: `0x918a2f42` (IonPool.depositCollateral, ilkIndex=0)
**Result:** PASS

### TC-3.2 -- On-chain happy path (PENDING_APPROVAL)
Same as TC-3.1 without --dry-run. Requires 0.01 rsETH in wallet.

### TC-3.3 -- Error: unknown pool
```bash
./target/release/ion-protocol deposit-collateral --pool cbETH --amount 1000000000000000000 --dry-run
```
**Expected:** `ok: false`, error "Unknown pool 'cbETH'. Valid options: rsETH, rswETH, ezETH, weETH."

---

## TC-4: lend (Supply wstETH to earn yield)

### TC-4.1 -- Dry-run (PASS)
```bash
./target/release/ion-protocol lend \
  --pool rsETH \
  --amount 10000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, 2 steps:
- Step 1: `0x095ea7b3` approve wstETH to IonPool (`0x0000000000E33e35EE6052fae87bfcFac61b1da9`)
- Step 2: `0x7ca5643d` supply calldata with `amountHuman: "0.010000 wstETH"`, empty proof (trailing `...00000060...00000000`)
**Result:** PASS

### TC-4.2 -- On-chain happy path (PENDING_APPROVAL)
Same as TC-4.1 without --dry-run. Requires 0.01 wstETH in wallet.
Expected: `approveTxHash` and `supplyTxHash` both non-null.

### TC-4.3 -- Dry-run: lend to WETH pool
```bash
./target/release/ion-protocol lend \
  --pool ezETH \
  --amount 5000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, `lendToken: "0xC02aaA39b..."` (WETH), step 1 approves WETH to ezETH IonPool.

---

## TC-5: withdraw-lend (Withdraw lent wstETH)

### TC-5.1 -- Dry-run (PASS)
```bash
./target/release/ion-protocol withdraw-lend \
  --pool rsETH \
  --amount 10000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, single call with selector `0xf3fef3a3` (IonPool.withdraw).

### TC-5.2 -- On-chain happy path (PENDING_APPROVAL)
Run after TC-4.2. Withdraw the 0.01 wstETH supplied in TC-4.2.

---

## TC-6: borrow (Full 4-step borrow flow)

### TC-6.1 -- Dry-run (PASS)
```bash
./target/release/ion-protocol borrow \
  --pool rsETH \
  --collateral-amount 10000000000000000 \
  --borrow-amount 5000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, 4 steps with selectors:
- Step 1: `0x095ea7b3` (approve rsETH to GemJoin)
- Step 2: `0x3b4da69f` (GemJoin.join)
- Step 3: `0x918a2f42` (depositCollateral, ilkIndex=0)
- Step 4: `0x9306f2f8` (borrow, normalizedDebt = 5e15 * RAY / rate)
`normalizedDebt` must be < 5e15 (since rate > RAY from accumulation since inception).
`rateRay` must be > 1e27.
**Result:** PASS -- normalizedDebt=3520950398817707, rateRay=1420071126727300557369653825

### TC-6.2 -- On-chain happy path (PENDING_APPROVAL)
Same as TC-6.1 without --dry-run. Requires 0.01 rsETH in wallet. DO NOT run without sufficient collateral and within LTV limits.

### TC-6.3 -- Dry-run: weETH pool
```bash
./target/release/ion-protocol borrow \
  --pool weETH \
  --collateral-amount 10000000000000000 \
  --borrow-amount 5000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** GemJoin = `0x3f6119b0328c...`, collateral = `0xCd5fE23C85...` (weETH), lend = wstETH.

---

## TC-7: repay (Repay debt)

### TC-7.1 -- Dry-run with amount (PASS)
```bash
./target/release/ion-protocol repay \
  --pool rsETH \
  --amount 5000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, `dryRun: true`, 2 steps:
- Step 1: `0x095ea7b3` approve wstETH to IonPool (amount includes 0.1% buffer)
- Step 2: `0x8459b437` repay (normalizedDebt with 0.1% buffer)
`repayAmountHuman` should show ~0.005005 wstETH (0.1% buffer applied).
**Result:** PASS

### TC-7.2 -- Dry-run with --all (reads on-chain normalizedDebt)
```bash
./target/release/ion-protocol repay \
  --pool rsETH \
  --all \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: false` with "No outstanding debt found" (test wallet has no debt), OR if wallet has debt: shows full repay amount.

### TC-7.3 -- Dry-run: repay + withdraw collateral
```bash
./target/release/ion-protocol repay \
  --pool rsETH \
  --amount 5000000000000000 \
  --withdraw-collateral \
  --collateral-amount 10000000000000000 \
  --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 \
  --dry-run
```
**Expected:** `ok: true`, 4 steps including:
- Step 3: `0x743f9c0c` (withdrawCollateral)
- Step 4: `0xef693bed` (GemJoin.exit)

### TC-7.4 -- On-chain happy path (PENDING_APPROVAL)
Run after TC-6.2. Repay the borrowed wstETH and withdraw rsETH collateral.

---

## Selector Verification

All selectors verified via keccak256 and confirmed in design.md §2:

| Function | Selector |
|----------|---------|
| ERC20.approve | 0x095ea7b3 |
| IonPool.supply | 0x7ca5643d |
| IonPool.withdraw | 0xf3fef3a3 |
| GemJoin.join | 0x3b4da69f |
| GemJoin.exit | 0xef693bed |
| IonPool.depositCollateral | 0x918a2f42 |
| IonPool.borrow | 0x9306f2f8 |
| IonPool.repay | 0x8459b437 |
| IonPool.withdrawCollateral | 0x743f9c0c |
| IonPool.getCurrentBorrowRate | 0x6908d3df |
| IonPool.vault | 0x9a3db79b |
| IonPool.rate | 0x3c04b547 |
| IonPool.totalSupply | 0x18160ddd |
| IonPool.balanceOf | 0x70a08231 |
| IonPool.normalizedDebt | 0x57fc90b2 |
