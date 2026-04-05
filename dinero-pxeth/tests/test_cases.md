# Test Cases — dinero-pxeth

## Level 1 — Compile + Lint

| # | Test | Expected |
|---|------|---------|
| 1 | `cargo build --release` | Compiles without errors |
| 2 | `cargo clean && plugin-store lint .` | 0 errors, 0 warnings |

## Level 2 — Read Tests (no wallet, no gas)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L2-1 | Query apxETH exchange rate and vault TVL | `rates` | JSON with apxeth_per_pxeth ≈ 1.116, total_assets > 0 |
| L2-2 | Query positions for known wallet | `positions --address 0x87fb0647faabea33113eaf1d80d67acb1c491b90` | JSON with pxETH + apxETH balances |

## Level 3 — Dry-run Tests (calldata validation)

| # | Scenario (user view) | Command | Expected calldata |
|---|---------------------|---------|-----------------|
| L3-1 | Simulate deposit 0.00005 ETH to pxETH | `deposit --amount 0.00005 --chain 1 --dry-run` | selector: `adc9740c`, dry_run: true, warning about paused |
| L3-2 | Simulate deposit ETH with compound=true | `deposit --amount 0.00005 --compound --chain 1 --dry-run` | selector: `adc9740c`, compound field in calldata |
| L3-3 | Simulate stake 0.00005 pxETH to apxETH | `stake --amount 0.00005 --chain 1 --dry-run` | approve_calldata selector `095ea7b3`, deposit_calldata selector `6e553f65` |
| L3-4 | Simulate redeem 0.00005 apxETH | `redeem --amount 0.00005 --chain 1 --dry-run` | selector: `ba087652`, dry_run: true |

## Level 4 — On-chain Write Tests (needs lock)

| # | Scenario (user view) | Command | Notes |
|---|---------------------|---------|-------|
| L4-1 | Stake 0.00005 pxETH → apxETH | `stake --amount 0.00005 --chain 1` | SKIP if wallet has no pxETH; need pxETH first |
| L4-SKIP | Deposit ETH → pxETH | `deposit --amount 0.00005 --chain 1` | SKIPPED — PirexEth is paused |
| L4-SKIP | Redeem apxETH → pxETH | `redeem --amount 0.00005 --chain 1` | SKIPPED — only if L4-1 succeeded |
