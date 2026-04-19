# Test Cases — drift-protocol Phase 3 QA

Generated: 2026-04-19  
Protocol status: Paused (exploit 2026-04-01)

## Tier 1 — Read Operations (L2)

### TC-R01: get-balance (no args — default chain 501)
```bash
./target/release/drift-protocol get-balance
```
Expected:
- `ok: true`
- Fields: wallet, sol, usdc, usdt, chain="solana", note
- Calls `onchainos wallet balance --chain 501` (no --output json)

### TC-R02: get-markets (default SOL-PERP)
```bash
./target/release/drift-protocol get-markets
```
Expected (protocol paused):
- `ok: false`
- `error` contains "Drift Protocol is currently paused"
- `note` mentions read operations will return data at relaunch
- Exit code 1 (error path)

### TC-R03: get-markets --market BTC-PERP
```bash
./target/release/drift-protocol get-markets --market BTC-PERP
```
Expected: Same paused error JSON as TC-R02 (different market param, same degradation path)

### TC-R04: get-funding-rates (default SOL-PERP)
```bash
./target/release/drift-protocol get-funding-rates
```
Expected (protocol paused):
- `ok: false`
- `error` contains "Drift Protocol is currently paused"
- `note` present
- Exit code 1

## Tier 2 — Write Stubs (L3)

### TC-W01: place-order
```bash
./target/release/drift-protocol place-order --market SOL-PERP --side buy --size 0.1 --price 85.0
```
Expected:
- `ok: false`
- `error` contains "Drift Protocol is currently paused"
- `note` mentions "When Drift relaunches"
- No panic, no crash
- Exit code 1

### TC-W02: deposit
```bash
./target/release/drift-protocol deposit --token USDT --amount 10.0
```
Expected: Same structured paused error as TC-W01

### TC-W03: cancel-order
```bash
./target/release/drift-protocol cancel-order --order-id 123
```
Expected: Same structured paused error as TC-W01

## L4 — On-chain Tests
N/A: Write operations are stubs (no transactions). get-balance uses onchainos read on Solana (no gas cost). No on-chain execution needed.
