# Test Results Report — Clanker Plugin

- **Date:** 2026-04-05
- **Test chain:** Base (8453)
- **Compile:** PASS
- **Lint:** PASS

---

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|------------|---------|-------------|-------------|--------|---------|
| 12    | 2          | 5       | 2           | 1           | 0      | 1 (SKIPPED) |

---

## Detailed Results

| # | Scenario (User View) | Level | Command | Result | TxHash / Calldata | Notes |
|---|---------------------|-------|---------|--------|-------------------|-------|
| 1 | Build compiles cleanly | L1 | `cargo build --release` | PASS | — | 8 dead-code warnings, no errors |
| 2 | Plugin store lint passes | L1 | `cargo clean && plugin-store lint .` | PASS | — | 0 errors, 0 warnings |
| 3 | "Show latest Clanker tokens" | L2 | `list-tokens --limit 5 --sort desc` | PASS | — | 5 tokens returned, total: 653336. **Bug fixed: API returns `data[]` not `tokens[]`** |
| 4 | "List newest 3 tokens on Base" | L2 | `--chain 8453 list-tokens --limit 3` | PASS | — | 3 Base tokens, `chain_id: 8453` correct |
| 5 | "Search tokens by creator 0xd8dA..." | L2 | `search-tokens --query 0xd8dA6BF...` | PASS | — | 240 tokens found for Vitalik's address |
| 6 | "What tokens did dwr launch" | L2 | `search-tokens --query dwr --limit 5` | PASS | — | 5 tokens, `searched_address` resolved to 0x6Ce09... |
| 7 | "Get info for Clanker token BRETT" | L2 | `token-info --address 0x532f271...` | PASS | — | Token info + price (895,719 holders, $0.006188/token) |
| 8 | "Preview claiming rewards" | L3 | `--dry-run claim-rewards --token-address 0xCdA659... --from 0xee385...` | PASS | `input_data: 0x5763dbd0000000000000000000000000cda659768e7d7f0f3ccca25185660fd67c7e2b07` | selector `0x5763dbd0` = `collectRewards(address)`. **Bug fixed: was `collectFees` with wrong selector** |
| 9 | "Preview deploying a new token" | L3 | `--dry-run deploy-token --name TestDog --symbol TDOG --api-key testkey123 --from 0xee385...` | PASS | — | Preview shows `name`, `symbol`, `token_admin`, `api_endpoint`, `request_key` |
| 10 | "Claim my LP rewards on-chain" | L4 | `claim-rewards --token-address 0xCdA659768E7D7F0F3CCca25185660FD67c7e2B07 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | PASS | `0x43f0c63d7a1d0806fc28268372fc6827b8890324228a0ad2455cc14187246ab1` | [BaseScan verified](https://basescan.org/tx/0x43f0c63d7a1d0806fc28268372fc6827b8890324228a0ad2455cc14187246ab1). Transaction broadcast (test wallet not token creator — no fee payout, but call succeeded) |
| 11 | Deploy token on-chain | L4 | N/A | SKIPPED (no key) | — | No partner API key in test env. Dry-run covers API call path (test #9) |
| 12 | `claim-rewards` without args | Error | `claim-rewards` (no --token-address) | PASS (expected error) | — | Clap exits with "required arguments were not provided" |

---

## Fix Log

| # | Problem | Root Cause | Fix | File(s) |
|---|---------|-----------|-----|---------|
| 1 | `list-tokens` returned empty `tokens: []` despite `total: 653336` | Clanker `/api/tokens` API returns `{data: [...], total, cursor}` but code read `result["tokens"]` | Changed `result["tokens"]` → `result["data"]`; updated `has_more` to use `cursor` field | `src/commands/list_tokens.rs` |
| 2 | `claim-rewards --dry-run` failed with `execution reverted` | Pre-computed keccak256 selector for `feeLockerForToken(address)` was `0xd4a786e3` (wrong) — correct selector is `0xb14177cb` | Fixed selector in `resolve_fee_locker()` | `src/rpc.rs` |
| 3 | `pendingRewards(address,address)` call always reverted | Wrong selector `0x5d9e7166` AND wrong function — Clanker V4 locker does NOT have `pendingRewards()`. It uses `tokenRewards(address)` at `0x30bd3eeb` | Replaced `pending_rewards()` with `has_pending_rewards()` using correct selector `0x30bd3eeb` | `src/rpc.rs`, `src/commands/claim_rewards.rs` |
| 4 | `collectFees(address)` calldata selector wrong | Sol macro used `collectFees` (selector `0xa480ca79`) but the V4 locker function is `collectRewards(address)` at `0x5763dbd0` | Changed `sol! { function collectFees(...) }` → `sol! { function collectRewards(...) }` | `src/commands/claim_rewards.rs` |
| 5 | Fallback fee locker address incorrect | `config.rs` had V3 locker `0xF3622742...` as fallback; V4 tokens use `0x63D2DfEA...` | Updated `fallback_fee_locker()` to `0x63D2DfEA64b3433F4071A98665bcD7Ca14d93496` | `src/config.rs` |

---

## On-chain Test Details

### L4-1 — claim-rewards

- **Token:** OkChain (`0xCdA659768E7D7F0F3CCca25185660FD67c7e2B07`, Base)
- **Fee Locker:** `0x63D2DfEA64b3433F4071A98665bcD7Ca14d93496` (Clanker V4 locker)
- **Calldata:** `0x5763dbd0000000000000000000000000cda659768e7d7f0f3ccca25185660fd67c7e2b07`
- **Function:** `collectRewards(address)` — selector `0x5763dbd0`
- **TxHash:** `0x43f0c63d7a1d0806fc28268372fc6827b8890324228a0ad2455cc14187246ab1`
- **Explorer:** https://basescan.org/tx/0x43f0c63d7a1d0806fc28268372fc6827b8890324228a0ad2455cc14187246ab1
- **Note:** Test wallet (`0xee385...`) is not the token's reward recipient — no fee payout occurred but the transaction was accepted by the contract (correct behavior; rewards go to the admin registered with the token, not the caller).

---

## Wallet State (after tests)

- ETH: ~0.00426 (above 0.001 reserve — SAFE)
- USDC: 1.265 (above 0.1 trigger — SAFE)
- USDT: 1.000 (above 0.1 trigger — SAFE)

No funds consumed by read/dry-run tests. One claim-rewards tx was broadcast (gas only, no token transfer to test wallet since it's not the reward recipient).
