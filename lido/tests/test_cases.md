# Lido Plugin — Test Cases

**Plugin:** lido  
**Binary:** `lido`  
**Date:** 2026-04-05  
**Chains tested:** Ethereum (1), Base (8453)

---

## Level 1 — Compile + Lint

| # | Test | Command | Pass Criteria |
|---|------|---------|---------------|
| L1-1 | Compile release binary | `cargo build --release` | Exit 0, binary produced |
| L1-2 | Lint plugin | `cargo clean && plugin-store lint .` | 0 errors |

---

## Level 2 — Read Tests (No Wallet, No Gas)

| # | Scenario (user view) | Command | Pass Criteria |
|---|---------------------|---------|---------------|
| L2-1 | Check Lido staking APR on Ethereum | `lido get-apr` | `ok=true`, `smaApr` is a non-zero float |
| L2-2 | Check Lido staking APR on Base (API is chain-agnostic) | `lido --chain 8453 get-apr` | `ok=true`, `smaApr` is a non-zero float |
| L2-3 | Query stETH/wstETH position for known wallet | `lido get-position --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok=true`, `stETH` and `wstETH` fields present |
| L2-4 | Query withdrawal status for a known finalized request (request ID 1) | `lido get-withdrawal-status --request-ids 1` | `ok=true`, `statuses` array with `status=claimed` |

---

## Level 3 — Dry-Run / Calldata Verification

| # | Scenario (user view) | Command | Expected Selector | Pass Criteria |
|---|---------------------|---------|-------------------|---------------|
| L3-1 | Dry-run stake 0.00005 ETH | `lido --dry-run stake --amount 50000000000000 --from 0xee385...` | `0xa1903eab` | `dry_run=true`, calldata starts with `0xa1903eab` |
| L3-2 | Dry-run wrap 1000000000000 wei stETH | `lido --dry-run wrap --amount 1000000000000 --from 0xee385...` | `0xea598cb0` | `dry_run=true`, calldata starts with `0xea598cb0` |
| L3-3 | Dry-run unwrap 1000000000000 wei wstETH on Ethereum | `lido --dry-run unwrap --amount 1000000000000 --from 0xee385...` | `0xde0e9a3e` | `dry_run=true`, calldata starts with `0xde0e9a3e` |
| L3-4 | Dry-run unwrap on Base | `lido --chain 8453 --dry-run unwrap --amount 1000000000000 --from 0xee385...` | `0xde0e9a3e` | `dry_run=true`, correct wstETH contract on Base |
| L3-5 | Dry-run request-withdrawal 100000000000000000 wei stETH | `lido --dry-run request-withdrawal --amount 100000000000000000 --from 0xee385...` | `0xd6681042` | `dry_run=true`, calldata starts with `0xd6681042` |

---

## Level 4 — On-Chain Write Tests (Ethereum, needs lock)

| # | Scenario (user view) | Command | Min Amount | Pass Criteria |
|---|---------------------|---------|------------|---------------|
| L4-1 | User stakes 0.00005 ETH to get stETH | `lido stake --amount 50000000000000 --from 0xee385...` | 50000000000000 wei (0.00005 ETH) | `ok=true`, `txHash` non-zero, confirmed on Etherscan |
| L4-2 | User wraps a tiny amount of stETH into wstETH | `lido wrap --amount 40000000000000 --from 0xee385...` | 40000000000000 wei stETH | `ok=true`, `txHash` non-zero, confirmed on Etherscan |

**Notes:**
- L4-1: stake 0.00005 ETH → receive ~0.00005 stETH
- L4-2: wrap ~40000000000000 wei stETH → receive wstETH (amount < staked to leave buffer)
- request-withdrawal and claim-withdrawal are L3 dry-run only — minimum is ~0.1 stETH which exceeds test limits
