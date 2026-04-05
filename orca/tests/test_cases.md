# Orca Plugin — Test Cases

## L1: Compilation Tests

### T-L1-01: Debug build
```bash
cd ~/projects/plugin-store-dev/orca
~/.cargo/bin/cargo build
```
Expected: `Finished dev profile` — no errors, only dead-code warnings acceptable.

### T-L1-02: Release build
```bash
~/.cargo/bin/cargo build --release
```
Expected: `Finished release profile` — binary at `target/release/orca`.

### T-L1-03: Plugin lint
```bash
~/.cargo/bin/cargo clean && ~/.cargo/bin/plugin-store lint .
```
Expected: `Plugin 'orca' passed all checks!` — 0 errors, 0 warnings.

---

## L2: Mock / Offline Tests

### T-L2-01: get-pools help
```bash
./target/release/orca get-pools --help
```
Expected: Help text showing `--token-a`, `--token-b`, `--min-tvl`, `--include-low-liquidity`.

### T-L2-02: get-quote help
```bash
./target/release/orca get-quote --help
```
Expected: Help text showing `--from-token`, `--to-token`, `--amount`, `--slippage-bps`, `--pool`.

### T-L2-03: swap help
```bash
./target/release/orca swap --help
```
Expected: Help text showing `--from-token`, `--to-token`, `--amount`, `--slippage-bps`, `--skip-security-check`.

### T-L2-04: swap dry-run
```bash
./target/release/orca --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5
```
Expected: JSON with `"ok": true, "dry_run": true` — no onchainos call, no network call.

### T-L2-05: missing required arg
```bash
./target/release/orca get-pools --token-a So11111111111111111111111111111111111111112
```
Expected: CLI error `error: the following required arguments were not provided: --token-b`.

---

## L3: Live API / Chain Tests (read-only)

### T-L3-01: get-pools SOL/USDC
```bash
./target/release/orca get-pools \
  --token-a So11111111111111111111111111111111111111112 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```
Expected:
- `ok: true`
- `pools_found >= 1`
- Each pool has `address`, `fee_rate_pct`, `tvl_usd > 10000`, `price > 0`
- Pools sorted by TVL descending

### T-L3-02: get-pools with native SOL address
```bash
./target/release/orca get-pools \
  --token-a 11111111111111111111111111111111 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```
Expected: Same result as T-L3-01 (native SOL normalized to wSOL internally).

### T-L3-03: get-pools ORCA/USDC
```bash
./target/release/orca get-pools \
  --token-a orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```
Expected: `ok: true`, pools with ORCA/USDC pair.

### T-L3-04: get-pools no match
```bash
./target/release/orca get-pools \
  --token-a EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v
```
Expected: `ok: true, pools_found: 0, pools: []`.

### T-L3-05: get-pools include-low-liquidity
```bash
./target/release/orca get-pools \
  --token-a So11111111111111111111111111111111111111112 \
  --token-b EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --include-low-liquidity
```
Expected: Returns all pools including those with TVL < $10,000.

### T-L3-06: get-quote SOL -> USDC
```bash
./target/release/orca get-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.5
```
Expected:
- `ok: true`
- `estimated_amount_out > 0` (should be ~60-70 USDC for 0.5 SOL)
- `minimum_amount_out < estimated_amount_out`
- `pool_address` non-empty
- `slippage_bps: 50` (default)

### T-L3-07: get-quote USDC -> SOL
```bash
./target/release/orca get-quote \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token So11111111111111111111111111111111111111112 \
  --amount 10
```
Expected: `estimated_amount_out > 0` (inverse direction works).

### T-L3-08: get-quote with custom slippage
```bash
./target/release/orca get-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 1 \
  --slippage-bps 100
```
Expected: `slippage_bps: 100`, `minimum_amount_out ≈ estimated_amount_out * 0.99`.

### T-L3-09: get-quote no pool found
```bash
./target/release/orca get-quote \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 10
```
Expected: Error: `No Orca pools found for pair`.

---

## L4: Live Swap Tests (on-chain — requires onchainos login + SOL balance)

### Prerequisites
- `onchainos` CLI installed and authenticated
- Wallet has ≥ 0.01 SOL for gas fees
- Test on small amounts only

### T-L4-01: swap dry-run (no wallet required)
```bash
./target/release/orca --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001
```
Expected: `ok: true, dry_run: true` — no real transaction.

### T-L4-02: swap SOL -> USDC (real, small amount)
**CONFIRM WITH USER before executing.**
```bash
./target/release/orca swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001 \
  --slippage-bps 100
```
Expected:
- `ok: true`
- `tx_hash` non-empty (base58 signature)
- `solscan_url` points to valid transaction
- Verify on https://solscan.io/tx/{tx_hash}

### T-L4-03: price impact block (large amount)
```bash
./target/release/orca swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token orcaEKTdK7LKz57vaAYr9QeNsVEPfiu6QeMU1kektZE \
  --amount 1000000
```
Expected: `ok: false` with `error` containing "Price impact" and "exceeds block threshold".

---

## Security / Edge Case Tests

### T-SEC-01: Security scan block
Tests that a known scam token mint triggers the block response.
```bash
# Replace SCAM_MINT with a known flagged address
./target/release/orca swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token <SCAM_MINT> \
  --amount 0.001
```
Expected: `ok: false` with `error` containing "Security scan blocked".

### T-SEC-02: skip-security-check flag
```bash
./target/release/orca --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001 \
  --skip-security-check
```
Expected: Returns dry_run result without calling `onchainos security token-scan`.
