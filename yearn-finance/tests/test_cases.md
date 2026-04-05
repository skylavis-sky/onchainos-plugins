# Test Cases — Yearn Finance Plugin

## Level 1: Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|---------|
| L1-1 | cargo build | `cargo build --release` | Compiles with 0 errors |
| L1-2 | plugin-store lint | `cargo clean && plugin-store lint .` | 0 errors |

## Level 2: Read Tests (no wallet, no gas)

| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|---------|
| L2-1 | List all Yearn vaults on Ethereum | `vaults --chain 1` | JSON with count > 100, each vault has address/name/apr |
| L2-2 | Filter USDT vaults | `vaults --chain 1 --token USDT` | Count ≥ 2, all have USDT token |
| L2-3 | Check APR rates for all vaults | `rates --chain 1` | Sorted by APR, each has net_apr/history |
| L2-4 | Check APR for USDT vaults | `rates --chain 1 --token USDT` | USDT vaults only, APR data present |
| L2-5 | Query my positions (no balance) | `positions --chain 1 --wallet 0x87fb...` | Returns positions array (empty OK) |

## Level 3: Dry-run / Simulate Tests

| # | Scenario (user view) | Command | Expected Calldata |
|---|---------------------|---------|-------------------|
| L3-1 | Simulate deposit 0.01 USDT | `--dry-run deposit --vault USDT --amount 0.01` | Steps: approve `0x095ea7b3`, deposit `0x6e553f65` |
| L3-2 | Simulate withdraw from vault | `--dry-run withdraw --vault 0x6D2981...` | Calldata starts `0xba087652` |
| L3-3 | Simulate deposit with vault address | `--dry-run deposit --vault 0x6D2981... --amount 0.01` | approve + deposit steps |

## Level 4: On-chain Tests (requires lock)

| # | Scenario (user view) | Command | Required Fund |
|---|---------------------|---------|--------------|
| L4-1 | Deposit 0.01 USDT into Gauntlet USDT Prime vault | `deposit --vault 0x6D2981... --amount 0.01` | 0.01 USDT + gas |
| L4-2 | Withdraw all shares from Gauntlet USDT Prime vault | `withdraw --vault 0x6D2981...` | gas only |

## Error Cases

| # | Scenario | Command | Expected Error |
|---|---------|---------|----------------|
| E1 | Vault not found | `deposit --vault NONEXISTENT --amount 1` | "Vault not found" |
| E2 | Invalid amount | `deposit --vault USDT --amount abc` | "Invalid amount" |
| E3 | Withdraw with no shares | `withdraw --vault USDT` (no position) | "No shares held in vault" |
