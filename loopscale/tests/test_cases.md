# Loopscale Plugin — Test Cases

## L1: Read — get-vaults

**Command:**
```bash
./target/release/loopscale get-vaults
```

**Expected:**
- `ok: true`
- `data.vaults` array with at least one entry
- Each vault has `vault_address`, `token`, `tvl_display`, `apy_pct`
- Vaults sorted by TVL descending

**Variations:**
```bash
./target/release/loopscale get-vaults --token USDC
./target/release/loopscale get-vaults --token SOL
```

---

## L2: Read — get-position

**Requires onchainos login.**

```bash
./target/release/loopscale get-position
```

**Expected:**
- `ok: true`
- `data.wallet` — resolved Solana pubkey
- `data.lend_positions` — array (may be empty if no deposits)
- `data.borrow_positions` — array (may be empty if no active loans)
- `data.summary.active_loans` and `data.summary.vault_deposits` — counts

---

## L3: Write dry-run — lend

```bash
./target/release/loopscale lend --token USDC --amount 10 --dry-run
```

**Expected:**
- `ok: true`, `dry_run: true`
- `data.operation: "lend"`
- `data.token: "USDC"`, `data.amount: 10.0`
- `data.lamports: 10000000` (10 USDC = 10_000_000 lamports)
- `data.vault` — default USDC vault address

---

## L4: Write dry-run — withdraw

```bash
./target/release/loopscale withdraw --token USDC --amount 5 --dry-run
./target/release/loopscale withdraw --token SOL --all --dry-run
```

**Expected:**
- `ok: true`, `dry_run: true`
- `data.operation: "withdraw"`
- `data.lamports` — correct amount
- `data.withdraw_all: true` for `--all` variant

---

## L5: Write dry-run — borrow

```bash
./target/release/loopscale borrow \
  --principal USDC \
  --amount 50 \
  --collateral SOL \
  --collateral-amount 1 \
  --duration 7 \
  --dry-run
```

**Expected:**
- `ok: true`, `dry_run: true`
- `data.principal_token: "USDC"`, `data.collateral_token: "SOL"`
- `data.principal_lamports: 50000000`
- `data.collateral_lamports: 1000000000`
- `data.strategy` — populated from quote API
- `data.expected_apy` — e.g. `"8.50%"`
- `data.steps` — confirms two-step flow

---

## L6: Write dry-run — repay

**Requires a valid loan address from L5 (non-dry-run) or existing loan.**

```bash
./target/release/loopscale repay \
  --loan <LOAN_ADDRESS> \
  --all \
  --dry-run
```

**Expected:**
- `ok: true`, `dry_run: true`
- `data.operation: "repay"`
- `data.loan_address` — matches input
- `data.repay_all: true`
- `data.principal_due` — outstanding balance

---

## L7: Error handling — missing args

```bash
./target/release/loopscale lend --token USDC
# Missing --amount
```

**Expected:** clap error with usage message, exit code 2

```bash
./target/release/loopscale withdraw --token USDC
# Missing --amount and --all
```

**Expected:** `{"ok": false, "error": "Provide --amount <value> or --all to withdraw everything."}`

---

## Notes

- L4 live execution (borrow) is not tested in automated suite due to two-step tx requirement and Solana blockhash TTL (~60s).
- All live write tests (L3-L6 without --dry-run) require onchainos login and funded Solana wallet.
- The `get-vaults` command (L1) can be run without login — it is a public API call.
