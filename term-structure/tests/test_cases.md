# Term Structure Plugin — Test Cases

## L1: Help / Smoke Tests (no network required)

```bash
./target/release/term-structure --help
./target/release/term-structure get-markets --help
./target/release/term-structure lend --help
./target/release/term-structure borrow --help
./target/release/term-structure repay --help
./target/release/term-structure redeem --help
```

Expected: usage text printed, exit code 0.

## L2: Read-only Tests (requires Arbitrum RPC)

### 2.1 Get Markets
```bash
./target/release/term-structure get-markets --chain 42161
```
Expected:
- `ok: true`
- `markets` array with at least 1 entry
- Each entry has: market, collateral, underlying, maturity_date, lend_apr_pct, borrow_apr_pct, ft_liquidity

### 2.2 Get Markets with Underlying Filter
```bash
./target/release/term-structure get-markets --chain 42161 --underlying USDC
```
Expected: only USDC markets shown.

### 2.3 Get Position (no wallet needed — empty position)
```bash
./target/release/term-structure get-position --chain 42161 --from 0x0000000000000000000000000000000000000001
```
Expected: `ok: true`, `positions: []` (no positions for zero address).

## L3: Dry-run Tests (no real tx broadcast)

### 3.1 Lend Dry-run
```bash
./target/release/term-structure lend \
  --chain 42161 \
  --market 0x1aD9e38F7E2B8d8B64Fc0D6AA29C2ced9fE1E0dC \
  --amount 1000 \
  --token USDC \
  --from 0x0000000000000000000000000000000000000001 \
  --dry-run
```
Expected:
- `ok: true`, `dryRun: true`
- `steps` array with 2 entries (approve + swapExactTokenToToken)
- Each step has `simulatedCommand` starting with `onchainos wallet contract-call`
- `amount_raw` = "1000000000" (1000 USDC with 6 decimals)

### 3.2 Borrow Dry-run
```bash
./target/release/term-structure borrow \
  --chain 42161 \
  --market 0x1aD9e38F7E2B8d8B64Fc0D6AA29C2ced9fE1E0dC \
  --collateral-amount 1.0 \
  --collateral-token WETH \
  --borrow-amount 500 \
  --from 0x0000000000000000000000000000000000000001 \
  --dry-run
```
Expected:
- `ok: true`, `dryRun: true`
- `steps` array with 2 entries (approve collateral + borrowTokenFromCollateral)
- `collateral_amount_raw` = "1000000000000000000" (1 WETH with 18 decimals)
- `borrow_amount_raw` = "500000000" (500 USDC with 6 decimals)

### 3.3 Repay Dry-run
```bash
./target/release/term-structure repay \
  --chain 42161 \
  --market 0x1aD9e38F7E2B8d8B64Fc0D6AA29C2ced9fE1E0dC \
  --loan-id 1 \
  --max-amount 510 \
  --token USDC \
  --from 0x0000000000000000000000000000000000000001 \
  --dry-run
```
Expected:
- `ok: true`, `dryRun: true`
- `steps` with approve + repayByTokenThroughFt calldata

### 3.4 Redeem Dry-run
```bash
./target/release/term-structure redeem \
  --chain 42161 \
  --market 0x1aD9e38F7E2B8d8B64Fc0D6AA29C2ced9fE1E0dC \
  --all \
  --from 0x0000000000000000000000000000000000000001 \
  --dry-run
```
Expected:
- `ok: true`, `dryRun: true`
- `steps` with redeem calldata starting `0x7bde82f2`

## L4: Live Transaction Tests (requires funded wallet on Arbitrum)

### Prerequisites
- Wallet must have USDC on Arbitrum (chain 42161)
- Wallet must have ETH for gas

### 4.1 Lend USDC (live)
```bash
./target/release/term-structure lend \
  --chain 42161 \
  --market 0x1aD9e38F7E2B8d8B64Fc0D6AA29C2ced9fE1E0dC \
  --amount 10 \
  --token USDC
```
Expected: `approve_tx_hash` and `lend_tx_hash` returned (0x... format).

### 4.2 Get Position (verify FT received)
```bash
./target/release/term-structure get-position --chain 42161
```
Expected: position entry showing FT balance > 0 for the USDC/WETH market.

### 4.3 Redeem after maturity (requires market to be matured)
Only executable after `maturity_ts` has passed.
```bash
./target/release/term-structure redeem \
  --chain 42161 \
  --market 0x1aD9e38F7E2B8d8B64Fc0D6AA29C2ced9fE1E0dC \
  --all
```

## Known Limitations

1. Market addresses are hardcoded from TermMax V2 deployment files. New markets require config.rs update.
2. No factory `getMarkets()` enumerator on-chain; curated list used.
3. Borrow positions: GT NFT loanId enumeration via viewer may have limited data in simplified ABI decode.
4. Early FT exit (sell before maturity) not implemented in this version — use TermMax frontend for early exit via `sellTokens`.
5. Thin liquidity: check `ft_liquidity` in get-markets before large orders.
