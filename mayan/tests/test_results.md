# Mayan Plugin — Phase 3 Test Results

**Date:** 2026-04-05  
**Tester:** Phase 3 automated test agent  
**Plugin version:** 0.1.0  
**Binary:** `/Users/samsee/projects/plugin-store-dev/mayan/target/release/mayan`

---

## Summary

| Level | Name | Result | Notes |
|-------|------|--------|-------|
| L1 | Build + Lint | PASS | 1 warning (non-blocking) |
| L2 | Read ops (get-quote) | BLOCKED | Mayan `/v3/quote` API returning HTTP 500 globally |
| L3 | Dry-run swap | PARTIAL PASS | Wallet resolution passes; fails at quote step (API down) |
| L4 | Live swap | SKIPPED | Prerequisite (L2/L3 quote) blocked by API outage |

**Overall verdict: BLOCKED — Mayan price-api `/v3/quote` endpoint is experiencing a service outage (HTTP 500 on all route combinations). L1 passes cleanly. L3 confirms wallet resolution works correctly. L4 skipped due to quote dependency.**

---

## L1 — Build and Lint

### Build (`cargo build --release`)

```
Finished `release` profile [optimized] target(s) in 14.62s
```

**Result: PASS**

All 109 crates compiled without errors. Binary produced at `target/release/mayan`.

### Lint (`plugin-store lint .`)

```
⚠️  [W060] binary.checksums_asset not set — SHA256 verification will be skipped
✓ Plugin 'mayan' passed with 1 warning(s)
```

**Result: PASS (1 non-blocking warning)**

Warning W060: `binary.checksums_asset` not set. This is a distribution-time concern (SHA256 pinning for release assets) and does not affect plugin functionality or correctness. No errors.

---

## L2 — Read Operations (get-quote)

### Test 2a: Solana USDC → Base USDC

```
Command: ./target/release/mayan get-quote \
  --from-chain 501 --to-chain 8453 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 0.3

Error: Failed to fetch quote: Mayan /quote returned HTTP 500 Internal Server Error:
{"statusCode":500,"message":"Internal server error"}
```

**Result: FAIL — API outage**

### Test 2b: Base ETH → Solana USDC

```
Command: ./target/release/mayan get-quote \
  --from-chain 8453 --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001

Error: Failed to fetch quote: Mayan /quote returned HTTP 500 Internal Server Error:
{"statusCode":500,"message":"Internal server error"}
```

**Result: FAIL — API outage**

### Test 2c: Arbitrum ETH → Solana USDC

```
Command: ./target/release/mayan get-quote \
  --from-chain 42161 --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001

Error: Failed to fetch quote: Mayan /quote returned HTTP 500 Internal Server Error:
{"statusCode":500,"message":"Internal server error"}
```

**Result: FAIL — API outage**

### API investigation

Direct `curl` probes to `https://price-api.mayan.finance/v3/quote` with multiple parameter
variations all returned HTTP 500:

- `amountIn64` parameter (integer base units) — HTTP 500
- `amountIn` parameter (float) — HTTP 500 
- Different chain combinations (Solana→EVM, EVM→Solana, EVM→EVM) — all HTTP 500
- `slippageBps` values of 10, 50, 100 — all HTTP 500

Supplemental checks confirmed the Mayan API is partially operational:

- `GET /v3/tokens?chain=base` — **HTTP 200** (tokens list works)
- `GET https://explorer-api.mayan.finance/v3/swap/trx/<hash>` — **HTTP 404** (expected, explorer API works)
- `GET /v3/quote` — **HTTP 500** (all route combinations, confirmed outage)

The `/v3/quote` endpoint appears to be experiencing a backend service outage as of 2026-04-05.
This is an external dependency failure, not a plugin bug.

---

## L3 — Dry-run Swap

### Test 3a: Solana USDC → Base USDC (dry-run)

```
Command: ./target/release/mayan swap \
  --from-chain 501 --to-chain 8453 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 0.3 --dry-run

Output:
  Mayan Cross-Chain Swap
  ------------------------------------------------------------
    From: EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v on solana (chain 501)
    To:   0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 on base (chain 8453)
    Amount: 0.3 (base units: 300000)
    Slippage: 100 bps
    [DRY RUN - no transactions will be broadcast]

  [1/4] Resolving wallet addresses...
    From wallet: 6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY   ✓ resolved correctly
    To wallet:   0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9    ✓ resolved correctly

  [2/4] Fetching quote...
  Error: Failed to fetch quote: Mayan /quote returned HTTP 500
```

**Result: PARTIAL PASS**

Step 1 (wallet resolution) passed for both Solana and EVM sides:
- Solana wallet resolved via `onchainos wallet balance --chain 501` without `--output json` (correct per KB)
- Wallet address extracted from `data.details[0].tokenAssets[0].address` path (correct per KB)
- EVM wallet resolved correctly for Base (chain 8453)
- Amount encoding: 0.3 USDC correctly encoded as 300000 (6 decimals)

