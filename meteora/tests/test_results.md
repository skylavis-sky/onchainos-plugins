# Test Results Report

- Date: 2026-04-05
- DApp supported chains: Solana only
- Solana test chain: mainnet (501)
- Compile: ✅
- Lint: ✅
- Overall pass criteria: Solana DApp → Solana all-pass

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|-----------|---------|------------|------------|--------|---------|
| 9     | 2         | 5       | 1          | 1          | 0      | 0       |

## Detailed Results

| # | Scenario (User Perspective) | Level | Command | Result | TxHash / Note | Remarks |
|---|-----------------------------|-------|---------|--------|---------------|---------|
| 1 | Build plugin binary | L1 | `cargo build --release` | ✅ PASS | — | Compiled successfully |
| 2 | Lint plugin code and config | L1 | `cargo clean && plugin-store lint .` | ✅ PASS | — | 0 errors, 0 warnings |
| 3 | Browse top 5 Meteora DLMM pools by TVL | L2 | `meteora get-pools --page-size 5` | ✅ PASS | — | 5 pools returned: SOL-USDC, TRUMP-USDC, etc.; ok=true, total=102366 |
| 4 | Search SOL liquidity pools | L2 | `meteora get-pools --search-term SOL --page-size 3` | ✅ PASS | — | 3 SOL pools returned; top result: SOL-USDC 5rCf...AS6 |
| 5 | View detail for SOL-USDC pool | L2 | `meteora get-pool-detail --address 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6` | ✅ PASS | — | token_x=SOL, token_y=USDC, bin_step=4, TVL=$2.09M |
| 6 | Check my Meteora positions (empty wallet) | L2 | `meteora get-user-positions --wallet 6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY` | ✅ PASS | — | ok=true, positions=[], message="No positions found" (after fix) |
| 7 | Check my Meteora positions (no wallet arg) | L2 | `meteora get-user-positions` | ✅ PASS | — | Resolved wallet via onchainos; returned empty list gracefully |
| 8 | Preview SOL→USDC swap without submitting | L3 | `meteora swap --from-token 111...1 --to-token EPjF... --amount 0.001 --dry-run` | ✅ PASS | dry_run=true | ok=true, dry_run=true, quoted 79953 USDC out; no tx submitted |
| 9 | Execute 0.001 SOL → USDC swap on Meteora | L4 | `meteora swap --from-token 111...1 --to-token EPjF... --amount 0.001` | ✅ PASS | `3F7ZLM6TVd8ZUfXNjqjtUGqCgZbTMj5EajnEryDRfqeop1NBoCqPXGv3sfqhxKA99diF47m6Zwd7uKxRtUchgZN9` | Solscan: https://solscan.io/tx/3F7ZLM6TVd8ZUfXNjqjtUGqCgZbTMj5EajnEryDRfqeop1NBoCqPXGv3sfqhxKA99diF47m6Zwd7uKxRtUchgZN9; 1000000 lamports in → 80079 USDC units out |

## Bug Fixes

| # | Problem | Root Cause | Fix | File |
|---|---------|-----------|-----|------|
| 1 | `get-user-positions` crashes with exit 1 when wallet has no positions | Meteora API returns HTTP 404 for wallets with no positions; code called `anyhow::bail!` on any non-2xx status before the empty-list handling could run | Added explicit 404 → return `Ok(Vec::new())` check before the general error bail | `src/api.rs` |
| 2 | `tx_hash` field shows "pending" and `explorer_url` is empty after successful on-chain swap | `extract_tx_hash()` only checked `data.txHash`; Solana swap execute response uses `data.swapTxHash` (not `txHash`) | Added `data.swapTxHash` as second lookup before falling back to `data.txHash` | `src/onchainos.rs` |
