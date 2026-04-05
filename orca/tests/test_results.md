# Test Results Report

- Date: 2026-04-05
- DApp: Orca Whirlpools DEX
- Supported chains: Solana only
- Solana test chain: mainnet (501)
- Wallet: `6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY`
- Compilation: ✅
- Lint: ✅
- Overall pass standard: Solana DApp → Solana all pass ✅

## Summary

| Total | L1 Build | L2 Read | L3 Simulate | L4 On-chain | Fail | Blocked |
|-------|----------|---------|-------------|-------------|------|---------|
| 7     | 2        | 2       | 1           | 2           | 0    | 0       |

## Detailed Results

| # | Scenario (user perspective)                                      | Level | Command                                                   | Result    | TxHash / Note                                                                                    | Notes                                            |
|---|------------------------------------------------------------------|-------|-----------------------------------------------------------|-----------|--------------------------------------------------------------------------------------------------|--------------------------------------------------|
| 1 | Build Orca plugin binary                                         | L1    | `cargo build --release`                                   | ✅ PASS   | —                                                                                                | 10 dead-code warnings (acceptable)               |
| 2 | Lint Orca plugin source                                          | L1    | `cargo clean && plugin-store lint .`                      | ✅ PASS   | —                                                                                                | 0 errors, 0 warnings                             |
| 3 | List all SOL/USDC pools on Orca                                  | L2    | `orca get-pools --token-a 111...1 --token-b EPjF...`      | ✅ PASS   | —                                                                                                | 9 pools returned, top TVL $32.5M, sorted by TVL  |
| 4 | Get quote for 0.001 SOL → USDC                                   | L2    | `orca get-quote --from-token 111...1 --amount 0.001`      | ✅ PASS   | —                                                                                                | estimated_out: 0.1275 USDC, slippage_bps: 50     |
| 5 | Simulate 0.001 SOL → USDC swap (no real tx)                     | L3    | `orca --dry-run swap --from-token 111...1 --amount 0.001` | ✅ PASS   | dry_run: true                                                                                    | Returns ok:true, dry_run:true, no network call   |
| 6 | Dry-run result format re-verified after bug fix                   | L3    | `orca --dry-run swap --from-token 111...1 --amount 0.001` | ✅ PASS   | dry_run: true                                                                                    | Confirmed format intact post-fix                 |
| 7 | Swap 0.001 SOL → USDC on Solana mainnet                         | L4    | `orca swap --from-token 111...1 --amount 0.001`           | ✅ PASS   | `3fSGeq2EgWNXk22KUtqweN4enGfVVNN7RpAkhcd9GgdpmCAb237bD2URaXcdRFYp6RubderGjsVgQkDaAAas1zVp` | https://solscan.io/tx/3fSGeq2EgWNXk22KUtqweN4enGfVVNN7RpAkhcd9GgdpmCAb237bD2URaXcdRFYp6RubderGjsVgQkDaAAas1zVp — received 0.080024 USDC |

## Fix Log

| # | Problem                                                    | Root Cause                                                                                                       | Fix                                                                                                       | File                                     |
|---|------------------------------------------------------------|------------------------------------------------------------------------------------------------------------------|-----------------------------------------------------------------------------------------------------------|------------------------------------------|
| 1 | `onchainos dex swap execute` — unrecognized subcommand     | Plugin used wrong CLI subcommand path: `onchainos dex swap execute`. Correct path is `onchainos swap execute`.  | Changed args from `["dex", "swap", "execute", ...]` to `["swap", "execute", ...]`; also added `--wallet` arg and switched `--from-token`/`--to-token` to `--from`/`--to` per actual CLI spec | `src/commands/swap.rs`                   |
| 2 | `tx_hash: "pending"` despite successful on-chain broadcast | `extract_tx_hash` looked for `data.txHash` but `onchainos swap execute` returns `data.swapTxHash`               | Updated `extract_tx_hash` to check `data.swapTxHash` first, then fall back to `data.txHash` / root `txHash` | `src/onchainos.rs`                       |
| 3 | `onchainos wallet balance --output json` not supported     | `resolve_wallet_solana` passed `--output json` flag which onchainos does not support for balance command         | Removed `--output json` from wallet balance call; already fixed in source before this test run            | `src/onchainos.rs`                       |

## Balance After L4

| Token | Before      | After        | Change         |
|-------|-------------|--------------|----------------|
| SOL   | 0.01000000  | 0.00687289   | -0.00312711    |
| USDC  | 0.000000    | 0.080024     | +0.080024      |

SOL remaining: 0.00687289 — above 0.003 SOL hard reserve ✅
