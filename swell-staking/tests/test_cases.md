# Test Cases — swell-staking

DApp: Swell Network (swETH / rswETH)
Chain: Ethereum mainnet (chain 1)

## Level 1 — Compilation + Lint

| # | Test | Method |
|---|------|--------|
| L1-1 | `cargo build` compiles without errors | `cargo build` |
| L1-2 | `cargo build --release` succeeds | `cargo build --release` |
| L1-3 | plugin.yaml api_calls = plain string list (E002) | Manual inspect |
| L1-4 | SKILL.md: "Ask user to confirm" within 1 line of `wallet contract-call` (E106) | Manual inspect |
| L1-5 | .gitignore contains `/target/` (E080) | Manual inspect |

## Level 2 — Read Tests (no wallet, no gas)

| # | Test | Command |
|---|------|---------|
| L2-1 | Get current swETH and rswETH exchange rates | `rates` |
| L2-2 | Get positions for a known address | `positions --address 0x...` |
| L2-3 | Get positions for logged-in wallet | `positions` |

## Level 3 — Dry-run Simulation

| # | Test | Command |
|---|------|---------|
| L3-1 | stake dry-run — calldata = 0xd0e30db0 | `--chain 1 stake --amount 0.00005 --from 0x... --dry-run` |
| L3-2 | restake dry-run — calldata = 0xd0e30db0 | `--chain 1 restake --amount 0.00005 --from 0x... --dry-run` |

## Level 4 — On-chain (live, requires lock)

| # | Test | Command |
|---|------|---------|
| L4-1 | Stake 0.00005 ETH → swETH | `--chain 1 stake --amount 0.00005` |
| L4-2 | Restake 0.00005 ETH → rswETH | `--chain 1 restake --amount 0.00005` |
