# Compound V3 — Test Cases (v2 — Arbitrum USDC Bug Fix)

**Plugin:** compound-v3  
**Binary:** `compound-v3`  
**Supported Chains:** Ethereum (1), Base (8453), Arbitrum (42161), Polygon (137)  
**Primary Test Chain:** Base (8453) — baseline  
**Bug Fix Validation Chain:** Arbitrum (42161) — borrow/repay with native USDC  
**Date:** 2026-04-11  
**Change from v1:** Added Arbitrum borrow/repay L4 live tests to validate fix for USDC.e→native USDC bug.  
**Bug:** `src/config.rs:40` Arbitrum base_asset was `0xFF970A61...` (USDC.e); comet `baseToken()` returns `0xaf88d065...` (native USDC). Caused `arithmetic underflow` on borrow/repay.

---

## Pre-conditions

### Wallet Setup

Resolve wallet dynamically before running any L4 test:

```bash
WALLET=$(onchainos wallet balance --chain 8453 --output json | python3 -c "import json,sys; print(json.load(sys.stdin)['data']['address'])")
echo "Test wallet: $WALLET"
```

### Arbitrum L4 Pre-conditions (⚠️ required before Arbitrum borrow/repay tests)

The wallet must have on **Arbitrum (chain 42161)**:

| Asset | Required | Purpose |
|-------|----------|---------|
| WETH (`0x82aF49447D8a07e3bd95BD0d56f35241523fBab1`) | ≥ 0.00005 ETH (50_000_000_000_000 wei) | Collateral for borrow test |
| ETH (native) | ≥ 0.0005 ETH | Gas for 4 transactions |

> **Note:** Native USDC is NOT required upfront — the borrow step delivers it to the wallet, and repay uses that balance immediately.  
> **Cost to complete Arbitrum L4 suite:** ~$0.05–$0.10 in gas + 0.00005 WETH locked temporarily (returned via withdraw). No net USDC cost if repay runs immediately after borrow.

---

## Level 1 — Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|----------|
| L1-1 | Build binary with fix | `cargo build --release 2>&1` | Exit 0, binary produced |
| L1-2 | Lint (must cargo clean first) | `cargo clean && plugin-store lint .` | 0 errors, 0 warnings |

---

## Level 2 — Read Tests (no wallet, no gas)

### All 4 Chains — get-markets

| # | Scenario | Command | Expected |
|---|---------|---------|----------|
| L2-1 | Base market stats | `compound-v3 --chain 8453 --market usdc get-markets` | `ok:true`, supply_apr_pct, borrow_apr_pct, utilization_pct, total_supply present |
| L2-2 | Ethereum market stats | `compound-v3 --chain 1 --market usdc get-markets` | `ok:true`, valid market data |
| L2-3 | **Arbitrum market stats** | `compound-v3 --chain 42161 --market usdc get-markets` | `ok:true`, valid market data (RPC must not 429) |
| L2-4 | Polygon market stats | `compound-v3 --chain 137 --market usdc get-markets` | `ok:true`, valid market data |
| L2-5 | Unsupported chain error | `compound-v3 --chain 99999 --market usdc get-markets` | `ok:false`, error references supported chains |

### get-position (read-only, use any funded address)

