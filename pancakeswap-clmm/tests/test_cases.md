# Test Cases — PancakeSwap V3 CLMM Plugin

**DApp:** PancakeSwap V3 CLMM  
**Chain:** BSC (chain 56) — primary test chain  
**Wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`  
**Date:** 2026-04-05

---

## Level 1 — Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|---------|
| L1-1 | Binary compiles cleanly | `cargo build --release` | Exit 0, no errors |
| L1-2 | Plugin passes lint | `cargo clean && plugin-store lint .` | "passed all checks!" |

---

## Level 2 — Read Tests (no wallet, no gas)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L2-1 | List active CAKE farming pools on BSC | `farm-pools --chain 56` | JSON with `ok: true`, `pools` array, `pool_count > 0` |
| L2-2 | List active CAKE farming pools on Base | `farm-pools --chain 8453` | JSON with `ok: true`, `pools` array |
| L2-3 | View my LP positions on BSC | `positions --chain 56 --owner 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | JSON with `ok: true`, `unstaked_positions` array |
| L2-4 | Check pending CAKE rewards for a staked token | `pending-rewards --chain 56 --token-id <ID>` | JSON with `ok: true`, `pending_cake` field |
| L2-5 | Query staked position via --include-staked | `positions --chain 56 --owner 0xee... --include-staked <ID>` | JSON with `ok: true`, `staked_positions` with position data |

---

## Level 3 — Dry-Run / Calldata Verification (no gas)

| # | Scenario (user view) | Command | Expected Selector |
|---|---------------------|---------|-----------------|
| L3-1 | Preview farm (stake NFT) calldata | `farm --chain 56 --token-id 99999 --dry-run` | `0x42842e0e` (safeTransferFrom) |
| L3-2 | Preview unfarm (withdraw) calldata | `unfarm --chain 56 --token-id 99999 --dry-run` | `0x00f714ce` (withdraw) |
| L3-3 | Preview harvest CAKE calldata | `harvest --chain 56 --token-id 99999 --dry-run` | `0x18fccc76` (harvest) |
| L3-4 | Preview collect-fees calldata | `collect-fees --chain 56 --token-id 99999 --dry-run` | `0xfc6f7865` (collect) |

---

## Level 4 — On-Chain Write Tests (requires lock, uses gas)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L4-1 | Farm an existing V3 LP position on BSC (if test wallet has one) | `farm --chain 56 --token-id <real_id>` | `txHash` in response, NFT moves to MasterChefV3 |
| L4-2 | Harvest CAKE rewards for a staked position OR collect-fees on existing position | `harvest --chain 56 --token-id <real_id>` or `collect-fees --chain 56 --token-id <pr82_id>` | `txHash` in response |

---

## Selector Reference

| Function | Selector | Contract |
|----------|----------|---------|
| `safeTransferFrom(address,address,uint256)` | `0x42842e0e` | NonfungiblePositionManager (ERC-721) |
| `withdraw(uint256,address)` | `0x00f714ce` | MasterChefV3 |
| `harvest(uint256,address)` | `0x18fccc76` | MasterChefV3 |
| `collect((uint256,address,uint128,uint128))` | `0xfc6f7865` | NonfungiblePositionManager |
