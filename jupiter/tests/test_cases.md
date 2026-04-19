# Jupiter Plugin — Test Cases (L2 / L3 / L4)

## L2 — Read Tests (no wallet, no gas)

### TC-L2-01: get-quote SOL→USDC
```bash
./target/release/jupiter get-quote --input-mint SOL --output-mint USDC --amount 0.1
```
**Expected:**
- Exit code 0
- JSON with fields: `input`, `output`, `price_impact`, `route`, `slippage_bps`, `raw`
- `raw.inAmount` = 100000000 (0.1 SOL in lamports)
- `raw.outAmount` > 0
- `route` is a non-empty array of DEX labels
- `price_impact` ends with `%`

### TC-L2-02: get-price SOL
```bash
./target/release/jupiter get-price --token SOL
```
**Expected:**
- Exit code 0
- JSON with fields: `token`, `mint`, `price`, `vs`, `vs_mint`, `price_change_24h`
- `token` = "SOL"
- `mint` = `So11111111111111111111111111111111111111112`
- `price` is a positive numeric string (e.g. "150.123456")
- `vs` = "USDC"

### TC-L2-03: get-tokens --limit 5
```bash
./target/release/jupiter get-tokens --limit 5
```
**Expected:**
- Exit code 0
- JSON with `count` <= 5
- `tokens` array with each entry having: `symbol`, `name`, `mint`, `decimals`, `verified`

### TC-L2-04: get-quote with slippage override
```bash
./target/release/jupiter get-quote --input-mint SOL --output-mint USDT --amount 0.05 --slippage-bps 100
```
**Expected:**
- Exit code 0
- `slippage_bps` = 100 in output
- Valid output/route

### TC-L2-05: get-tokens search
```bash
./target/release/jupiter get-tokens --search JUP --limit 3
```
**Expected:**
- Exit code 0
- `count` <= 3
- Results include JUP token with correct mint `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN`

### TC-L2-06: get-price JUP
```bash
./target/release/jupiter get-price --token JUP
```
**Expected:**
- Exit code 0
- `token` = "JUP"
- `mint` = `JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN`
- `price` is a valid positive number

### TC-L2-07: get-quote with raw mint addresses
```bash
./target/release/jupiter get-quote \
  --input-mint So11111111111111111111111111111111111111112 \
  --output-mint EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.1
```
**Expected:**
- Same as TC-L2-01 but using explicit mint addresses
- `raw.inputMint` = `So11111111111111111111111111111111111111112`

---

## L3 — Dry-Run Tests (swap --dry-run, no gas)

### TC-L3-01: swap --dry-run SOL→USDC
```bash
./target/release/jupiter swap --input-mint SOL --output-mint USDC --amount 0.01 --dry-run
```
**Expected:**
- Exit code 0
- JSON with: `ok: true`, `dry_run: true`
- Fields: `inputMint`, `outputMint`, `amount`, `slippageBps`, `note`
- `inputMint` = `So11111111111111111111111111111111111111112`
- `outputMint` = `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
- NO onchainos wallet resolution (dry_run exits before that)
- NO `serialized_tx` field (dry_run returns early without building tx)

### TC-L3-02: swap --dry-run with slippage
```bash
./target/release/jupiter swap --input-mint SOL --output-mint USDT --amount 0.01 --slippage-bps 100 --dry-run
```
**Expected:**
- Exit code 0
- `dry_run: true`
- `slippageBps` = 100

---

## L4 — On-Chain Tests (real broadcast, requires SOL balance)

### TC-L4-01: swap SOL→USDC 0.01 SOL (MAIN L4 TEST)
```bash
./target/release/jupiter swap --input-mint SOL --output-mint USDC --amount 0.01
```
**Prerequisites:**
- SOL balance >= 0.02 (0.01 swap + ~0.001 gas)
- onchainos wallet configured for chain 501

**Expected:**
- Exit code 0
- JSON with: `ok: true`, `txHash` (non-empty string), `input`, `output_estimate`, `price_impact`, `wallet`
- `txHash` is a valid Solana transaction signature (base58, ~88 chars)
- `input` = "0.01 SOL"
- `output_estimate` shows positive USDC amount
- Transaction confirmed on-chain (verify via Solscan/explorer)

### TC-L4-02: wallet balance check (pre-L4)
```bash
onchainos wallet balance --chain 501
```
**Expected:**
- JSON output with SOL balance
- Balance >= 0.02 SOL to proceed

---

## Negative / Edge Cases

### TC-NEG-01: get-quote missing required arg
```bash
./target/release/jupiter get-quote --input-mint SOL --amount 0.1
```
**Expected:** Non-zero exit, error about missing `--output-mint`

### TC-NEG-02: invalid token symbol
```bash
./target/release/jupiter get-quote --input-mint INVALIDTOKEN --output-mint USDC --amount 0.1
```
**Expected:** API error or "token not found" — should not panic

### TC-NEG-03: zero amount
```bash
./target/release/jupiter get-quote --input-mint SOL --output-mint USDC --amount 0
```
**Expected:** API error response or validation error

### TC-NEG-04: swap without wallet (no onchainos)
```bash
./target/release/jupiter swap --input-mint SOL --output-mint USDC --amount 0.01
```
**Expected (if no wallet configured):** Error from `onchainos::resolve_wallet_solana()` — graceful error, not panic
