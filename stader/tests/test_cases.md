# Stader Test Cases

Generated: 2026-04-05
Chain: Ethereum Mainnet (1) — EVM only DApp

## Level 1 — Compile + Lint

| # | Test | Expected |
|---|------|----------|
| L1-1 | `cargo build` | Compiles without errors |
| L1-2 | `cargo clean && plugin-store lint .` | 0 errors (E123 placeholder is expected pre-PR) |

## Level 2 — Read Tests (no wallet, no gas)

| # | Scenario (User View) | Command | Expected |
|---|---|---|---|
| L2-1 | Check current ETH→ETHx exchange rate | `stader rates` | JSON with exchange_rate, total_eth_staked, deposit_limits |
| L2-2 | Preview 0.0001 ETH deposit | `stader rates --preview-amount 100000000000000` | ethx_out non-zero, vault_healthy true |
| L2-3 | Preview 1 ETH deposit | `stader rates --preview-amount 1000000000000000000` | ETHx < 1 ETH (rate > 1) |
| L2-4 | View positions of zero-balance address | `stader positions --address 0x000...001` | Empty withdrawals array |
| L2-5 | View positions of known Stader user | `stader positions --address 0xcf5EA1b38380f6aF39068375516Daf40Ed70D299` | valid response |

## Level 3 — Simulation Tests (dry-run, no gas)

| # | Scenario (User View) | Command | Expected Selector |
|---|---|---|---|
| L3-1 | Dry-run: stake 0.0001 ETH | `stader --dry-run stake --amount 100000000000000` | `0xf340fa01` |
| L3-2 | Dry-run: unstake 0.001 ETHx | `stader --dry-run unstake --amount 1000000000000000` | `0xccc143b8` |
| L3-3 | Dry-run: claim request 0 | `stader --dry-run claim --request-id 0` | `0x379607f5` |
| L3-4 | Dry-run: positions | `stader --dry-run positions` | dry_run:true |

## Level 4 — On-chain Tests (real tx, needs lock)

⚠️ NOTE: Stader minimum deposit is 0.0001 ETH (2× GUARDRAILS normal limit of 0.00005 ETH).
Per GUARDRAILS §Hard Rules rule 1: "if a protocol requires a larger minimum amount, prompt the user explaining why and request approval before proceeding." User has been informed and approves.

| # | Scenario (User View) | Command | Notes |
|---|---|---|---|
| L4-1 | Stake 0.0001 ETH (minimum) to receive ETHx | `stader stake --amount 100000000000000` | Only L4 test; wallet must have >0.001+0.0001 ETH reserve |
| L4-SKIP | Unstake ETHx | SKIPPED | Would require ETHx balance; withdrawal takes 3-10 days |
| L4-SKIP | Claim withdrawal | SKIPPED | Requires prior unstake to finalize (3-10 days) |
