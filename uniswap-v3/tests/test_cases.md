# Test Cases — uniswap-v3 v0.1.0

Generated: 2026-04-19  
Tester: Phase 3 Tester Agent  
Target chain for L2/L3/L4: Base (chain 8453)

---

## Flag Reference (confirmed from `--help`)

| SKILL.md flag | Actual binary flag | Match? |
|--------------|-------------------|--------|
| `--amount` | `--amount` | YES |
| `--token-in` | `--token-in` | YES |
| `--token-out` | `--token-out` | YES |
| `--token-a` | `--token-a` | YES |
| `--token-b` | `--token-b` | YES |
| `--amount-a` | `--amount-a` | YES |
| `--amount-b` | `--amount-b` | YES |
| `--slippage-bps` | `--slippage-bps` | YES |
| `--dry-run` | `--dry-run` | YES |
| `--chain` | `--chain` | YES |
| `--fee` | `--fee` | YES |

> Note: The Phase 3 test instructions reference `--amount-in` for get-quote/swap, but actual binary flag is `--amount`. Correct flag used in all tests below.

---

## L1 — Build + Lint

| ID | Test | Command | Expected |
|----|------|---------|----------|
| L1-01 | Release build | `cargo build --release` | Exit 0, binary at target/release/uniswap-v3 |
| L1-02 | Lint | `plugin-store lint .` | "passed all checks" |

---

## L2 — Read Tests (Base, chain 8453)

All L2 tests are read-only `eth_call` operations. No wallet required.

| ID | Command | Test Scenario | Expected Fields |
|----|---------|--------------|----------------|
| L2-01 | `get-quote --token-in WETH --token-out USDC --amount 0.0001 --chain 8453` | Quote 0.0001 WETH → USDC on Base | JSON with `amount_out`, `fee_tier`, `amount_in_human`, non-zero amount_out |
| L2-02 | `get-quote --token-in WETH --token-out USDC --amount 0.0001 --chain 8453 --fee 3000` | Quote with explicit fee tier override | JSON with `fee_tier: 3000`, non-zero amount_out |
| L2-03 | `get-pools --token-a WETH --token-b USDC --chain 8453` | List all WETH/USDC pools on Base | JSON with at least one pool entry with `fee_tier`, `pool_address` (non-zero), `exists: true` |
| L2-04 | `get-pools --token-a WETH --token-b USDC --chain 1` | List pools on Ethereum | JSON pools for chain 1, different contract addresses than Base |
| L2-05 | `get-positions --chain 8453` | Get positions for connected wallet on Base | JSON output (may be empty array `[]` — that's acceptable) |
| L2-06 | `get-positions --token-id 1 --chain 8453` | Query specific position token ID 1 on Base | JSON with position fields or graceful error if no such position |

---

## L3 — Dry-Run Tests (Base, chain 8453)

All L3 tests use `--dry-run` and should NOT submit any on-chain transactions.

| ID | Command | Expected Calldata Prefix | Other Checks |
|----|---------|--------------------------|-------------|
| L3-01 | `swap --token-in WETH --token-out USDC --amount 0.0001 --slippage-bps 50 --chain 8453 --dry-run` | `0x04e45aaf` (exactInputSingle SwapRouter02) | `dry_run: true` in output, `to` = SwapRouter02 `0x2626664c2603336E57B271c5C0b26F421741e481` |
| L3-02 | `swap --token-in WETH --token-out USDC --amount 0.0001 --chain 8453 --fee 3000 --dry-run` | `0x04e45aaf` | fee=3000 reflected in calldata (bytes 68-75 of calldata = `0x0000...0BB8`) |
| L3-03 | `add-liquidity --token-a WETH --token-b USDC --fee 3000 --amount-a 0.0001 --amount-b 0.01 --tick-lower -887220 --tick-upper 887220 --chain 8453 --dry-run` | First output: `0x095ea7b3` (approve) then `0x88316456` (mint) | `dry_run: true`, NFPM address = `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` |

---

## L4 — On-Chain Tests (Base, chain 8453)

**These will NOT be run until checkpoint 3 approval.**

Minimum gas tests (small amounts, Base chain — very low fees).

| ID | Command | Chain | Amount | Estimated Gas | Notes |
|----|---------|-------|--------|--------------|-------|
| L4-01 | `get-quote --token-in WETH --token-out USDC --amount 0.0001 --chain 8453` | Base 8453 | read-only (L2, no gas) | $0.00 | Warm-up, confirms RPC is live |
| L4-02 | `swap --token-in WETH --token-out USDC --amount 0.00005 --slippage-bps 50 --chain 8453` | Base 8453 | 0.00005 WETH (~$0.13) | ~$0.01-0.05 gas on Base | Minimum viable swap |

**Gas estimation for L4-02:**
- Base gas fees are typically 0.001-0.01 gwei
- Uniswap V3 swap: ~130,000 gas units
- Estimated total: ~0.0001-0.001 ETH (~$0.25-$2.50) including possible approve
- Wallet ETH balance requirement: minimum 0.001 ETH on Base

**Pre-L4 checklist:**
1. Check wallet ETH balance on Base: `onchainos wallet balance --chain 8453 --output json`
2. Verify WETH balance >= 0.00005 WETH (or ETH can be wrapped)
3. Confirm RPC endpoint `https://base-rpc.publicnode.com` is reachable

---

## Acceptance Criteria

| Level | Criteria |
|-------|---------|
| L1 | Build: exit 0, binary exists. Lint: "passed all checks". |
| L2 | Each test returns valid JSON. Key numeric fields (amount_out, pool_address) are non-zero. get-positions may return empty array. |
| L3 | Calldata starts with expected selector (0x04e45aaf for swap, 0x88316456 for mint). `dry_run: true` present. No on-chain call made. |
| L4 | txHash returned (non-empty, 0x-prefixed). Transaction appears on BaseScan. |
