# Morpho — Test Results

- Date: 2026-04-04
- Test chain: Base (8453)
- Compiled: ✅
- Lint: ✅

## Results

| Total | Pass | Fail | Blocked |
|-------|------|------|---------|
| 10    | 10   | 0    | 0       |

## Detailed Results

| # | Test | Type | Result | TxHash | Notes |
|---|------|------|--------|--------|-------|
| TC-1 | `markets --chain 1` | read | ✅ PASS | N/A | 9 markets returned on Ethereum Mainnet |
| TC-2 | `markets --chain 8453` | read | ✅ PASS | N/A | 50 markets returned on Base after deserialization fix |
| TC-3 | `vaults --chain 8453` | read | ✅ PASS | N/A | 50 vaults returned on Base after deserialization fix |
| TC-4 | `positions --chain 8453` | read | ✅ PASS | N/A | Empty positions (wallet has no Morpho Blue positions) |
| TC-5 | `--dry-run supply --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --amount 0.01 --chain 8453` | dry-run | ✅ PASS | 0x000...000 | Approve + deposit calldata correct. ERC-4626 selector 0x6e553f65 confirmed |
| TC-6 | `--dry-run withdraw --vault 0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183 --asset USDC --amount 0.01 --chain 8453` | dry-run | ✅ PASS | 0x000...000 | Withdraw calldata correct, selector 0xb460af94 |
| TC-7 | Supply 0.01 USDC to steakUSDC on Base | on-chain | ✅ PASS | approve: `0x0ae84b978d4e6c1cae583eb59955970bb31b78004daa61c742dd97662f51b0d2` deposit: `0x4daca2a21e53ceba7c223fdc1708e43bfc79987d801e19c0a05ebd43c0ea95c2` | Used correct vault `0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183` (Steakhouse USDC). Both txs confirmed on Base (status 0x1). Fixed zero-address receiver bug in supply.rs. |
| TC-8 | `positions --chain 8453` after supply | on-chain | ✅ PASS | N/A | steakUSDC vault position shows 0.01 USDC balance at 3.73% APY. User address correctly resolved. |
| TC-9 | Withdraw 0.01 USDC from steakUSDC | on-chain | ✅ PASS | `0xd7a5534ad7d1afb4e6bdc5abeab6021b9eef447573d5d656db9098872b2fcc35` | Used ERC-4626 withdraw(). Tx confirmed on Base (status 0x1). Fixed zero-address owner/receiver bug in withdraw.rs. |
| TC-10 | `--dry-run borrow --market-id 0xff0f2bd52ca786a4f8149f96622885e880222d8bed12bbbf5950296be8d03f89 --amount 0.001 --chain 8453` | dry-run | ✅ PASS | 0x000...000 | Borrow calldata correct for Morpho Blue (dry-run only per GUARDRAILS.md) |

## Fix Log

| # | Issue | Root Cause | Fix | File |
|---|-------|-----------|-----|------|
| 1 | `--dry-run` flag passed to `onchainos wallet contract-call` which doesn't support it | Plugin was passing `--dry-run` to onchainos, but that CLI flag doesn't exist on `wallet contract-call` | Removed `args.push("--dry-run")` and added early-return with simulated response when `dry_run=true` | `src/onchainos.rs` |
| 2 | `markets --chain 8453` returned 0 markets | GraphQL `orderBy: TotalSupplyUsd` enum value does not exist in Morpho API schema | Removed `orderBy` and `orderDirection` from markets query | `src/api.rs` |
| 3 | `vaults --chain 8453` returned only 1 test vault | Same GraphQL issue: `orderBy: TotalAssetsUsd` enum does not exist | Removed `orderBy` and `orderDirection` from vaults query | `src/api.rs` |
| 4 | `markets` and `vaults` deserializing 0 items despite valid API responses | API returns numeric fields (`supplyAssets`, `borrowAssets`, etc.) as JSON numbers, but Rust structs expected `Option<String>` — `serde` drops items that fail deserialization | Added `deser_number_or_string` custom deserializer; applied to `MarketState`, `PositionState`, `VaultState`, `VaultPosition` numeric fields | `src/api.rs` |
| 5 | `positions --chain 8453` returned GraphQL error on `healthFactor` field | `healthFactor` is not a field on `MarketPositionState` in the Morpho API schema (confirmed via introspection) | Removed `healthFactor` from GraphQL query and from `PositionState` struct; removed `health_factor_status` function | `src/api.rs`, `src/commands/positions.rs` |
| 6 | `supply` and `withdraw` sent funds to/from address(0) when `--from` not provided | `from.unwrap_or("0x0000...0000")` was used as receiver/owner — silently lost funds to zero address | Added `resolve_wallet(from, chain_id)` in `onchainos.rs` that queries active wallet via `onchainos wallet balance`; used in `supply.rs`, `withdraw.rs`, and `positions.rs` | `src/onchainos.rs`, `src/commands/supply.rs`, `src/commands/withdraw.rs`, `src/commands/positions.rs` |
| 7 | Token symbol shows "UNKNOWN" for USDC on Base | `mainnet.base.org` rate-limits aggressively causing `erc20_symbol()` RPC calls to fail | Changed Base RPC URL to `https://base-rpc.publicnode.com` (reliable public endpoint) | `src/config.rs` |

## All Tests Complete

TC-7, TC-8, and TC-9 were unblocked by using the correct vault address `0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183` (Steakhouse USDC). All bugs were fixed and all 10 test cases pass.
