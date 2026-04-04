# Test Results Report — Compound V3

- **Date:** 2026-04-05
- **Test chain:** Base (8453) + Ethereum (1) for L2
- **Compile:** PASS
- **Lint:** PASS

---

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|-----------|---------|-------------|-------------|--------|---------|
| 14    | 2         | 5       | 6           | 2           | 0      | 0       |

All tests PASS after fixes. 5 bugs found and fixed in Phase 3.

---

## Detailed Results

| # | Scenario (user view) | Level | Command | Result | TxHash / Calldata | Notes |
|---|---------------------|-------|---------|--------|-------------------|-------|
| 1 | Compile plugin binary | L1 | `cargo build --release` | PASS | — | 3 dead-code warnings, no errors |
| 2 | Lint plugin | L1 | `cargo clean && plugin-store lint .` | PASS | — | 0 errors |
| 3 | View USDC market stats on Base | L2 | `compound-v3 --chain 8453 --market usdc get-markets` | PASS | — | 83.76% utilization, supply 3.02% APR, borrow 3.83% APR |
| 4 | View USDC market stats on Ethereum | L2 | `compound-v3 --chain 1 --market usdc get-markets` | PASS | — | 69.81% utilization after RPC fix |
| 5 | View my position on Base (empty) | L2 | `compound-v3 --chain 8453 --market usdc get-position --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | PASS | — | supply=0, borrow=0, is_borrow_collateralized=true |
| 6 | View position with WETH collateral check | L2 | `compound-v3 --chain 8453 --market usdc get-position --wallet 0x... --collateral-asset 0x4200...0006` | PASS | — | collateral.balance_raw=0 |
| 7 | Error on unsupported chain | L2 | `compound-v3 --chain 99999 --market usdc get-markets` | PASS | — | Returns `ok:false` with clear error message |
| 8 | Preview supplying WETH collateral | L3 | `compound-v3 --chain 8453 --market usdc --dry-run supply --asset 0x4200...0006 --amount 50000000000000 --from 0xee3...` | PASS | calldata: `0xf2b9fdb8000...` | Selector `0xf2b9fdb8` (Comet.supply) correct |
| 9 | Preview supplying 0.01 USDC | L3 | `compound-v3 --chain 8453 --market usdc --dry-run supply --asset 0x833...913 --amount 10000 --from 0xee3...` | PASS | calldata: `0xf2b9fdb8000...2710` | Selector `0xf2b9fdb8` correct; amount 0x2710 = 10000 |
| 10 | Preview borrowing 1 USDC (dry-run only) | L3 | `compound-v3 --chain 8453 --market usdc --dry-run borrow --amount 1000000 --from 0xee3...` | PASS | calldata: `0xf3fef3a3000...f4240` | Selector `0xf3fef3a3` (Comet.withdraw) correct |
| 11 | Preview repay when no debt | L3 | `compound-v3 --chain 8453 --market usdc --dry-run repay --from 0xee3...` | PASS | — | Returns "No outstanding borrow balance to repay" |
| 12 | Preview withdrawing WETH collateral | L3 | `compound-v3 --chain 8453 --market usdc --dry-run withdraw --asset 0x4200...0006 --amount 50000000000000 --from 0xee3...` | PASS | calldata: `0xf3fef3a3000...2000` | Selector `0xf3fef3a3` correct |
| 13 | Preview claiming COMP rewards (none available) | L3 | `compound-v3 --chain 8453 --market usdc --dry-run claim-rewards --from 0xee3...` | PASS | — | Returns "No claimable COMP rewards at this time." — correct behavior |
| 14 | Supply 0.01 USDC to Compound V3 Base | L4 | `compound-v3 --chain 8453 --market usdc supply --asset 0x833...913 --amount 10000 --from 0xee3...` | PASS | approve: `0xd6dc75ca625822cb4a50ac56e238e487b850c1d0066139fb8f76497c3d1f9248`  supply: `0x1585c20c3fc0841041038aceaeaf03759ee0c0c4ac61fdf46521465f64623c7f` | Supply confirmed: new balance 0.009999 USDC |
| 15 | Withdraw 0.009999 USDC from Compound V3 Base | L4 | `compound-v3 --chain 8453 --market usdc withdraw --asset 0x833...913 --amount 9999 --from 0xee3...` | PASS | withdraw: `0xded9f12569767b19db381db8645b9c46623e8b5fa9f9075b02a23980395cb1e4` | Withdrew full supply balance |

**L4 Block Explorer:**
- Approve tx: https://basescan.org/tx/0xd6dc75ca625822cb4a50ac56e238e487b850c1d0066139fb8f76497c3d1f9248
- Supply tx: https://basescan.org/tx/0x1585c20c3fc0841041038aceaeaf03759ee0c0c4ac61fdf46521465f64623c7f
- Withdraw tx: https://basescan.org/tx/0xded9f12569767b19db381db8645b9c46623e8b5fa9f9075b02a23980395cb1e4

---

## Fix Log

| # | Issue | Root Cause | Fix | File |
|---|-------|-----------|-----|------|
| 1 | `get-markets` returned `execution reverted` on `getUtilization()` | Wrong function selector `0xd7a5b8ab` — keccak256("getUtilization()") = `0x7eb71131`, not `0xd7a5b8ab` | Changed selector to `0x7eb71131` | `src/rpc.rs` |
| 2 | `get-markets` would fail on `baseBorrowMin()` | Wrong function selector `0x29f2a836` — correct: keccak256("baseBorrowMin()") = `0x300e6beb` | Changed selector to `0x300e6beb` | `src/rpc.rs` |
| 3 | `get-position --collateral-asset` returned `execution reverted` | Wrong function selector for `collateralBalanceOf(address,address)`: `0x487dd147` — correct: `0x5c2549ee` | Changed selector to `0x5c2549ee` | `src/rpc.rs` |
| 4 | `isBorrowCollateralized(address)` returned `execution reverted` | Wrong selector `0x0f3bde75` — correct: `0x38aa813f` | Changed selector to `0x38aa813f` | `src/rpc.rs` |
| 5 | `getRewardOwed(address,address)` on CometRewards returned `execution reverted` | Wrong selector `0xfd27b525` — correct: `0x41e0cad6`; also `claimTo(address,address,address,bool)` wrong selector `0x52a4ef2e` — correct: `0x4ff85d94` | Updated both selectors | `src/rpc.rs`, `src/commands/claim_rewards.rs` |
| 6 | `get-markets --chain 1` failed with rate-limit error | `eth.llamarpc.com` returns HTTP 429 rate-limit error; plugin was using it for Ethereum mainnet | Switched Ethereum RPC to `https://ethereum.publicnode.com` | `src/config.rs`, `plugin.yaml` |