| # | Scenario | Command | Expected |
|---|---------|---------|----------|
| L2-6 | Position on Base | `compound-v3 --chain 8453 --market usdc get-position --wallet $WALLET` | `ok:true`, supply_balance, borrow_balance, is_borrow_collateralized |
| L2-7 | **Position on Arbitrum** | `compound-v3 --chain 42161 --market usdc get-position --wallet $WALLET` | `ok:true`, all balance fields present |
| L2-8 | Collateral check on Base | `compound-v3 --chain 8453 --market usdc get-position --wallet $WALLET --collateral-asset 0x4200000000000000000000000000000000000006` | `ok:true`, collateral.balance_raw present |
| L2-9 | **Collateral check on Arbitrum (WETH)** | `compound-v3 --chain 42161 --market usdc get-position --wallet $WALLET --collateral-asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | `ok:true`, collateral.balance_raw present |

---

## Level 3 — Dry-run / Calldata Verification

### ⚠️ Key Verification: Arbitrum Borrow + Repay Token Addresses

These are the critical L3 checks that prove the USDC.e bug is fixed in the binary.

| # | Scenario | Command | What to verify |
|---|---------|---------|----------------|
| L3-ARB-BORROW | **[BUG FIX CHECK] Preview borrow on Arbitrum** | `compound-v3 --chain 42161 --market usdc --dry-run borrow --amount 10000 --from $WALLET` | `steps[0].calldata` must contain `af88d065e77c8cc2239327c5edb3a432268e5831` (native USDC). Must NOT contain `ff970a61a04b1ca14834a43f5de4533ebddb5cc8` (USDC.e). Selector: `0xf3fef3a3` |
| L3-ARB-REPAY | **[BUG FIX CHECK] Preview repay on Arbitrum** | `compound-v3 --chain 42161 --market usdc --dry-run repay --amount 10000 --from $WALLET` | `steps[0].token` (ERC-20 approve target) must equal `0xaf88d065e77c8cC2239327C5EDb3A432268e5831`. `steps[2].calldata` selector: `0xf2b9fdb8` |

### Base (8453) — Full Command Coverage

| # | Scenario | Command | Expected Selector / Output |
|---|---------|---------|----------------------------|
| L3-1 | Preview supply WETH collateral | `compound-v3 --chain 8453 --market usdc --dry-run supply --asset 0x4200000000000000000000000000000000000006 --amount 50000000000000 --from $WALLET` | Steps: approve (0x095ea7b3) → supply (0xf2b9fdb8) |
| L3-2 | Preview supply USDC (repay path) | `compound-v3 --chain 8453 --market usdc --dry-run supply --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --amount 10000 --from $WALLET` | Steps: approve → supply (0xf2b9fdb8). Token = `0x833589...` |
| L3-3 | Preview borrow USDC | `compound-v3 --chain 8453 --market usdc --dry-run borrow --amount 10000 --from $WALLET` | Selector `0xf3fef3a3`, base_asset in calldata = `0x833589...` (native USDC Base) |
| L3-4 | Preview repay-all | `compound-v3 --chain 8453 --market usdc --dry-run repay --from $WALLET` | No borrow balance → `{"message":"No outstanding borrow balance to repay."}` OR approve (0x095ea7b3) + supply (0xf2b9fdb8) |
| L3-5 | Preview partial repay | `compound-v3 --chain 8453 --market usdc --dry-run repay --amount 10000 --from $WALLET` | Approve token = `0x833589...`, supply calldata present |
| L3-6 | Preview withdraw WETH | `compound-v3 --chain 8453 --market usdc --dry-run withdraw --asset 0x4200000000000000000000000000000000000006 --amount 50000000000000 --from $WALLET` | Selector `0xf3fef3a3`, asset in calldata = `0x42000...` |
| L3-7 | Preview claim-rewards | `compound-v3 --chain 8453 --market usdc --dry-run claim-rewards --from $WALLET` | `ok:true` — either "No claimable rewards" or claimTo calldata (0x4ff85d94) |

### Arbitrum (42161) — Supply/Withdraw Preview

| # | Scenario | Command | Expected |
|---|---------|---------|----------|
| L3-8 | **Preview supply WETH on Arbitrum** | `compound-v3 --chain 42161 --market usdc --dry-run supply --asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 --amount 50000000000000 --from $WALLET` | approve + supply(0xf2b9fdb8), spender = Arbitrum comet `0x9c4ec768...` |
| L3-9 | **Preview withdraw WETH on Arbitrum** | `compound-v3 --chain 42161 --market usdc --dry-run withdraw --asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 --amount 50000000000000 --from $WALLET` | Selector `0xf3fef3a3`, asset in calldata = `0x82aF49...` |

---

## Level 4 — On-Chain Live Transactions

### Base (8453) — Baseline (existing tests, supply + withdraw only)

| # | Scenario | Command | Expected |
|---|---------|---------|----------|
| L4-B1 | Supply 0.01 USDC on Base | `compound-v3 --chain 8453 --market usdc supply --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --amount 10000 --from $WALLET` | `ok:true`, approve_tx_hash + supply_tx_hash, new supply balance > 0 |
| L4-B2 | Withdraw 0.01 USDC on Base | `compound-v3 --chain 42161 --market usdc withdraw --asset 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 --amount 10000 --from $WALLET` | `ok:true`, withdraw_tx_hash |

> L4-B2 depends on L4-B1.

---

### ⚠️ Arbitrum (42161) — Borrow/Repay Bug Fix Validation

**These are the primary new tests for the fix. Must run in strict sequence.**

**Verify wallet is funded before starting:**
```bash
onchainos wallet balance --chain 42161 --output json
# Confirm: WETH balance ≥ 50000000000000 (50_000_000_000_000 wei = 0.00005 WETH)
# Confirm: ETH balance ≥ 0.0005 ETH (gas reserve)
```

| # | Scenario | Command | Expected | Validates |
|---|---------|---------|----------|-----------|
| L4-A1 | **Supply WETH collateral on Arbitrum** | `compound-v3 --chain 42161 --market usdc supply --asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 --amount 50000000000000 --from $WALLET` | `ok:true`, approve_tx_hash + supply_tx_hash | Collateral supply works |
| L4-A2 | **get-position after supply** | `compound-v3 --chain 42161 --market usdc get-position --wallet $WALLET --collateral-asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` | collateral.balance_raw ≈ 50000000000000 | Position tracking |
| L4-A3 | **🔴 Borrow 0.01 USDC on Arbitrum** | `compound-v3 --chain 42161 --market usdc borrow --amount 10000 --from $WALLET` | `ok:true`, borrow_tx_hash, new_borrow_balance ≈ "0.010000" USDC. **Must NOT return arithmetic underflow** | **Core bug fix: native USDC address in Comet.withdraw calldata** |
| L4-A4 | **get-position after borrow** | `compound-v3 --chain 42161 --market usdc get-position --wallet $WALLET` | borrow_balance ≈ "0.010000" | Borrow tracking |
| L4-A5 | **🔴 Repay debt on Arbitrum (repay-all)** | `compound-v3 --chain 42161 --market usdc repay --from $WALLET` | `ok:true`, approve_tx_hash + repay_tx_hash, remaining_borrow_balance = "0.000000". **Must NOT revert on ERC-20 approve** | **Core bug fix: native USDC address in ERC-20 approve + Comet.supply** |
| L4-A6 | **get-position after repay** | `compound-v3 --chain 42161 --market usdc get-position --wallet $WALLET` | borrow_balance = "0.000000", is_borrow_collateralized = true | Zero-debt state |
| L4-A7 | **Withdraw WETH collateral on Arbitrum** | `compound-v3 --chain 42161 --market usdc withdraw --asset 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 --amount 50000000000000 --from $WALLET` | `ok:true`, withdraw_tx_hash | Collateral recovery |
| L4-A8 | **get-position cleanup check** | `compound-v3 --chain 42161 --market usdc get-position --wallet $WALLET` | supply_balance = "0.000000", borrow_balance = "0.000000" | Clean state |

> ⚠️ **L4-A3 and L4-A5 are the definitive tests for the bug fix.**
> - **Pass**: tx broadcasts, returns txHash, no arithmetic underflow → native USDC address is correct
> - **Fail**: `arithmetic underflow or overflow` → the binary was not rebuilt with the fix

> **Abort rule:** If L4-A1 fails, stop the entire Arbitrum suite. If L4-A3 fails with arithmetic underflow, it means the binary was not rebuilt — do NOT proceed to L4-A5.

---

## Selector Reference

| Method | Selector | Used In |
|--------|----------|---------|
| `supply(address,uint256)` | `0xf2b9fdb8` | supply, repay |
| `withdraw(address,uint256)` | `0xf3fef3a3` | borrow, withdraw |
| `approve(address,uint256)` | `0x095ea7b3` | ERC-20 approve step |
| `claimTo(address,address,address,bool)` | `0x4ff85d94` | claim-rewards |

## Token Reference

| Token | Chain | Address |
|-------|-------|---------|
| USDC (native) | Base 8453 | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| USDC (native) ← **was USDC.e bug** | Arbitrum 42161 | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |
| USDC.e (bridged) ← **wrong, do not use** | Arbitrum 42161 | `0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8` |
| WETH | Base 8453 | `0x4200000000000000000000000000000000000006` |
| WETH | Arbitrum 42161 | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` |
