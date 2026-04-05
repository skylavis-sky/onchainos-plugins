# Across Protocol Plugin — Test Results

**Date:** 2026-04-05  
**Tester:** Tester Agent (automated)  
**Plugin version:** 0.1.0  
**Wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` (Base chain 8453)

---

## L1 — Compile + Lint

| Check | Result |
|---|---|
| `cargo build --release` | PASS |
| `plugin-store lint .` | PASS (1 warning) |

**Warning:** `[W060] binary.checksums_asset not set — SHA256 verification will be skipped`  
This is non-blocking; no SHA256 asset is published yet.

**Fix records:** None required.

---

## L2 — Read Tests (No Wallet, No Gas)

### get-routes (Base → Optimism)
- **Command:** `./target/release/across get-routes --origin-chain-id 8453 --destination-chain-id 10`
- **Result:** PASS
- **Output:** 9 routes found, including USDC→USDC, ETH, WETH, USDT, DAI routes
- **Valid JSON / structured output:** Yes

### get-limits (Base USDC → Optimism USDC)
- **Command:** `./target/release/across get-limits --origin-chain-id 8453 --destination-chain-id 10 --input-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85`
- **Result:** PASS
- **minDeposit:** 500019 (at time of test)
- **maxDeposit:** 429758944292
- **Raw JSON returned:** Yes

### get-quote (1 USDC, Base → Optimism)
- **Command:** `./target/release/across get-quote --origin-chain-id 8453 --destination-chain-id 10 --input-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 --amount 1000000`
- **Result:** PASS
- **outputAmount:** 999641
- **isAmountTooLow:** false
- **estimatedFillTimeSec:** 2
- **spokePoolAddress:** 0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64

### get-status (fake hash, graceful 404 handling)
- **Command:** `./target/release/across get-status --tx-hash 0x000...001 --origin-chain-id 8453`
- **Result:** PASS (after bug fix — see Fix Records below)
- **Output:** Status "not_found" printed gracefully, exit code 0

**Note on test instruction discrepancy:** The test instructions specify `--deposit-tx-hash` but the CLI argument is `--tx-hash`. The CLI code is correct; the test instructions had a typo. Used `--tx-hash` flag throughout.

---

## L3 — Dry-Run Tests

### Dry-run: Bridge 1 USDC Base → Optimism
- **Command:** `bridge --origin-chain-id 8453 --destination-chain-id 10 --input-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 --amount 1000000 --dry-run`
- **Result:** PASS
- **dry_run:** true (confirmed "DRY RUN COMPLETE" output)
- **Calldata selector:** `0x7b939232` (depositV3) — PASS
- **Approve calldata:** `0x095ea7b3...` — PASS
- **No on-chain tx submitted:** Confirmed

**Note on L3 ETH test:** The test instructions specify `--input-token 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE` for native ETH but the Across API rejects this address (returns `INVALID_PARAM: Unsupported token on given origin chain`). Additionally, the WETH Base→Arbitrum route had zero liquidity at the time of testing. Used WETH Ethereum→Optimism as a substitute ETH dry-run test.

### Dry-run: Bridge WETH Ethereum → Optimism (substitute for ETH test)
- **Command:** `bridge --origin-chain-id 1 --destination-chain-id 10 --input-token 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 --output-token 0x4200000000000000000000000000000000000006 --amount 10000000000000000 --dry-run`
- **Result:** PASS
- **Calldata selector:** `0x7b939232` — PASS
- **SpokePool:** 0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5 (Ethereum mainnet config) — PASS
- **No ETH value set for WETH (ERC-20):** Confirmed (`ETH value (wei): None`)

---

## L4 — On-Chain Bridge Test

### Pre-test wallet balance (Base, chain 8453)
| Asset | Balance |
|---|---|
| ETH | 0.004247226217956315 |
| USDC | 1.235393 |

### Bridge execution
- **Command:** `bridge --origin-chain-id 8453 --destination-chain-id 10 --input-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 --amount 600000`
- **Amount:** 600000 (0.6 USDC) — raised from 500000 because minDeposit was ~500018-500019 and API rejected 500000 as AMOUNT_TOO_LOW
- **Approve tx:** `0xd940d5adfd8ef1b06aa259fa6dc1d79b616419c63161c393a50a8b1e18bf2b33`
- **Deposit tx:** `0xa75ee25531150bb2637e78365a2f42813ddd1b91b283b1a1967a4f8966318bfd`
- **BaseScan:** https://basescan.org/tx/0xa75ee25531150bb2637e78365a2f42813ddd1b91b283b1a1967a4f8966318bfd

### Bridge status polling
- **Attempts:** 1 (fill confirmed on first poll)
- **Status:** `filled` — Bridge COMPLETE
- **Destination tx:** N/A (API returned status "filled" but fillTxnHash was N/A at poll time; fill was confirmed)
- **Estimated fill time:** 2 seconds — accurate

### Post-test wallet balance (Base, chain 8453)
| Asset | Balance | Guardrail | Status |
|---|---|---|---|
| ETH | 0.004245478209714022 | > 0.001 | PASS |
| USDC | 0.635393 | > 0.1 | PASS |

---

## Fix Records

### Fix 1: get-status 404 graceful handling
- **Test:** L2 get-status with fake tx hash
- **Error:** Exit code 1, `deposit/status API error 404 Not Found: {"error":"DepositNotFoundException",...}`
- **Root cause:** `api::get_deposit_status` called `anyhow::bail!` on 404 responses, causing non-zero exit code instead of graceful output
- **Fix:** Added 404 special-case in `/Users/samsee/projects/plugin-store-dev/across/src/api.rs` to return a synthetic `{"status":"not_found",...}` JSON object instead of bailing
- **File:** `/Users/samsee/projects/plugin-store-dev/across/src/api.rs` (lines 153-163)
- **Retest:** PASS

---

## Summary

| Level | Tests | Passed | Failed | Blocked |
|---|---|---|---|---|
| L1 Compile+Lint | 2 | 2 | 0 | 0 |
| L2 Read Tests | 4 | 4 | 0 | 0 |
| L3 Dry-Run | 2 | 2 | 0 | 0 |
| L4 On-Chain | 1 | 1 | 0 | 0 |
| **Total** | **9** | **9** | **0** | **0** |

**Overall result: ALL TESTS PASSED**

### Known Issues / Notes
1. **`0xEeee...eEeE` ETH placeholder not supported by Across API** — The Across API rejects the conventional EVM native ETH placeholder address. The ETH route uses the WETH address (`0x4200000000000000000000000000000000000006` on Base/Optimism, `0xC02aaa...` on mainnet). This is an API behavior to document; no code change needed in the plugin since it passes through whatever address the user provides.
2. **WETH Base→Arbitrum had zero liquidity** at test time — not a plugin bug.
3. **L4 amount changed from 500000 to 600000** — The minDeposit was approximately 500018-500019 and the API also uses dynamic fee calculations that can cause 500000 to be rejected even though it's numerically above minDeposit. Using 600000 (0.6 USDC) was the safe minimum.
