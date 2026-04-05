# Test Results Report — Rocket Pool Plugin

- **Date:** 2026-04-05
- **Test chain:** Ethereum mainnet (chain ID 1)
- **Binary:** `rocket-pool v0.1.0`
- **Compile:** PASS
- **Lint:** PASS (manual — plugin-store not installed in environment)

---

## Summary

| Total | L1 Build | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|----------|---------|-------------|-------------|--------|---------|
| 10    | 2        | 4       | 4           | 0 (skipped) | 0      | 1 (min deposit) |

---

## Detailed Results

| # | Scenario (User View) | Level | Command | Result | Notes |
|---|---------------------|-------|---------|--------|-------|
| 1 | Release build | L1 | `cargo build --release` | PASS | No errors, 2 minor `dead_code` warnings suppressed with `#[allow(dead_code)]` |
| 2 | Lint: plugin.yaml format | L1 | Manual review | PASS | api_calls is string list, source_repo correct, author GeoGu360 |
| 3 | Lint: write ops have confirm text | L1 | Manual review SKILL.md | PASS | Both stake and unstake include "Ask user to confirm" per E106 |
| 4 | Get rETH exchange rate | L2 | `rocket-pool rate --chain 1` | PASS | 1 rETH = 1.160804 ETH (live data) |
| 5 | Get protocol stats | L2 | `rocket-pool stats --chain 1` | PASS | TVL: 394,230 ETH, 4,114 nodes, 42,317 minipools, 12.93 ETH in deposit pool |
| 6 | Get staking APY | L2 | `rocket-pool apy --chain 1` | PASS | 2.02% APY from Rocket Pool API |
| 7 | Check positions (no rETH) | L2 | `rocket-pool positions --chain 1 --address 0x87fb0647...` | PASS | Shows 0 rETH position correctly |
| 8 | Dry-run stake below minimum | L3 | `rocket-pool stake --amount 0.00005 --dry-run` | PASS | Correctly rejected: "Minimum deposit is 0.01 ETH" |
| 9 | Dry-run stake 0.01 ETH | L3 | `rocket-pool stake --amount 0.01 --from 0x87fb... --dry-run` | PASS | Calldata: `0xd0e30db0`, contract: `0xce152...`, expected ~0.008615 rETH |
| 10 | Dry-run unstake 0.01 rETH | L3 | `rocket-pool unstake --amount 0.01 --from 0x87fb... --dry-run` | PASS | Calldata: `0x42966c68` + ABI-encoded amount, expected ~0.011608 ETH |
| 11 | Live stake transaction | L4 | SKIPPED | — | See L4 Skip Reason below |

---

## L4 Skip Reason: Insufficient ETH for Minimum Deposit

**Wallet:** `0x87fb0647faabea33113eaf1d80d67acb1c491b90`
**Current ETH balance:** ~0.005272 ETH
**Required reserve:** 0.001 ETH (minimum to keep)
**Available for deposit:** 0.005272 - 0.001 = ~0.004272 ETH
**Rocket Pool minimum deposit:** 0.01 ETH (protocol-enforced)

Since 0.004272 ETH < 0.01 ETH minimum deposit, a live on-chain stake transaction cannot be
submitted without violating the Rocket Pool protocol minimum. L4 is intentionally skipped.

**To run L4 in the future:** Top up the wallet to at least 0.011 ETH (0.01 ETH deposit + 0.001 ETH gas reserve).

---

## Selector Verification

All function selectors verified via `cast sig`. See `tests/selector_verification.md`.

| Function | Selector | Status |
|---|---|---|
| `deposit()` | `0xd0e30db0` | Verified |
| `burn(uint256)` | `0x42966c68` | Verified |
| `getExchangeRate()` | `0xe6aa216c` | Verified, live data returned |
| `getAddress(bytes32)` | `0x21f8a721` | Verified, all 5 contracts resolved |
| `balanceOf(address)` | `0x70a08231` | Verified |
| `totalSupply()` | `0x18160ddd` | Verified |
| `getTotalETHBalance()` | `0x964d042c` | Verified |
| `getNodeCount()` | `0x39bf397e` | Verified: 4,114 |
| `getMinipoolCount()` | `0xae4d0bed` | Verified: 42,317 |
