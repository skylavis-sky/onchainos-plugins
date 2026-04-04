# Compound V3 — Test Cases

**Plugin:** compound-v3  
**Binary:** `compound-v3`  
**Test Chain:** Base (8453)  
**Date:** 2026-04-05  

---

## Level 1 — Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|----------|
| L1-1 | Build binary | `cargo build --release` | Exit 0, binary produced |
| L1-2 | Lint | `cargo clean && plugin-store lint .` | 0 errors |

---

## Level 2 — Read Tests (no wallet needed)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|----------|
| L2-1 | View USDC market stats on Base | `compound-v3 --chain 8453 --market usdc get-markets` | `ok:true`, supply_apr_pct, borrow_apr_pct, total_supply, utilization_pct present |
| L2-2 | View USDC market stats on Ethereum | `compound-v3 --chain 1 --market usdc get-markets` | `ok:true`, valid market data for Ethereum Comet |
| L2-3 | View my position on Base (with explicit wallet) | `compound-v3 --chain 8453 --market usdc get-position --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok:true`, supply_balance, borrow_balance, is_borrow_collateralized present |
| L2-4 | View position with collateral asset check | `compound-v3 --chain 8453 --market usdc get-position --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 --collateral-asset 0x4200000000000000000000000000000000000006` | `ok:true`, collateral.balance_raw present for WETH |
| L2-5 | Error on unsupported chain/market | `compound-v3 --chain 99999 --market usdc get-markets` | `ok:false`, error message about unsupported chain |

---

## Level 3 — Simulate / Dry-run (no gas, calldata verification)

| # | Scenario (user view) | Command | Expected Selector |
|---|---------------------|---------|-------------------|
| L3-1 | Preview supplying WETH collateral (0.00005 ETH worth) | `compound-v3 --chain 8453 --market usdc --dry-run supply --asset 0x4200000000000000000000000000000000000006 --amount 50000000000000 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | calldata: `0xf2b9fdb8` (Comet.supply) |
| L3-2 | Preview supplying USDC (0.01 = 10000 raw) | `compound-v3 --chain 8453 --market usdc --dry-run supply --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --amount 10000 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | calldata: `0xf2b9fdb8` (Comet.supply) |
| L3-3 | Preview borrowing USDC (dry-run only per GUARDRAILS) | `compound-v3 --chain 8453 --market usdc --dry-run borrow --amount 1000000 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | calldata: `0xf3fef3a3` (Comet.withdraw/borrow) |
| L3-4 | Preview repaying debt (dry-run only per GUARDRAILS) | `compound-v3 --chain 8453 --market usdc --dry-run repay --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | calldata: `0xf2b9fdb8` (Comet.supply/repay) |
| L3-5 | Preview withdrawing WETH collateral | `compound-v3 --chain 8453 --market usdc --dry-run withdraw --asset 0x4200000000000000000000000000000000000006 --amount 50000000000000 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | calldata: `0xf3fef3a3` (Comet.withdraw) |
| L3-6 | Preview claiming COMP rewards on Base | `compound-v3 --chain 8453 --market usdc --dry-run claim-rewards --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok:true`, either no rewards msg or calldata: `0x52a4ef2e` |

---

## Level 4 — On-Chain Tests (requires lock, minimal amounts)

Per GUARDRAILS.md: supply → withdraw are the L4 on-chain ops. Borrow/repay are dry-run only.

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|----------|
| L4-1 | Supply 0.01 USDC to Compound V3 Base market | `compound-v3 --chain 8453 --market usdc supply --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --amount 10000 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok:true`, approve_tx_hash + supply_tx_hash (real txHashes), new supply balance > 0 |
| L4-2 | Withdraw 0.01 USDC from Compound V3 Base market | `compound-v3 --chain 8453 --market usdc withdraw --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --amount 10000 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok:true`, withdraw_tx_hash (real txHash) |

**Note:** L4-1 must run before L4-2. L4-2 (withdraw) requires zero borrow balance.  
**Note:** Amounts: 10000 = 0.01 USDC (6 decimals). GUARDRAILS max = 0.01 USDT per tx.

---

## Error Handling Tests

| # | Scenario | Command | Expected |
|---|---------|---------|----------|
| E-1 | Unsupported chain | `compound-v3 --chain 999 --market usdc get-markets` | `ok:false`, unsupported chain error |
| E-2 | Supply without wallet (no --from, not logged in to onchainos) | dry-run only verification above covers this path |
