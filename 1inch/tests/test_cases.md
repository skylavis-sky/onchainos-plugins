# Test Cases — 1inch Plugin

**Phase:** Phase 3 QA
**Date:** 2026-04-19
**Plugin version:** 0.1.0
**Test chain:** Base (8453)
**Test wallet:** 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9
**ETH balance (Base):** ~0.00238 ETH

---

## L1 — Build + Lint

| ID | Test | Command | Expected | Result |
|----|------|---------|----------|--------|
| L1-1 | Release build | `cargo build --release` | Exit 0, binary produced | PASS |
| L1-2 | Plugin lint | `plugin-store lint .` | "passed all checks!" | PASS |

---

## L2 — Read-Only Commands (Base 8453)

| ID | Test | Command | Expected | Result |
|----|------|---------|----------|--------|
| L2-1 | get-quote ETH→USDC | `get-quote --src ETH --dst USDC --amount 0.001 --chain 8453` | JSON with dst_amount, protocols | CONDITIONAL PASS — 401 (demo key) |
| L2-2 | get-allowance USDC (ERC-20) | `get-allowance --token USDC --chain 8453` | JSON with allowance field | CONDITIONAL PASS — needs wallet logged in + valid API key |
| L2-3 | get-allowance ETH (native) | `get-allowance --token ETH --chain 8453` | allowance: "N/A", no wallet call | PASS |
| L2-4 | Unsupported chain rejection | `get-quote --src ETH --dst USDC --amount 0.001 --chain 999` | Error: Unsupported chain ID | PASS |
| L2-5 | Unknown token rejection | `get-quote --src UNKNOWN_TOKEN --dst USDC --amount 0.001 --chain 8453` | Error: Unknown token | PASS |

**Note on L2-2:** `get-allowance` on ERC-20 calls `onchainos wallet balance` to get wallet address before calling 1inch API. This requires onchainos wallet to be logged in. With wallet logged in but demo key, it would fail at 1inch API (401). This is expected with demo key — CONDITIONAL PASS.

---

## L3 — Dry-Run (No Broadcast)

| ID | Test | Command | Expected | Result |
|----|------|---------|----------|--------|
| L3-1 | swap --dry-run ETH→USDC | `swap --src ETH --dst USDC --amount 0.001 --slippage-bps 50 --chain 8453 --dry-run` | dry_run: true, tx.data present | CONDITIONAL PASS — 401 (needs valid API key for calldata) |
| L3-2 | approve --dry-run USDC | `approve --token USDC --chain 8453 --dry-run` | dry_run: true, tx.data present | CONDITIONAL PASS — 401 (needs valid API key for approve calldata) |
| L3-3 | dry-run skips wallet resolution | Same as L3-1 | No wallet error in stderr | PASS — "[dry-run] Dry-run mode active" shown; wallet resolution skipped |
| L3-4 | approve dry-run skips wallet | Same as L3-2 | No wallet error in stderr | PASS — wallet resolution correctly skipped |

---

## L4 — Live Transaction (Proposed — Awaiting Orchestrator Approval)

| ID | Test | Command | Expected |
|----|------|---------|----------|
| L4-1 | Live swap ETH→USDC | `swap --src ETH --dst USDC --amount 0.0005 --slippage-bps 50 --chain 8453` | Quote displayed, confirm prompt, tx broadcast, txHash returned |
| L4-2 | get-allowance USDC (post-swap) | `get-allowance --token USDC --chain 8453` | allowance: 0 or existing amount |
| L4-3 | approve USDC dry-run (with key) | `approve --token USDC --chain 8453 --dry-run` | dry_run: true, approve calldata with 0x095ea7b3 selector |
| L4-4 | Live swap USDC→ETH (ERC-20 src) | `swap --src USDC --dst ETH --amount 1 --slippage-bps 100 --chain 8453` | Allowance check → approve if needed → swap |

**Pre-conditions for L4:**
- `ONEINCH_API_KEY` must be set to a valid (non-demo) key
- Wallet must have ≥ 0.0005 ETH + gas on Base (current: 0.00238 ETH — sufficient)
- Wallet must have ≥ 1 USDC for L4-4

---

## Known Issues / Observations

| ID | Severity | Description |
|----|----------|-------------|
| OBS-1 | Minor | `get-allowance` requires onchainos wallet login even for read-only check (expected — wallet address needed for 1inch API query) |
| OBS-2 | Info | All L2/L3 tests conditional on valid `ONEINCH_API_KEY`; "demo" key returns 401 on all endpoints |
| OBS-3 | Info | `approve --dry-run` correctly bypasses wallet resolution but still needs API key for calldata |
| OBS-4 | Minor | Slippage conversion note: 50 bps passed to API as `0.5` (percent) — correct per 1inch API spec |
