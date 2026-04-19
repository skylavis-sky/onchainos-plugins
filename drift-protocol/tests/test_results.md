# Test Results — drift-protocol Phase 3 QA

Date: 2026-04-19  
Tester: Phase 3 Tester Agent  
Plugin version: 0.1.0  
Protocol status: Paused (security incident 2026-04-01)

## Overall Result: PASS

All critical paths pass. One Minor finding noted (SKILL.md example output incomplete).

---

## L1 — Build + Lint

### Build
```
cargo clean && cargo build --release
```
Result: **PASS**  
- Zero warnings, zero errors
- Binary produced: `target/release/drift-protocol` (4.1 MB)
- All dependencies resolved cleanly

### Lint
```
plugin-store lint . (on clean tree without target/)
```
Result: **PASS** (with fix applied)

Fix applied: Added `.gitignore` with `/target/` — plugin was missing it, causing lint to flag build artifacts. After adding `.gitignore` and running on the clean source tree, lint reported: `✓ Plugin 'drift-protocol' passed all checks!`

---

## L0 — Routing Validation

See `tests/routing_test.md` for full matrix.

Result: **PASS** — 16/16 routing test cases pass.

Key checks:
- All 6 subcommands correctly registered (`get-balance`, `get-markets`, `get-funding-rates`, `place-order`, `deposit`, `cancel-order`)
- Invalid subcommand returns exit code 2 with helpful error
- Missing required args for `place-order` returns exit code 2 with field list
- All default values match SKILL.md (`--chain 501`, `--market SOL-PERP`, `--depth 10`)

---

## L2 — Read Operation Tests

### TC-R01: get-balance
```bash
./target/release/drift-protocol get-balance
```
Result: **PASS**

Output:
```json
{
  "chain": "solana",
  "note": "Drift deposits are currently paused (protocol recovery mode). These are your on-chain Solana wallet balances.",
  "ok": true,
  "sol": "0.069374067",
  "usdc": "20.709789",
  "usdt": "0",
  "wallet": "6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY"
}
```
Exit code: 0  
Verification: Correctly calls `onchainos wallet balance --chain 501` (no `--output json`). Returns real wallet address and balances. SOL + USDC present. USDT returns "0" (no USDT on this wallet).

### TC-R02: get-markets (default SOL-PERP)
```bash
./target/release/drift-protocol get-markets
```
Result: **PASS** (graceful 503 degradation)

Output (stderr, exit 1):
```json
{
  "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Track status: https://drift.trade",
  "note": "Read operations (get-markets, get-funding-rates) will return data when the protocol relaunches.",
  "ok": false
}
```
Exit code: 1  
Note: Graceful degradation — no panic, structured JSON error, actionable status link.

### TC-R03: get-markets --market BTC-PERP
```bash
./target/release/drift-protocol get-markets --market BTC-PERP
```
Result: **PASS** (graceful 503 degradation)

Output: Same paused error JSON as TC-R02. Exit code: 1.

### TC-R04: get-funding-rates (default SOL-PERP)
```bash
./target/release/drift-protocol get-funding-rates
```
Result: **PASS** (graceful degradation)

Output:
```json
{
  "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Track status: https://drift.trade",
  "note": "Read operations (get-markets, get-funding-rates) will return data when the protocol relaunches.",
  "ok": false
}
```
Exit code: 1

---

## L3 — Write Stub Tests

### TC-W01: place-order
```bash
./target/release/drift-protocol place-order --market SOL-PERP --side buy --size 0.1 --price 85.0
```
Result: **PASS**

Output (stderr, exit 1):
```json
{
  "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Trading will resume after independent security audits complete. Track status: https://drift.trade",
  "note": "When Drift relaunches with a public transaction API, this command will be fully implemented.",
  "ok": false
}
```
No panic, no crash. Structured error with forward-looking note.

### TC-W02: deposit
```bash
./target/release/drift-protocol deposit --token USDT --amount 10.0
```
Result: **PASS**

Output: Same structured paused write error as TC-W01. Exit code: 1.

Note: Test instructions contained `--dydx-address dummy` which is incorrect for this plugin. The actual `deposit` signature uses `--token <TOKEN> --amount <AMOUNT>` per SKILL.md — tested with correct args.

### TC-W03: cancel-order
```bash
./target/release/drift-protocol cancel-order --order-id 123
```
Result: **PASS**

Output: Same structured paused write error as TC-W01. Exit code: 1.

---

## L4 — On-chain Tests

**N/A** — Write operations are stubs (no transactions broadcast). `get-balance` uses onchainos read on Solana (no gas cost, read-only RPC). No on-chain execution required.

---

## Findings Summary

| ID | Severity | Category | Description |
|----|----------|----------|-------------|
| F01 | Minor | Documentation | SKILL.md `get-balance` example output missing `note` field (binary emits it; schema is correct, docs incomplete) |
| F02 | Minor | Submission | `.gitignore` was absent; added `/target/` entry to pass lint |

No Critical or Major findings.

---

## Fix Applied During Testing

**F02 fix:** Created `/Users/samsee/projects/plugin-store-dev/drift-protocol/.gitignore` with content:
```
/target/
```
This is a required submission prerequisite for all Rust plugins.

---

## L1 / L0 / L2 / L3 Summary

| Level | Result | Notes |
|-------|--------|-------|
| L1 Build | PASS | Zero warnings |
| L1 Lint | PASS | After .gitignore fix |
| L0 Routing | PASS | 16/16 cases pass |
| L2 Read | PASS | get-balance live; get-markets + get-funding-rates graceful 503 |
| L3 Stubs | PASS | All 3 write ops return structured pause error |
| L4 On-chain | N/A | No broadcasts needed |
