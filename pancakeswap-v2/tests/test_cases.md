# PancakeSwap V2 — Test Cases

**Plugin:** pancakeswap-v2  
**Binary:** `./target/release/pancakeswap-v2`  
**Chains:** BSC (56), Base (8453)  
**Generated:** 2026-04-05

---

## L1 — Compile + Lint

| # | Test | Command | Expected |
|---|------|---------|---------|
| L1-1 | Release build | `cargo build --release` | Exit 0, no errors |
| L1-2 | Plugin lint | `cargo clean && plugin-store lint .` | 0 errors, "passed all checks" |

---

## L2 — Read Tests (no wallet, pure RPC)

### quote — Get Expected Swap Output

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| L2-1 | "How much USDT for 1 WBNB on PancakeSwap V2?" | `--chain 56 quote --token-in WBNB --token-out USDT --amount-in 1000000000000000000` | `ok:true`, `amountOut > 0`, `path` has 2 elements, `symbolIn=WBNB` |
| L2-2 | "Quote WETH→USDC on Base PancakeSwap V2" | `--chain 8453 quote --token-in WETH --token-out USDC --amount-in 1000000000000000` | `ok:true`, `amountOut > 0`, Base pair path returned |

### get-pair — Look Up Pair Address

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| L2-3 | "What is the WBNB/USDT pair on BSC PancakeSwap V2?" | `--chain 56 get-pair --token-a WBNB --token-b USDT` | `ok:true`, `exists:true`, non-zero pair address |
| L2-4 | "What is the WETH/USDC pair on Base PancakeSwap V2?" | `--chain 8453 get-pair --token-a WETH --token-b USDC` | `ok:true`, `exists:true`, non-zero pair address |

### get-reserves — Get Pool Reserves

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| L2-5 | "What are the WBNB/USDT pool reserves on BSC?" | `--chain 56 get-reserves --token-a WBNB --token-b USDT` | `ok:true`, `reserveA > 0`, `reserveB > 0`, `priceBPerA > 0` |
| L2-6 | "What are the WETH/USDC pool reserves on Base?" | `--chain 8453 get-reserves --token-a WETH --token-b USDC` | `ok:true`, `reserveA > 0`, `reserveB > 0` |

### lp-balance — Check LP Token Balance

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| L2-7 | "How much LP do I have in WBNB/USDT on BSC?" | `--chain 56 lp-balance --token-a WBNB --token-b USDT --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok:true`, valid pair address, `totalSupply > 0` |
| L2-8 | "How much LP do I have in WETH/USDC on Base?" | `--chain 8453 lp-balance --token-a WETH --token-b USDC --wallet 0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9` | `ok:true`, valid pair address, `totalSupply > 0` |

---

## L3 — Dry-Run Simulate (calldata selector verification)

| # | Scenario | Command | Expected Selector |
|---|---------|---------|---------|
| L3-1 | "Simulate swapping WBNB→USDT on PancakeSwap V2 BSC" | `--chain 56 --dry-run swap --token-in WBNB --token-out USDT --amount-in 1000000000000000000` | `steps` includes `swapExactTokensForTokens`, selector `0x38ed1739` in calldata |
| L3-2 | "Preview adding WBNB/USDT liquidity" | `--chain 56 --dry-run add-liquidity --token-a WBNB --token-b USDT --amount-a 1000000000000000 --amount-b 500000000000000000` | `steps` includes `addLiquidity`, selector `0xe8e33700` in calldata |
| L3-3 | "Preview removing WBNB/USDT liquidity" | `--chain 56 --dry-run remove-liquidity --token-a WBNB --token-b USDT --liquidity 1000000000000000` | `steps` includes `removeLiquidity`, selector `0xbaa2abde` in calldata |

---

## L4 — On-Chain Tests (BSC chain 56, requires lock + BSC funds)

> Core write operations. Requires wallet funded on BSC (chain 56).

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| L4-1 | "Swap WBNB→USDT with minimum amount on BSC PancakeSwap V2" | `--chain 56 swap --token-in WBNB --token-out USDT --amount-in <min_wbnb>` | `ok:true`, valid txHash, BscScan verified |
| L4-2 | "Swap USDT→WBNB with minimum amount on BSC PancakeSwap V2" | `--chain 56 swap --token-in USDT --token-out WBNB --amount-in 10000000000000000000` | `ok:true`, valid txHash, BscScan verified |

---

## Error / Edge Cases

| # | Scenario | Command | Expected |
|---|---------|---------|---------|
| E1 | Unsupported chain | `--chain 1 quote --token-in WETH --token-out USDC --amount-in 1000` | `ok:false`, error mentions supported chains |
| E2 | Non-existent pair | `--chain 56 get-pair --token-a CAKE --token-b 0x0000000000000000000000000000000000000001` | `ok:true`, `exists:false` |
