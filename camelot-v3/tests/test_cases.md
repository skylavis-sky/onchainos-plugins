# Test Cases — Camelot V3

DApp supports: EVM only (Arbitrum, chain 42161)

## L1 — Build + Lint

| # | Description | Command | Expected |
|---|-------------|---------|----------|
| 1 | Build release binary | `cargo build --release` | 0 errors, 0 warnings |

## L2 — Read Tests (no gas)

| # | Scenario | Command | Expected |
|---|----------|---------|----------|
| 2 | Quote 0.001 ETH → USDT | `quote --token-in WETH --token-out USDT --amount-in 1000000000000000 --chain 42161` | ok=true, amount_out>0, pool address |
| 3 | Quote 1 USDT → WETH | `quote --token-in USDT --token-out WETH --amount-in 1000000 --chain 42161` | ok=true, amount_out>0 |
| 4 | List LP positions for test wallet | `positions --chain 42161` | ok=true, total=0 (empty wallet) |
| 5 | Quote non-existent pair | `quote --token-in GRAIL --token-out ARB --amount-in 1000000 --chain 42161` | error: no pool found |

## L3 — Dry-run Tests (calldata verification)

| # | Scenario | Command | Expected Selector |
|---|----------|---------|------------------|
| 6 | Swap USDT→WETH dry-run | `swap --token-in USDT --token-out WETH --amount-in 10000 --chain 42161 --dry-run` | `0xbc651188` (exactInputSingle) |
| 7 | Add-liquidity dry-run | `add-liquidity --token0 USDT --token1 WETH --amount0 10000 --amount1 0 --chain 42161 --dry-run` | `0xa232240b` (mint) |
| 8 | Remove-liquidity dry-run | `remove-liquidity --token-id 99999 --liquidity 1000 --chain 42161 --dry-run` | dry-run response, no RPC |

## L4 — On-chain Tests (real transactions)

| # | Scenario | Command | Fund |
|---|----------|---------|------|
| 9 | Swap 0.01 USDT for WETH | `swap --token-in USDT --token-out WETH --amount-in 10000 --chain 42161` | 0.01 USDT |
