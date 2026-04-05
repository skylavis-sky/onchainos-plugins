# Curve DEX Plugin — Test Cases

**Plugin:** curve  
**Binary:** `./target/release/curve`  
**Test chain:** Ethereum (1), Base (8453)  
**Generated:** 2026-04-05

---

## L1 — Compile + Lint

| # | Description | Command | Expected |
|---|-------------|---------|----------|
| L1-1 | Release build compiles without errors | `cargo build --release` | Exit 0, binary produced |
| L1-2 | Lint passes with no errors | `cargo clean && plugin-store lint .` | ✓ Plugin passed all checks |

---

## L2 — Read Tests (no wallet, no gas)

### get-pools
| # | Scenario (user view) | Command | Expected |
|---|---------------------|---------|----------|
| L2-1 | List top Curve pools on Ethereum | `curve --chain 1 get-pools --limit 5` | JSON with `ok:true`, `pools` array non-empty |
| L2-2 | List factory pools on Ethereum | `curve --chain 1 get-pools --registry factory --limit 5` | JSON with `ok:true`, pools list |
| L2-3 | List Curve pools on Base | `curve --chain 8453 get-pools --limit 5` | JSON with `ok:true` |

### get-pool-info
| # | Scenario | Command | Expected |
|---|----------|---------|----------|
| L2-4 | Get info for Curve 3pool on Ethereum | `curve --chain 1 get-pool-info --pool 0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7` | JSON with pool name, coins, tvl |

### quote
| # | Scenario | Command | Expected |
|---|----------|---------|----------|
| L2-5 | Quote 1 USDC → USDT on Ethereum | `curve --chain 1 quote --token-in USDC --token-out USDT --amount 1000000` | JSON with `expected_out_raw` > 0 |
| L2-6 | Quote 1 USDC → DAI on Ethereum | `curve --chain 1 quote --token-in USDC --token-out DAI --amount 1000000` | JSON with `expected_out_raw` > 0 |

### get-balances
| # | Scenario | Command | Expected |
|---|----------|---------|----------|
| L2-7 | Check LP balances for test wallet on Ethereum | `curve --chain 1 get-balances --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | JSON with `ok:true`, balances list (may be empty) |
| L2-8 | Check LP balances on Base | `curve --chain 8453 get-balances --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | JSON with `ok:true` |

---

## L3 — Dry-Run / Simulation Tests (calldata verification)

| # | Scenario | Command | Expected Selector |
|---|----------|---------|------------------|
| L3-1 | Dry-run swap USDC→USDT on Ethereum | `curve --chain 1 --dry-run swap --token-in USDC --token-out USDT --amount 1000000` | `calldata` starts with `0x5f575529` (CurveRouterNG exchange) |
| L3-2 | Dry-run swap USDC→DAI on Ethereum | `curve --chain 1 --dry-run swap --token-in USDC --token-out DAI --amount 1000000` | `calldata` starts with `0x5f575529` |
| L3-3 | Dry-run add liquidity to 3pool (2-coin version for Base pool) | `curve --chain 8453 --dry-run add-liquidity --pool <base_pool> --amounts "0,1000000"` | `calldata` starts with `0x0b4c7e4d` (add_liquidity 2-coin) |
| L3-4 | Dry-run add liquidity to 3pool on Ethereum | `curve --chain 1 --dry-run add-liquidity --pool 0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7 --amounts "0,1000000,0"` | `calldata` starts with `0x4515cef3` (add_liquidity 3-coin) |
| L3-5 | Dry-run remove liquidity proportional from 3pool | `curve --chain 1 --dry-run remove-liquidity --pool 0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7 --lp-amount 1000000000000000000 --min-amounts "0,0,0"` | `calldata` starts with `0x1a4d01d2` (remove_liquidity 3-coin) |
| L3-6 | Dry-run remove liquidity single-coin from 3pool | `curve --chain 1 --dry-run remove-liquidity --pool 0xbEbc44782C7dB0a1A60Cb6fe97d0b483032FF1C7 --lp-amount 1000000000000000000 --coin-index 1` | Response with `coin_index:1`, `estimated_out_raw` |

---

## L4 — On-Chain Tests (need lock, real gas)

Priority: swap (USDC → USDT on stableswap) as core L4 test.

| # | Scenario | Command | Chain | Amount | Expected |
|---|----------|---------|-------|--------|----------|
| L4-1 | User swaps 0.01 USDC for USDT on Curve (Ethereum stableswap) | `curve --chain 1 swap --token-in USDC --token-out USDT --amount 10000` | Ethereum | 0.01 USDC (10000 raw) | `tx_hash` non-zero, basescan/etherscan verifiable |
| L4-2 | User swaps 0.01 USDC for DAI on Curve Ethereum | `curve --chain 1 swap --token-in USDC --token-out DAI --amount 10000` | Ethereum | 0.01 USDC | `tx_hash` non-zero |

Note: L4 tests use chain 1 (Ethereum) because stablecoin pools (3pool with USDC/USDT/DAI) are best on Ethereum mainnet. Base has limited Curve pool liquidity.

---

## Selector Reference

| Function | Selector |
|----------|---------|
| CurveRouterNG `exchange(address[11],uint256[5][5],uint256,uint256,address[5],address)` | `0x5f575529` |
| Pool direct `exchange(int128,int128,uint256,uint256)` | `0x3df02124` |
| `add_liquidity(uint256[2],uint256)` | `0x0b4c7e4d` |
| `add_liquidity(uint256[3],uint256)` | `0x4515cef3` |
| `add_liquidity(uint256[4],uint256)` | `0x029b2f34` |
| `remove_liquidity(uint256,uint256[2])` | `0x5b36389c` |
| `remove_liquidity(uint256,uint256[3])` | `0x1a4d01d2` |
| `remove_liquidity_one_coin(uint256,int128,uint256)` | `0x517a55a3` |
| ERC-20 `approve(address,uint256)` | `0x095ea7b3` |
