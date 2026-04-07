# Aura Finance Plugin — Test Results

**Date:** 2026-04-07  
**Plugin:** `/tmp/onchainos-plugins/aura-finance/`  
**Tester:** Claude (automated)

---

## Summary

| Level | Status | Notes |
|-------|--------|-------|
| L1 — Build | **PASS** | 1 expected warning only (`AURA_BAL` unused) |
| L2 — Read Operations | **PASS** | Balancer API unavailable (expected fallback); on-chain reads succeeded |
| L3 — Dry-Run | **PASS** | All 5 commands produce correct calldata |
| L4 — Live On-Chain | **BLOCKED** | No test wallet with staked BPT/AURA on Ethereum |
| Static Analysis | **PASS** | All checks pass (with one doc-level note) |

**Overall Recommendation: SHIP** — No P0 issues. One P1 (test spec invocation mismatch), one P2 (informational).

---

## L1 — Build

**Result: PASS**

```
warning: constant `AURA_BAL` is never used
  --> src/config.rs:10:11
warning: `aura-finance` (bin "aura-finance") generated 1 warning
    Finished `release` profile [optimized] target(s)
```

- Zero errors.
- One warning for unused `AURA_BAL` constant — expected and acceptable per spec.

---

## L2 — Read Operations

**Result: PASS**

### get-pools

```
./target/release/aura-finance get-pools --chain 1
```

- Balancer REST API unavailable in test environment; plugin correctly fell back to on-chain reads with warning message.
- Returned 10 pools from 283 total (scanned most-recent 50, skipped shutdown pools).
- Each pool includes `aura_pid`, `crv_rewards`, and `lp_token` addresses — all live on-chain data.
- `tvl_usd: "N/A"` and `tokens: []` as expected (Balancer API offline, tokens not enriched).

### get-position

```
./target/release/aura-finance get-position --chain 1 --address 0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1 --pool-id 0
```

- NOTE: The test spec used `--pid 0` but the correct CLI flag is `--pool-id`. Corrected for testing.
- Returned correct on-chain data: pool 0 is shutdown, lp_token and base_reward_pool addresses fetched live.
- vlAURA balance, liquid AURA/BAL balances all returned correctly (zero for this address as expected).

---

## L3 — Dry-Run

**Result: PASS**

All 5 commands produce correct output with verified calldata.

### deposit
```
./target/release/aura-finance --dry-run deposit --chain 1 --pool-id 0 --amount 1.0
```
- Selector `0x43a0d066` confirmed in calldata.
- Two steps output: approve + deposit.
- `_stake=true` (0x01) correctly appended.
- Approve uses `u128::MAX` (unlimited approval pattern).

### claim-rewards
```
./target/release/aura-finance --dry-run claim-rewards --chain 1 --pool-id 0
```
- Selector `0x7050ccd9` confirmed in calldata.
- `_claimExtras=true` (0x01) correctly encoded.

### lock-aura
```
./target/release/aura-finance --dry-run lock-aura --chain 1 --amount 1.0
```
- Selector `0x282d3fdf` confirmed in calldata.
- Uses `lock(address,uint256)` — Aura ABI, NOT Convex's `lock(uint256,uint256)` (`0x1338736f`).
- **16-week WARNING** present in dry-run output JSON field.
- Two steps output: approve AURA + lock.

### unlock-aura
```
./target/release/aura-finance --dry-run unlock-aura --chain 1
```
- Selector `0x312ff839` confirmed in calldata.
- Uses `alloy_sol_types` ABI encoding (not manual hex) — robust.
- `relock=false` correctly encoded as `0x00`.

### withdraw
```
./target/release/aura-finance --dry-run withdraw --chain 1 --pool-id 0 --amount 1.0
```
- Uses `withdrawAndUnwrap(uint256,bool)` selector `0xc32e7202`.
- `claim=false` correctly encoded (rewards handled separately).
- Note in output correctly explains BaseRewardPool address is fetched from `Booster.poolInfo(pid)`.

---

## L4 — Live On-Chain

**Result: BLOCKED**

No test wallet with staked BPT or AURA on Ethereum mainnet available in this environment. All write operation paths are verified via dry-run and static analysis. Live execution skipped.

---

## Static Analysis

**Result: PASS**

### ABI Selector Verification Table

| Function Signature | Expected Selector | Plugin Selector | Match |
|-------------------|------------------|-----------------|-------|
| `deposit(uint256,uint256,bool)` | `0x43a0d066` | `0x43a0d066` | PASS |
| `getReward(address,bool)` | `0x7050ccd9` | `0x7050ccd9` | PASS |
| `lock(address,uint256)` | `0x282d3fdf` | `0x282d3fdf` | PASS |
| `processExpiredLocks(bool)` | `0x312ff839` | `0x312ff839` | PASS |
| `approve(address,uint256)` | `0x095ea7b3` | `0x095ea7b3` | PASS |
| `withdrawAndUnwrap(uint256,bool)` | `0xc32e7202` | `0xc32e7202` | PASS |
| `balanceOf(address)` | `0x70a08231` | `0x70a08231` | PASS |
| `allowance(address,address)` | `0xdd62ed3e` | `0xdd62ed3e` | PASS |
| `earned(address)` | `0x008cc262` | `0x008cc262` | PASS |
| `poolLength()` | `0x081e3eda` | `0x081e3eda` | PASS |
| `poolInfo(uint256)` | `0x1526fe27` | `0x1526fe27` | PASS |

