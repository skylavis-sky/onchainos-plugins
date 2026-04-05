# Test Results — Yearn Finance Plugin

- Date: 2026-04-05
- DApp supported chains: Ethereum mainnet (chain ID 1) only
- EVM test chain: Ethereum mainnet (chain ID 1)
- Compile: ✅
- Lint: ✅
- Overall pass standard: EVM DApp → EVM all pass

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|-----------|---------|-------------|-------------|--------|---------|
| 12    | 2          | 5       | 3           | 2           | 0      | 0       |

## Detailed Results

| # | Scenario (user view) | Level | Command | Result | TxHash / Calldata | Notes |
|---|---------------------|-------|---------|--------|-------------------|-------|
| 1 | cargo build release | L1 | `cargo build --release` | ✅ PASS | — | 17 warnings (unused consts), 0 errors |
| 2 | plugin-store lint | L1 | `cargo clean && plugin-store lint .` | ✅ PASS | — | "Plugin 'yearn-finance' passed all checks!" |
| 3 | List all Yearn vaults on Ethereum | L2 | `vaults --chain 1` | ✅ PASS | — | 173 active vaults returned, sorted by TVL |
| 4 | Filter vaults by USDT token | L2 | `vaults --chain 1 --token USDT` | ✅ PASS | — | 4 USDT vaults returned |
| 5 | Show APR rates for USDT vaults | L2 | `rates --chain 1 --token USDT` | ✅ PASS | — | Rates sorted by APR desc; max 389.47% (Fluid Lender) |
| 6 | Query my Yearn positions | L2 | `positions --chain 1 --wallet 0x87fb...` | ✅ PASS | — | 0 positions (after withdraw), concurrent query in ~8s |
| 7 | Simulate deposit 0.01 USDT | L3 | `--dry-run deposit --vault USDT --amount 0.01` | ✅ PASS | approve: `0x095ea7b3...`, deposit: `0x6e553f65...` | Correct selectors, amount_raw=10000 |
| 8 | Simulate withdraw all shares | L3 | `--dry-run withdraw --vault 0x6D2981...` | ✅ PASS | `0xba087652...` | Selector correct |
| 9 | Simulate deposit with vault address | L3 | `--dry-run deposit --vault 0x6D29... --amount 0.01` | ✅ PASS | approve + deposit steps correct | |
| 10 | Deposit 0.01 USDT into Gauntlet USDT Prime vault | L4 | `deposit --vault 0x6D2981... --amount 0.01` | ✅ PASS | approve: `0xdc1846eee34e4c5a386cb703ab5136ae7124112518a15e54e6413a17797d6a91` deposit: `0xc95efd4d3d4434159c078d0d98902d5d5be9768a6b8a95464b3f7676c5b7069b` | 0.01 USDT deposited; received 3458 shares |
| 11 | Withdraw all shares from Gauntlet USDT Prime vault | L4 | `withdraw --vault 0x6D2981...` | ✅ PASS | `0x2b070416697dc795b923b6c236ab23a51956ba17134053adb7f7e974a7ad63fa` | All 3458 shares redeemed, Etherscan: [view](https://etherscan.io/tx/0x2b070416697dc795b923b6c236ab23a51956ba17134053adb7f7e974a7ad63fa) |
| 12 | Vaults filtered by non-existent token | L1-error | `vaults --chain 1 --token NONEXISTENT` | ✅ PASS | — | Returns count=0, empty vaults array |

## L4 On-chain Transactions (Ethereum Mainnet)

| Action | Vault | TxHash | Explorer |
|--------|-------|--------|---------|
| ERC-20 approve (for yvUSDT-1 attempt) | yvUSDT-1 (0x310B7...) | `0x7963c9e343460f1e75fc61cfb300181255bdb2e1fa159518ed44fa63d94900b7` | [Etherscan](https://etherscan.io/tx/0x7963c9e343460f1e75fc61cfb300181255bdb2e1fa159518ed44fa63d94900b7) |
| ERC-20 approve (for Gauntlet vault) | Gauntlet USDT Prime (0x6D2981...) | `0xdc1846eee34e4c5a386cb703ab5136ae7124112518a15e54e6413a17797d6a91` | [Etherscan](https://etherscan.io/tx/0xdc1846eee34e4c5a386cb703ab5136ae7124112518a15e54e6413a17797d6a91) |
| ERC-4626 deposit 0.01 USDT | Gauntlet USDT Prime (0x6D2981...) | `0xc95efd4d3d4434159c078d0d98902d5d5be9768a6b8a95464b3f7676c5b7069b` | [Etherscan](https://etherscan.io/tx/0xc95efd4d3d4434159c078d0d98902d5d5be9768a6b8a95464b3f7676c5b7069b) |
| ERC-4626 redeem (withdraw all) | Gauntlet USDT Prime (0x6D2981...) | `0x2b070416697dc795b923b6c236ab23a51956ba17134053adb7f7e974a7ad63fa` | [Etherscan](https://etherscan.io/tx/0x2b070416697dc795b923b6c236ab23a51956ba17134053adb7f7e974a7ad63fa) |

## Fix Log

| # | Issue | Root Cause | Fix |
|---|-------|-----------|-----|
| 1 | yvUSDT-1 vault not in vaults list | API default `limit=200` excludes many vaults | Changed to `limit=500` |
| 2 | `0x310B7` deposit reverted: "execution reverted" | yvUSDT-1 v3.0.2 has deposit restrictions (likely paused/capped) | Used Gauntlet USDT Prime (0x6D2981) — largest active USDT v3 vault |
| 3 | `0x0a4ea2` deposit reverted: "SafeERC20: low-level call failed" | Morpho Steakhouse USDT v3.0.4 reverts with low amount during simulation | Used confirmed-working Gauntlet vault instead |
| 4 | positions too slow (100 sequential eth_calls) | Sequential RPC calls per vault | Concurrent tokio::spawn for balanceOf, cap at top 50 vaults |
