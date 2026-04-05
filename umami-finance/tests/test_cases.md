# Test Cases — Umami Finance Plugin

- DApp: Umami Finance
- Chain: Arbitrum (42161)
- Date: 2026-04-05

## Level 1 — Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|---------|
| 1 | Cargo build (debug) | `cargo build` | 0 errors |
| 2 | Cargo build release | `cargo build --release` | 0 errors |
| 3 | Manual lint: E002 check | Review plugin.yaml api_calls | Pure string list |
| 4 | Manual lint: E106 check | Review SKILL.md write ops | "ask user to confirm" present |
| 5 | Manual lint: .gitignore | Check /target/ excluded | Excluded |

## Level 2 — Read Operations (no wallet, no gas)

| # | Scenario | Command | Expected |
|---|----------|---------|---------|
| 6 | List all vaults | `list-vaults --chain 42161` | 4 vaults, TVL > 0 |
| 7 | Get gmUSDC-eth vault info | `vault-info --vault gmUSDC-eth` | totalAssets, pricePerShare present |
| 8 | Get gmWETH vault info | `vault-info --vault gmWETH` | WETH asset, shares > 0 |
| 9 | Get gmWBTC vault info | `vault-info --vault gmWBTC` | WBTC asset |
| 10 | Invalid vault error | `vault-info --vault invalid` | Error message with guidance |

## Level 3 — Simulate (dry-run, no broadcast)

| # | Scenario | Command | Expected |
|---|----------|---------|---------|
| 11 | Preview deposit into gmUSDC-eth | `deposit --vault gmUSDC-eth --amount 0.01 --dry-run` | `0x6e553f65` selector in calldata |
| 12 | Preview redeem from gmUSDC-eth | `redeem --vault gmUSDC-eth --dry-run` | `0xba087652` selector in calldata |
| 13 | Preview deposit into gmWETH | `deposit --vault gmWETH --amount 0.00005 --dry-run` | calldata with 0x6e553f65 |

## Level 4 — On-chain Write Operations

| # | Scenario | Command | Expected |
|---|----------|---------|---------|
| 14 | Deposit 0.01 USDT worth into gmUSDC-eth | `deposit --vault gmUSDC-eth --amount 0.01` | txHash in response |
