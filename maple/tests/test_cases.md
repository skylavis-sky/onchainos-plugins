# Test Cases — Maple Finance Plugin

- DApp: Maple Finance
- Chain: Ethereum (1)
- Date: 2026-04-05

## Level 1: Compile + Lint

| # | Test | Expected |
|---|------|----------|
| L1-1 | cargo build --release | Compile success, 0 errors |
| L1-2 | cargo clean && plugin-store lint | 0 errors |

## Level 2: Read Operations (no wallet, no gas)

| # | Test | Command | Expected |
|---|------|---------|----------|
| L2-1 | List Maple pools | `maple pools --chain 1` | JSON with syrupUSDC + syrupUSDT pools, totalAssets > 0 |
| L2-2 | Show exchange rates | `maple rates --chain 1` | JSON with exchange_rate > 1.0 (yield has accrued) |
| L2-3 | Show positions (no wallet) | `maple positions --chain 1 --from 0x0000000000000000000000000000000000000000` | Returns zero balances for null address |
| L2-4 | Error: invalid pool name | `maple deposit --pool INVALID --amount 0.01 --chain 1 --dry-run` | Error message about unknown pool |

## Level 3: Dry-run Simulation (verify calldata)

| # | Test | Command | Expected calldata |
|---|------|---------|-------------------|
| L3-1 | Deposit dry-run | `maple deposit --pool usdc --amount 0.01 --chain 1 --dry-run` | calldata starts with 0xc9630cb0, txHash zero |
| L3-2 | Withdraw dry-run | `maple withdraw --pool usdc --chain 1 --dry-run` | calldata starts with 0x107703ab, txHash zero |
| L3-3 | Deposit USDT dry-run | `maple deposit --pool usdt --amount 0.01 --chain 1 --dry-run` | calldata starts with 0xc9630cb0 |
| L3-4 | Withdraw with shares dry-run | `maple withdraw --pool usdc --shares 0.5 --chain 1 --dry-run` | calldata has correct shares encoding |

## Level 4: On-chain Write Operations (need lock, spend gas)

| # | Test | Command | Expected |
|---|------|---------|----------|
| L4-1 | Deposit 0.01 USDT into syrupUSDT | `maple deposit --pool usdt --amount 0.01 --chain 1` | txHash on etherscan, 2 txs (approve + deposit) |

Note: Using USDT (15 USDT available) for L4 test per GUARDRAILS.
