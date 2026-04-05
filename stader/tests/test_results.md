# Test Results Report

- Date: 2026-04-05
- DApp: Stader ETHx Liquid Staking
- DApp supported chains: EVM only (Ethereum Mainnet, chain 1)
- EVM test chain: Ethereum Mainnet (1)
- Compile: ✅
- Lint: ✅ (E123 placeholder SHA expected pre-submission; 0 real errors)
- Overall pass standard: EVM DApp → EVM all pass ✅

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Fail | Blocked |
|-------|-----------|---------|------------|------------|------|---------|
| 12    | 2         | 5       | 4          | 1          | 0    | 0       |

## Detailed Results

| # | Scenario (User View) | Level | Command | Result | TxHash / Calldata | Notes |
|---|---|---|---|---|---|---|
| 1 | Compile debug build | L1 | `cargo build` | ✅ PASS | — | 0 errors, 0 warnings |
| 2 | Lint check | L1 | `cargo clean && plugin-store lint` | ✅ PASS | — | E123 only (placeholder SHA) |
| 3 | Check ETH→ETHx exchange rate | L2 | `stader rates` | ✅ PASS | — | 1 ETHx = 1.086162 ETH, 135K ETH staked |
| 4 | Preview 0.0001 ETH deposit | L2 | `stader rates --preview-amount 100000000000000` | ✅ PASS | — | ETHx out = 92067311796304 wei |
| 5 | Preview 1 ETH deposit | L2 | `stader rates --preview-amount 1000000000000000000` | ✅ PASS | — | ETHx out = 0.920673 (correctly < 1 ETH) |
| 6 | View positions (zero address) | L2 | `stader positions --address 0x...001` | ✅ PASS | — | Empty balance + withdrawals |
| 7 | View positions (contract address) | L2 | `stader positions --address 0xcf5EA1b...` | ✅ PASS | — | Valid JSON, 0 ETHx |
| 8 | Dry-run stake 0.0001 ETH | L3 | `stader --dry-run stake --amount 100000000000000` | ✅ PASS | `0xf340fa01...` | Selector 0xf340fa01 ✅ |
| 9 | Dry-run unstake 0.001 ETHx | L3 | `stader --dry-run unstake --amount 1000000000000000` | ✅ PASS | `0xccc143b8...` | Selector 0xccc143b8 ✅, step1 0x095ea7b3 ✅ |
| 10 | Dry-run claim request 0 | L3 | `stader --dry-run claim --request-id 0` | ✅ PASS | `0x379607f5...` | Selector 0x379607f5 ✅ |
| 11 | Dry-run positions | L3 | `stader --dry-run positions` | ✅ PASS | — | dry_run:true |
| 12 | Stake 0.0001 ETH (minimum deposit) | L4 | `stader stake --amount 100000000000000` | ✅ PASS | `0xb00fe8bf76cf8e89e67d518c948df3677ca25e009e4759a2fb02e14c173b7bbe` | etherscan.io/tx/0xb00fe8bf... |

## L4 Notes

- **L4-1 (stake):** Amount = 0.0001 ETH = protocol minimum. This is 2× GUARDRAILS normal limit (0.00005 ETH) but protocol enforces this minimum. Per GUARDRAILS §Hard Rules rule 1, this requires user approval — user approved.
- **unstake (L4):** SKIPPED — would require ETHx balance; withdrawal finalization takes 3-10 days.
- **claim (L4):** SKIPPED — requires prior unstake to finalize (3-10 days lockup).

## Code Bugs Fixed

| # | Bug | Root Cause | Fix |
|---|---|---|---|
| 1 | `resolve_wallet` failed with EOF on chain 1 | `--output json` flag not supported by `onchainos wallet balance --chain 1` | Changed to `wallet balance --chain <id>` (no --output json), parse from `data.details[0].tokenAssets[0].address` |
