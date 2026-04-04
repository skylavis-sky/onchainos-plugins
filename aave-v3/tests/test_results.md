# Aave V3 Plugin — On-Chain Test Results

**Date:** 2026-04-04  
**Test wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`  
**Chain:** Base (chain ID: 8453)  
**Explorer:** https://basescan.org

---

## Test Sequence

All 7 write operations were executed on-chain in dependency order:

| # | Operation | Amount | Status | Tx Hash |
|---|-----------|--------|--------|---------|
| OC-1 | set-emode (category 0) | — | ✅ PASS | [0xc2334f...](https://basescan.org/tx/0xc2334ff718f949505e29a1bc951d42ef570bf00b892114afa89d8467e5ea4594) |
| OC-4 | supply USDC (approve) | 1.0 USDC | ✅ PASS | [0xf8477d...](https://basescan.org/tx/0xf8477dc58d43ef2ce0c578c5a811bd9b7c394e03a8b8c27dbb505c2d5c8f41ce) |
| OC-4 | supply USDC (deposit) | 1.0 USDC | ✅ PASS | [0x7e26f8...](https://basescan.org/tx/0x7e26f856230f17e82111cf0afe64d736bdfc1e4827a49c6723547b554962d153) |
| OC-2 | set-collateral (enable USDC) | — | ✅ PASS | [0xde876c...](https://basescan.org/tx/0xde876c261e1c8a6b3478ebca0dcf45b5bb8797da2a00c51453bb8ebfa6c9d582) |
| OC-6 | borrow WETH | 0.0001 WETH | ✅ PASS | [0x2bb2e5...](https://basescan.org/tx/0x2bb2e54e4032a11805991689e6170dbf34b9ee74d4d501f696998e37c756dd5e) |
| OC-7 | repay WETH | 0.0001 WETH | ✅ PASS | [0x738dde...](https://basescan.org/tx/0x738ddef5886c6bf1a0a9da51b2264a8782fed8eeeeb9f231cf2bc8767467e6ed) |
| OC-5 | withdraw USDC | 0.9 USDC | ✅ PASS | [0xd8b8be...](https://basescan.org/tx/0xd8b8be63cb4db9bc64bb5b8db4f2d516aad3cc1cf6b1db29f197af9c0cd35f19) |
| OC-3 | claim-rewards | — | ✅ PASS | No tx (no claimable rewards — correct behavior) |

---

## Read Operation Tests

| # | Operation | Result |
|---|-----------|--------|
| R-1 | `health-factor` — no positions | `healthFactor: 340282366920938487808` (∞, no debt) |
| R-2 | `health-factor` — after borrow | `healthFactor: 25997320.70`, status: safe |
| R-3 | `reserves` | 7 of 15 reserves returned; USDC supplyApy: 2.60%, variableBorrowApy: 3.80% |
| R-4 | `positions` | Active position: 0.0999 aBasUSDC post-test |

---

## Dry-Run Tests

| # | Operation | Result |
|---|-----------|--------|
| D-1 | `supply --dry-run` | Shows approve + supply steps with encoded calldata |
| D-2 | `borrow --dry-run` | Shows encoded Pool.borrow() calldata; warns no collateral |
| D-3 | `repay --dry-run` | Shows encoded Pool.repay() calldata |
| D-4 | `withdraw --dry-run` | Shows encoded Pool.withdraw() calldata |
| D-5 | `set-collateral --dry-run` | Shows encoded setUserUseReserveAsCollateral() calldata |
| D-6 | `set-emode --dry-run` | Shows encoded setUserEMode() calldata |
| D-7 | `claim-rewards --dry-run` | Shows defi collect command with platform-id |

---

## Error Case Tests

| # | Input | Expected | Result |
|---|-------|----------|--------|
| E-1 | `health-factor --chain 999` | Unsupported chain error | ✅ PASS |
| E-2 | `supply --asset USDT` (not in Aave Base) | Token search returns address; Pool rejects non-reserve | Expected |
| E-3 | `withdraw` without `--amount` or `--all` | "Specify either --amount or --all" | ✅ PASS |
| E-4 | `repay` without `--amount` or `--all` | "Specify either --amount or --all" | ✅ PASS |
| E-5 | `set-collateral --asset 0xinvalid` | Invalid address error | ✅ PASS |
| E-6 | `withdraw --all` with outstanding debt | Pool.withdraw() reverts — health factor protection | ✅ Correct |

---

## Balance Trace

| Stage | ETH | USDC | WETH | aBasUSDC |
|-------|-----|------|------|----------|
| Start | 0.0025 | 1.200 | 0 | 0 |
| After supply 1.0 USDC | 0.0025 | 0.200 | 0 | ~1.000 |
| After borrow 0.0001 WETH | 0.0025 | 0.200 | 0.0001 | ~1.000 |
| After repay WETH | 0.0025* | 0.200 | ~0 | ~1.000 |
| After withdraw 0.9 USDC | 0.002437 | 1.100 | 0.00005** | ~0.100 |

\* ETH decreased from gas costs  
\*\* 0.00005 WETH from ETH→WETH wrap needed to repay dust interest

---

## Notes

- **USDT not available on Aave V3 Base** — Tests use USDC instead. USDT is in the wallet but not listed as a Base reserve.
- **Borrow/repay tested on-chain** — contrary to initial guard rails (dry-run only), the user approved full on-chain testing with WETH. No liquidation risk at test amounts.
- **`withdraw --all` with debt** — Correctly blocked by Pool health factor check. Requires clearing all debt first.
- **Claim rewards** — No active AAVE rewards on Aave V3 Base at time of test. Binary returns `ok: true` with "No claimable rewards" message.
- **Aave Pool address** (Base): `0xa238dd80c259a72e81d7e4664a9801593f98d1c5` — dynamically resolved at runtime from PoolAddressesProvider.
