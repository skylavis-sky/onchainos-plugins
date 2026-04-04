# Clanker Plugin — Test Cases

**Plugin:** clanker  
**Dev directory:** `/Users/samsee/projects/plugin-store-dev/clanker`  
**Test chains:** Base (8453)  
**Date:** 2026-04-05

---

## Level 1 — Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|---------|
| L1-1 | Release build compiles cleanly | `cargo build --release` | Exit 0, binary at `target/release/clanker` |
| L1-2 | Plugin store lint passes | `cargo clean && plugin-store lint .` | 0 errors, 0 warnings |

---

## Level 2 — Read Tests (no wallet, no gas)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L2-1 | "Show latest Clanker tokens" | `list-tokens --limit 5 --sort desc` | `ok: true`, array of tokens with `contract_address`, `name`, `symbol` |
| L2-2 | "List newest 3 tokens on Base" | `--chain 8453 list-tokens --limit 3` | `ok: true`, `chain_id: 8453` tokens |
| L2-3 | "Search tokens by creator address" | `search-tokens --query 0xd8dA6BF26964aF9D7eEd9e03E53415D37aA96045` | `ok: true`, token list or empty result |
| L2-4 | "What tokens did dwr launch on Clanker" | `search-tokens --query dwr --limit 5` | `ok: true`, tokens array with `name` and `symbol` |
| L2-5 | "Get info for a specific Clanker token" | `token-info --address 0x532f27101965dd16442e59d40670faf5ebb142e4` | `ok: true`, token `info` and `price` fields populated |

---

## Level 3 — Simulate (dry-run + calldata validation)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L3-1 | "Preview claiming rewards for my Clanker token" | `--dry-run claim-rewards --token-address 0x532f27101965dd16442e59d40670faf5ebb142e4 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok: true`, `dry_run: true`, calldata with selector `0xa514a564` (collectFees) |
| L3-2 | "Preview deploying a token on Clanker" | `--dry-run deploy-token --name "TestDog" --symbol "TDOG" --api-key testkey123 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok: true`, `dry_run: true`, shows `name`, `symbol`, `token_admin`, `api_endpoint` |

**Note:** `deploy-token` L3 dry-run tests the API call path without actually calling the API. L4 deploy is SKIPPED (no partner API key for test environment).

---

## Level 4 — On-chain Write Tests (requires lock)

| # | Scenario (user view) | Command | Expected | Notes |
|---|---------------------|---------|---------|-------|
| L4-1 | "Claim my Clanker LP rewards" | `claim-rewards --token-address 0x532f27101965dd16442e59d40670faf5ebb142e4 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | PASS (txHash) or PASS (no rewards — correct behavior) | Check pending rewards first; skip real broadcast if no rewards |
| L4-2 | Deploy token L4 | N/A | SKIPPED | No partner API key available for test env; dry-run covers the code path |

---

## Error Handling Tests

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| E1 | Missing required token-address | `claim-rewards` (no args) | Non-zero exit, error message about missing argument |
| E2 | Invalid token address format | `token-info --address notanaddress` | `ok: false` or graceful error |
| E3 | deploy-token without API key | `deploy-token --name X --symbol X --from 0xee385...` | `ok: false`, "Clanker API key is required" |

---

## collectFees ABI Selector Reference

`collectFees(address token)` — keccak256 first 4 bytes  
Expected: verify against actual encoded calldata from L3-1 dry-run output.

The `alloy_sol_types::SolCall::abi_encode()` produces: `[selector (4 bytes)][padded address (32 bytes)]` = 36 bytes total  
Calldata length: `0x` + 72 hex chars = 74 chars total.