Convex `lock(uint256,uint256)` = `0x1338736f` — correctly NOT used. Aura uses `lock(address,uint256)` = `0x282d3fdf`.

### extract_tx_hash_or_err Pattern

- `extract_tx_hash_or_err` defined in `src/onchainos.rs:76`.
- Used consistently across all 5 write commands: deposit (x2), claim_rewards, withdraw, lock_aura (x2), unlock_aura.
- No `unwrap_or("pending")` pattern found anywhere in codebase.
- **PASS**

### SKILL.md Description Field — No CJK

- Description: `"Deposit Balancer LP tokens (BPT) into Aura Finance for boosted BAL and AURA rewards on Ethereum..."`
- Zero CJK characters in description field.
- **PASS**

### --dry-run and --chain Flag Positions

- Both flags are declared `global = true` in `src/main.rs` via `#[arg(long, global = true)]`.
- Verified working in all positions:
  - `aura-finance --dry-run deposit ...` — PASS
  - `aura-finance --chain 1 --dry-run deposit ...` — PASS
  - `aura-finance deposit --dry-run --chain 1 ...` — PASS
- **PASS**

### --output json NOT Used

- `src/onchainos.rs` comment explicitly notes: `no --output json on chain 1`.
- `onchainos wallet balance --chain 1` called without `--output json` flag.
- SKILL.md notes: `onchainos wallet balance --chain 1 --output json is NOT supported`.
- No `--output json` appears as a live CLI invocation anywhere in source.
- **PASS**

### BPT lpToken Dynamic Fetch

- `deposit.rs:72`: `rpc::booster_pool_info(config::BOOSTER, args.pool_id)` fetches `lp_token` live from `Booster.poolInfo(pid)`.
- `withdraw.rs:54`: same dynamic lookup for `crv_rewards` address.
- `claim_rewards.rs:51`: same dynamic lookup.
- No BPT or BaseRewardPool addresses are hardcoded in write commands.
- **PASS**

### lock-aura 16-Week Warning

- Dry-run output contains JSON field: `"WARNING": "AURA will be locked as vlAURA for 16 WEEKS. This lock is IRREVERSIBLE until expiry. ask user to confirm."`
- Live path `eprintln!` in `lock_aura.rs:98-104` prints prominent multi-line warning to stderr before submitting.
- SKILL.md has bold warning block at top of `lock-aura` section.
- **PASS**

---

## Issues Found

### P1 — Test Spec Uses Wrong CLI Flags

**File:** N/A (test specification issue, not plugin bug)

The test spec invokes:
```
aura-finance --dry-run deposit --chain 1 --pid 0 --amount 1000000000000000000
aura-finance --dry-run claim-rewards --chain 1 --pid 0
aura-finance get-position --chain 1 --address 0x... --pid 0
```

The correct CLI flags are `--pool-id` (not `--pid`). The test spec also passes `--amount 1000000000000000000` (raw wei) when the CLI expects human-readable token units (e.g. `--amount 1.0`). Passing `1e18` as the amount triggers float multiplication by `1e18` again, resulting in an encoded amount of `~1e36` tokens — a non-fatal but semantically incorrect invocation.

**Impact:** Test invocations with `--pid` fail with `error: unexpected argument '--pid' found`. Plugin behavior is correct; test spec needs updating.

**Fix:** Update test spec to use `--pool-id` and human-readable amounts (e.g. `--amount 1.0`).

### P2 — get-position --pid Alias Not Available

**File:** `src/commands/get_position.rs`

No `--pid` short alias exists for `--pool-id`. This is consistent with the rest of the codebase but could improve DX if `--pid` were accepted as an alias. Low priority.

---

## Key Verification Findings

| Check | Result |
|-------|--------|
| deposit fetches lpToken dynamically | PASS — `booster_pool_info(pid)` called live |
| lock-aura 16-week WARNING | PASS — present in dry-run JSON and live eprintln |
| Aura `lock(address,uint256)` not Convex `lock(uint256,uint256)` | PASS — `0x282d3fdf` confirmed |
| `extract_tx_hash_or_err` pattern | PASS — used everywhere, no `unwrap_or("pending")` |
| No CJK in SKILL.md description | PASS |
| `--dry-run` and `--chain` global flags | PASS |
| `--output json` NOT used | PASS |
