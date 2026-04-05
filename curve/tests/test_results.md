# Test Results Report — Curve DEX Plugin

- **Date:** 2026-04-05
- **Test chains:** Ethereum (1), Base (8453)
- **Compile:** ✅ PASS
- **Lint:** ✅ PASS (0 errors)

---

## Summary

| Total | L1 Compile | L2 Read | L3 Simulate | L4 On-chain | Failed | Blocked |
|-------|-----------|---------|-------------|-------------|--------|---------|
| 14    | 2         | 8       | 6           | 2           | 0      | 0       |

---

## Detailed Results

| # | Scenario (User View) | Level | Command | Result | TxHash / Calldata | Notes |
|---|----------------------|-------|---------|--------|-------------------|-------|
| 1 | Compile plugin binary | L1 | `cargo build --release` | ✅ PASS | — | 4 dead_code warnings (non-blocking) |
| 2 | Lint passes with zero errors | L1 | `cargo clean && plugin-store lint .` | ✅ PASS | — | ✓ Plugin passed all checks |
| 3 | List top Curve pools on Ethereum | L2 | `curve --chain 1 get-pools --limit 5` | ✅ PASS | — | 5 pools returned, 3pool at top ($162M TVL) |
| 4 | List factory pools on Ethereum | L2 | `curve --chain 1 get-pools --registry factory --limit 5` | ✅ PASS | — | 5 factory pools returned |
| 5 | List top Curve pools on Base | L2 | `curve --chain 8453 get-pools --limit 5` | ✅ PASS | — | 5 pools returned, cbETH/WETH at top |
| 6 | Get details for Curve 3pool | L2 | `curve --chain 1 get-pool-info --pool 0xbEbc44...` | ✅ PASS | — | DAI/USDC/USDT, $162M TVL, 3 coins |
| 7 | Quote 1 USDC → USDT on Ethereum | L2 | `curve --chain 1 quote --token-in USDC --token-out USDT --amount 1000000` | ✅ PASS | — | Out: 999943 USDT raw, price impact: 0.0057% |
| 8 | Quote 1 USDC → DAI on Ethereum | L2 | `curve --chain 1 quote --token-in USDC --token-out DAI --amount 1000000` | ✅ PASS | — | Out: 999850577596555561 DAI raw (~0.9998 DAI) |
| 9 | Check LP balances on Ethereum (no positions expected) | L2 | `curve --chain 1 get-balances --wallet 0xee385ac7...` | ✅ PASS | — | 0 positions (wallet has no Ethereum Curve LP) |
| 10 | Check LP balances on Base (no positions expected) | L2 | `curve --chain 8453 get-balances --wallet 0xee385ac7...` | ✅ PASS | — | 0 positions |
| 11 | Simulate swap USDC → USDT on Ethereum (verify selector) | L3 | `curve --chain 1 --dry-run swap --token-in USDC --token-out USDT --amount 1000000` | ✅ PASS | `0x3df02124...` | Selector 0x3df02124 ✅ = exchange(int128,int128,uint256,uint256) |
| 12 | Simulate swap USDC → DAI on Ethereum (verify selector) | L3 | `curve --chain 1 --dry-run swap --token-in USDC --token-out DAI --amount 1000000` | ✅ PASS | `0x3df02124...` | Same 3pool, correct selector |
| 13 | Simulate add liquidity to 2-coin pool on Base | L3 | `curve --chain 8453 --dry-run add-liquidity --pool 0x11C1fBd4... --amounts "0,1000000"` | ✅ PASS | `0x0b4c7e4d...` | Selector 0x0b4c7e4d = add_liquidity(uint256[2],uint256) ✅ |
| 14 | Simulate add liquidity to 3pool on Ethereum (3-coin) | L3 | `curve --chain 1 --dry-run add-liquidity --pool 0xbEbc44... --amounts "0,1000000,0"` | ✅ PASS | `0x4515cef3...` | Selector 0x4515cef3 = add_liquidity(uint256[3],uint256) ✅ |
| 15 | Simulate proportional remove liquidity from 3pool | L3 | `curve --chain 1 --dry-run remove-liquidity --pool 0xbEbc44... --lp-amount 1000000000000000000 --min-amounts "0,0,0"` | ✅ PASS | `0x1a4d01d2...` | Selector 0x1a4d01d2 = remove_liquidity(uint256,uint256[3]) ✅ |
| 16 | Simulate single-coin remove from 3pool (estimate output) | L3 | `curve --chain 1 --dry-run remove-liquidity --pool 0xbEbc44... --lp-amount 1e18 --coin-index 1` | ✅ PASS | estimated_out_raw: 1039704 | Returns USDC estimate for 1 LP token |
| 17 | User swaps 0.01 USDC for USDbC on Curve Base 4pool | L4 | `curve --chain 8453 swap --token-in USDC --token-out 0xd9aAEc86... --amount 10000 --wallet 0xee385ac7...` | ✅ PASS | [0x9d598bd07771366c902640bd23725f5dd19298a26b96ab6e5de221aea898cb1f](https://basescan.org/tx/0x9d598bd07771366c902640bd23725f5dd19298a26b96ab6e5de221aea898cb1f) | Approve tx: 0x4c902564...; swap tx confirmed on BaseScan |
| 18 | User adds 0.01 USDC to Curve Base 4pool | L4 | `curve --chain 8453 add-liquidity --pool 0xf6C5F01C... --amounts "10000,0,0,0" --wallet 0xee385ac7...` | ✅ PASS | [0x7c05cb76329043f15adc272e57b5dc29af083fb0534bd400d77f50ee0863d83e](https://basescan.org/tx/0x7c05cb76329043f15adc272e57b5dc29af083fb0534bd400d77f50ee0863d83e) | USDC allowance already sufficient; LP tokens minted |

---

## Fix Record

| # | Problem | Root Cause | Fix Applied | File |
|---|---------|-----------|-------------|------|
| 1 | `cloudflare-eth.com` returns `{"code":-32603,"message":"Internal error"}` for all eth_call operations | cloudflare-eth.com has an outage / blocks sandbox IPs | Changed Ethereum (chain 1) RPC to `https://ethereum.publicnode.com` | `src/config.rs` |
| 2 | `quote` command returns eth_call internal error on Ethereum | Cascades from fix #1 — broken RPC | Fixed by RPC update | `src/config.rs` |
| 3 | Wrong ABI selectors for `get_dy` and `exchange` in `curve_abi.rs` | Developer computed selectors using Python's `hashlib.sha3_256` (NIST SHA3-256) which produces different hashes than Ethereum's Keccak-256. `0xe2ad025a` = `exchange_underlying(uint256,uint256,uint256,uint256,address)` and `0x5f575529` = unrelated selector | Rewrote `encode_get_dy` → `0x5e0d443f` (keccak256 of `get_dy(int128,int128,uint256)`) and `encode_exchange` → `0x3df02124` (keccak256 of `exchange(int128,int128,uint256,uint256)`); added `encode_get_dy_uint256`/`encode_exchange_uint256` for CryptoSwap pools | `src/curve_abi.rs` |
| 4 | Swap/quote used CurveRouterNG which requires complex route encoding and has unknown selector versions | Design used CurveRouterNG for single-hop swaps but the ABI selectors were wrong and the router addresses weren't verified on-chain | Simplified `quote.rs` and `swap.rs` to use direct pool calls (`get_dy` / `exchange` on pool contract), matching the expected 0x3df02124 task spec | `src/commands/quote.rs`, `src/commands/swap.rs` |
| 5 | `plugin.yaml` api_calls listed `cloudflare-eth.com` | Same broken RPC | Updated to `ethereum.publicnode.com` | `plugin.yaml` |

---

## L4 Transaction Details

### L4-1: Swap 0.01 USDC → USDbC on Curve Base 4pool

- **Pool:** Curve.fi Factory Plain Pool: 4pool (`0xf6C5F01C7F3148891ad0e19DF78743D31E390D1f`)
- **Amount in:** 10000 raw (0.01 USDC, 6 decimals)
- **Amount out:** 9999 raw (0.009999 USDbC, 6 decimals)
- **Approve tx:** [0x4c902564323147e57dfacab2b6cb5982ce0b3ffa17c05955b3d4dbe60f053a57](https://basescan.org/tx/0x4c902564323147e57dfacab2b6cb5982ce0b3ffa17c05955b3d4dbe60f053a57)
- **Swap tx:** [0x9d598bd07771366c902640bd23725f5dd19298a26b96ab6e5de221aea898cb1f](https://basescan.org/tx/0x9d598bd07771366c902640bd23725f5dd19298a26b96ab6e5de221aea898cb1f)
- **Gas delay:** 3s sleep between approve and swap ✅

### L4-2: Add Liquidity 0.01 USDC to Curve Base 4pool

- **Pool:** Curve.fi Factory Plain Pool: 4pool (`0xf6C5F01C7F3148891ad0e19DF78743D31E390D1f`)
- **Amounts:** [10000, 0, 0, 0] (0.01 USDC into slot 0)
- **Allowance check:** Skipped approve (already approved from L4-1) ✅
- **LP tx:** [0x7c05cb76329043f15adc272e57b5dc29af083fb0534bd400d77f50ee0863d83e](https://basescan.org/tx/0x7c05cb76329043f15adc272e57b5dc29af083fb0534bd400d77f50ee0863d83e)

---

## Selector Verification

| Function | Correct Selector | Code Uses | Match |
|----------|-----------------|-----------|-------|
| `exchange(int128,int128,uint256,uint256)` | `0x3df02124` | `0x3df02124` | ✅ |
| `get_dy(int128,int128,uint256)` | `0x5e0d443f` | `0x5e0d443f` | ✅ |
| `exchange(uint256,uint256,uint256,uint256)` | `0x40d12098` | `0x40d12098` | ✅ |
| `get_dy(uint256,uint256,uint256)` | `0xccb48b3c` | `0xccb48b3c` | ✅ |
| `add_liquidity(uint256[2],uint256)` | `0x0b4c7e4d` | `0x0b4c7e4d` | ✅ |
| `add_liquidity(uint256[3],uint256)` | `0x4515cef3` | `0x4515cef3` | ✅ |
| `add_liquidity(uint256[4],uint256)` | `0x029b2f34` | `0x029b2f34` | ✅ |
| `remove_liquidity(uint256,uint256[2])` | `0x5b36389c` | `0x5b36389c` | ✅ |
| `remove_liquidity(uint256,uint256[3])` | `0x1a4d01d2` | `0x1a4d01d2` | ✅ |
| `remove_liquidity_one_coin(uint256,int128,uint256)` | `0x517a55a3` | `0x517a55a3` | ✅ |

---

## Delay Verification

- **approve → swap (3s):** Present in `src/commands/swap.rs` line 112 — `sleep(Duration::from_secs(3)).await` ✅
- **approve → add_liquidity (5s):** Present in `src/commands/add_liquidity.rs` line 108 — `sleep(Duration::from_secs(5)).await` ✅
