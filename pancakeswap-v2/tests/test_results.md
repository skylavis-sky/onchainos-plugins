# Test Results Report — PancakeSwap V2

- **Date:** 2026-04-05
- **Tester:** Phase 3 Tester Agent
- **Plugin:** pancakeswap-v2 v0.1.0
- **Test chains:** BSC (56), Base (8453)
- **Compile:** ✅
- **Lint:** ✅

---

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|-----------|---------|------------|------------|--------|---------|
| 17    | 2 ✅       | 8 ✅     | 3 ✅        | 2 ✅ (Base) + 2 ⛔ BLOCKED (BSC) | 0      | 2       |

---

## Detailed Results

| # | Scenario (user view) | Level | Command | Result | TxHash / Calldata | Notes |
|---|---------------------|-------|---------|--------|------------------|-------|
| 1 | Build plugin binary | L1 | `cargo build --release` | ✅ PASS | — | 1 dead_code warning (validate_router, harmless) |
| 2 | Plugin lint check | L1 | `cargo clean && plugin-store lint .` | ✅ PASS | — | 0 errors, all checks passed |
| 3 | "How much USDT for 1 WBNB on PancakeSwap V2?" | L2 | `--chain 56 quote --token-in WBNB --token-out USDT --amount-in 1000000000000000000` | ✅ PASS | — | amountOut=592327660679599984516 (~592 USDT), direct path WBNB→USDT |
| 4 | "Quote WETH→USDC on Base PancakeSwap V2" | L2 | `--chain 8453 quote --token-in WETH --token-out USDC --amount-in 1000000000000000` | ✅ PASS | — | amountOut=2049543 (~2.05 USDC), direct path WETH→USDC |
| 5 | "What is the WBNB/USDT pair address on BSC?" | L2 | `--chain 56 get-pair --token-a WBNB --token-b USDT` | ✅ PASS | — | pair=0x16b9a82891338f9ba80e2d6970fdda79d1eb0dae, exists=true |
| 6 | "What is the WETH/USDC pair address on Base?" | L2 | `--chain 8453 get-pair --token-a WETH --token-b USDC` | ✅ PASS | — | pair=0x79474223aedd0339780bacce75abda0be84dcbf9, exists=true |
| 7 | "What are the reserves in the WBNB/USDT pool on BSC?" | L2 | `--chain 56 get-reserves --token-a WBNB --token-b USDT` | ✅ PASS | — | reserveA=28680 WBNB, reserveB=17.0M USDT, price=593.84 USDT/WBNB |
| 8 | "What are the reserves in the WETH/USDC pool on Base?" | L2 | `--chain 8453 get-reserves --token-a WETH --token-b USDC` | ✅ PASS | — | reserveA=0.174 WETH, reserveB=360 USDC, price=2066 USDC/WETH |
| 9 | "How much LP do I have in WBNB/USDT on BSC?" | L2 | `--chain 56 lp-balance --token-a WBNB --token-b USDT --wallet 0xee385ac7...` | ✅ PASS | — | lpBalance=0 (wallet not LP), totalSupply=272379606984276614941852 |
| 10 | "How much LP do I have in WETH/USDC on Base?" | L2 | `--chain 8453 lp-balance --token-a WETH --token-b USDC --wallet 0xee385ac7...` | ✅ PASS | — | lpBalance=0, totalSupply=5757069332383 |
| 11 | "Simulate swapping WBNB→USDT on PancakeSwap V2" | L3 | `--chain 56 --dry-run swap --token-in WBNB --token-out USDT --amount-in 1000000000000000000` | ✅ PASS | calldata selector: `0x38ed1739` (swapExactTokensForTokens) | dry_run=true, no broadcast, correct selector verified in source |
| 12 | "Preview adding WBNB/USDT liquidity on BSC" | L3 | `--chain 56 --dry-run add-liquidity --token-a WBNB --token-b USDT --amount-a 1000000000000000 --amount-b 500000000000000000` | ✅ PASS | calldata selector: `0xe8e33700` (addLiquidity) | dry_run=true, steps include approve_tokenA, approve_tokenB, addLiquidity |
| 13 | "Preview removing WBNB/USDT liquidity on BSC" | L3 | `--chain 56 --dry-run remove-liquidity --token-a WBNB --token-b USDT --liquidity 1000000000000000` | ✅ PASS | calldata selector: `0xbaa2abde` (removeLiquidity) | dry_run=true, steps include approve_lp, removeLiquidity |
| 14 | "Swap WBNB→USDT with minimum amount on BSC" | L4 | `--chain 56 swap --token-in WBNB --token-out USDT --amount-in <min>` | ⛔ BLOCKED | — | No BSC funds in test wallet (0 BNB, 0 WBNB, 0 USDT on chain 56) |
| 15 | "Swap USDT→WBNB with minimum amount on BSC" | L4 | `--chain 56 swap --token-in USDT --token-out WBNB --amount-in 10000000000000000000` | ⛔ BLOCKED | — | No BSC funds in test wallet |
| 16 | "Swap 0.01 USDC → WETH on Base PancakeSwap V2" | L4 | `--chain 8453 --from 0xee385... swap --token-in USDC --token-out WETH --amount-in 10000` | ✅ PASS | Approve: `0x3eec60959c82f1be38d1ea9ca1c9313e0e6fb6ee6ab563db1c9cb072d2cb9b04` Swap: [`0xaf843e802027e652095a1f84a76e849d447a0de747ffbc6f135fa2ab7ea7a5db`](https://basescan.org/tx/0xaf843e802027e652095a1f84a76e849d447a0de747ffbc6f135fa2ab7ea7a5db) | amountIn=10000 (0.01 USDC), amountOutExpected=4827037013997 (~0.00000000483 WETH), path: USDC→WETH direct, slippage 0.5%, chain 8453 |
| 17 | "LP balance WETH/USDC on Base post-swap" | L4 | `--chain 8453 lp-balance --token-a WETH --token-b USDC --wallet 0xee385...` | ✅ PASS | — (read-only) | lpBalance=0, totalSupply=5757069332383, pair=0x79474223aedd0339780bacce75abda0be84dcbf9, pool functional |

---

## Calldata Selector Verification (L3)

| Operation | Expected Selector | Source Location | Status |
|-----------|------------------|-----------------|--------|
| `swapExactTokensForTokens` | `0x38ed1739` | `src/commands/swap.rs:220` | ✅ Confirmed |
| `addLiquidity` | `0xe8e33700` | `src/commands/add_liquidity.rs:161` | ✅ Confirmed |
| `removeLiquidity` | `0xbaa2abde` | `src/commands/remove_liquidity.rs:173` | ✅ Confirmed |

---

## L4 On-Chain Results (Base, chain 8453)

**Date:** 2026-04-05
**Wallet:** `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`
**Pre-swap balance:** 1.245393 USDC, ~0.00425 ETH on Base

### Test 16 — Swap 0.01 USDC → WETH (PASS)

- Command: `./target/release/pancakeswap-v2 --chain 8453 --from 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9 swap --token-in USDC --token-out WETH --amount-in 10000`
- Step 1 (approve ERC-20): txHash `0x3eec60959c82f1be38d1ea9ca1c9313e0e6fb6ee6ab563db1c9cb072d2cb9b04`
- Step 2 (swapExactTokensForTokens): txHash `0xaf843e802027e652095a1f84a76e849d447a0de747ffbc6f135fa2ab7ea7a5db`
- BaseScan: https://basescan.org/tx/0xaf843e802027e652095a1f84a76e849d447a0de747ffbc6f135fa2ab7ea7a5db
- amountOutExpected: 4827037013997 (~0.00000000483 WETH), slippage 0.5%

### Test 17 — LP Balance WETH/USDC post-swap (PASS)

- Command: `./target/release/pancakeswap-v2 --chain 8453 lp-balance --token-a WETH --token-b USDC --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`
- Result: lpBalance=0, totalSupply=5757069332383, pool at pair `0x79474223aedd0339780bacce75abda0be84dcbf9` functional

---

## Blocked: L4 On-Chain Tests (BSC, chain 56)

**Root Cause:** Test wallet `0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` has zero balance on BSC (chain 56):
- Native BNB: 0
- WBNB (ERC-20): 0
- USDT (ERC-20): 0

**Resolution Required:** Fund the test wallet on BSC with at least ~0.001 BNB (for gas) and either:
- 0.001 WBNB (to swap WBNB→USDT), or
- 0.01 USDT (to swap USDT→WBNB)

---

## Fix Record

| # | Issue | Root Cause | Fix | File |
|---|-------|-----------|-----|------|
| — | No bugs found | — | — | — |

---

## Notes

- Dead code warning for `validate_router` in `rpc.rs` — harmless, function exists for optional router validation. Not a bug.
- L3 tests confirm the correct PancakeSwap V2 function selectors are used in all write operations.
- BSC RPC endpoint (`bsc-rpc.publicnode.com`) is correctly configured per KNOWLEDGE_HUB guidance.
- `--force` flag is correctly applied in all `wallet_contract_call` invocations (verified in `onchainos.rs`).
- Dry-run is handled in wrapper layer (not passed to onchainos CLI) per known behavior.