Step 2 blocked by API outage.

### Test 3b: Base ETH → Solana USDC (dry-run)

```
Command: ./target/release/mayan swap \
  --from-chain 8453 --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001 --dry-run

Output:
  Mayan Cross-Chain Swap
  ------------------------------------------------------------
    From: 0x0000000000000000000000000000000000000000 on base (chain 8453)
    To:   EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v on solana (chain 501)
    Amount: 0.001 (base units: 1000000000000000)
    Slippage: 100 bps
    [DRY RUN - no transactions will be broadcast]

  [1/4] Resolving wallet addresses...
    From wallet: 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9   ✓ resolved correctly
    To wallet:   6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY  ✓ resolved correctly

  [2/4] Fetching quote...
  Error: Failed to fetch quote: Mayan /quote returned HTTP 500
```

**Result: PARTIAL PASS**

Step 1 passed:
- EVM wallet resolved for Base (chain 8453)
- Solana wallet resolved for destination
- Amount encoding: 0.001 ETH correctly encoded as 1000000000000000 (18 decimals)

Step 2 blocked by API outage.

### Additional dry-run verification — get-status

```
Command: ./target/release/mayan get-status \
  --tx-hash 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef

Output:
  Error: Mayan explorer API returned HTTP 404 Not Found:
  {"statusCode":404,"timestamp":"...","path":"/v3/swap/trx/0x1234...","message":"Transaction not found"}
```

**Result: PASS** — get-status correctly queries explorer API; 404 for unknown hash is expected.

---

## L4 — Live Transactions

### Test 4a: Solana USDC → Base USDC (live)

**Result: SKIPPED — Prerequisite blocked by Mayan /v3/quote API outage**

Planned parameters:
- From: 0.3 USDC on Solana (wallet: `6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY`, balance: 0.327262)
- To: Base USDC (wallet: `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`)
- Expected route: SWIFT or MCTP

### Test 4b: Base ETH → Solana USDC (live)

**Result: SKIPPED — Prerequisite blocked by Mayan /v3/quote API outage**

Planned parameters:
- From: 0.001 ETH on Base (wallet: `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`, balance: 0.003242)
- To: Solana USDC (wallet: `6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY`)
- Native ETH — no ERC-20 approve needed

**Lock not acquired** — L4 was skipped due to API outage; no lock acquisition needed.

---

## Code Review Notes

The following implementation behaviors were verified by source inspection and dry-run output:

| Check | Status | Evidence |
|-------|--------|----------|
| Solana `wallet balance` called without `--output json` | PASS | `onchainos.rs:32` — no `--output json` flag |
| Wallet address parsed from `data.details[0].tokenAssets[0].address` | PASS | `onchainos.rs:14-24` — correct path used |
| base64→base58 conversion for `--unsigned-tx` | PASS | `onchainos.rs:47-50` — `BASE64.decode()` → `bs58::encode()` |
| `--force` flag on all `contract-call` invocations | PASS | `onchainos.rs:82` (EVM) and `onchainos.rs:122` (Solana) |
| Native ETH passes `--amt` in wei | PASS | `swap.rs:298-308` — `eth_value = Some(amount_in64_u64)` for native token |
| ERC-20 approve before swap with 3s wait | PASS | `swap.rs:311-333` — approve then `sleep(3s)` |
| SWIFT v2 program ID used | PASS | `swap.rs:393` — maps "SWIFT" → `SWIFT_V2_PROGRAM_ID` |
| Correct default slippage (100 bps) | PASS | `config.rs:21`, `swap.rs:34` |
| Solana tx fields probed in multiple shapes | PASS | `swap.rs:363-388` — checks `transaction`, `serializedTx`, `tx`, `swapTransaction`, `data` |

---

## Issues Found

### Issue 1: Mayan /v3/quote API outage (External — P0 blocker)

- **Type:** External service outage
- **Severity:** P0 — blocks all quote-dependent functionality (L2/L3/L4)
- **Scope:** All route combinations return HTTP 500 from `https://price-api.mayan.finance/v3/quote`
- **Plugin code:** Not at fault — error handling is correct (surfaces the HTTP error cleanly)
- **Recommendation:** Retry test run when Mayan API is restored

### Issue 2: W060 lint warning — checksums_asset not set (Low priority)

- **Type:** Distribution metadata
- **Severity:** Low — non-blocking warning
- **Recommendation:** Set `binary.checksums_asset` in `plugin.yaml` before publishing to store

---

## Wallet Balances (Pre-test, unchanged)

| Chain | Asset | Balance |
|-------|-------|---------|
| Base (8453) | ETH | 0.003242 |
| Base (8453) | USDC | 0.179209 |
| Base (8453) | USDT | 1.000099 |
| Solana (501) | USDC | 0.327262 |
| Solana (501) | SOL | 0.003629 |

No balances were spent (L4 was skipped).
