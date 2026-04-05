# Mayan Plugin Test Cases

All tests use --dry-run where transactions would be broadcast. Tests marked
[LIVE] require real wallet balance and should only be run in Phase 3.

---

## TC-01: get-quote Solana USDC -> Base USDC (MCTP route)

**Command:**
```bash
mayan get-quote \
  --from-chain 501 \
  --to-chain 8453 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 100 \
  --slippage 50
```

**Expected:**
- Exits with code 0
- Prints one or more routes (SWIFT, MCTP, or WH)
- MCTP or SWIFT route shown with expectedAmountOut close to 100 USDC
- ETA displayed in seconds
- Relayer fees printed
- "Recommended route:" line printed at the end

---

## TC-02: get-quote Arbitrum ETH -> Solana SOL (Swift route)

**Command:**
```bash
mayan get-quote \
  --from-chain 42161 \
  --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token So11111111111111111111111111111111111111112 \
  --amount 0.01 \
  --slippage 100
```

**Expected:**
- Exits with code 0
- At least one SWIFT route returned
- Price and ETA fields present and non-zero
- "From wallet" / "To wallet" NOT shown (get-quote does not resolve wallets)

---

## TC-03: swap Solana USDC -> Base USDC (dry run)

**Command:**
```bash
mayan swap \
  --from-chain 501 \
  --to-chain 8453 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 100 \
  --dry-run
```

**Expected:**
- Exits with code 0
- Prints "[DRY RUN - no transactions will be broadcast]"
- Prints "[1/4] Resolving wallet addresses..."
- Prints "[2/4] Fetching quote..."
- Prints "[3/4] Building and submitting swap transaction..."
- Source Tx Hash = "" or a dry-run placeholder
- No real transaction sent (dry_run path returns mock)

---

## TC-04: swap Arbitrum ETH -> Solana SOL (dry run)

**Command:**
```bash
mayan swap \
  --from-chain 42161 \
  --to-chain 501 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token So11111111111111111111111111111111111111112 \
  --amount 0.01 \
  --slippage 100 \
  --dry-run
```

**Expected:**
- Exits with code 0
- Native ETH detected — NO approve step printed
- "--amt" value = 10000000000000000 (0.01 ETH in wei)
- Source Tx Hash printed (dry-run placeholder)
- Status check command printed

---

## TC-05: swap Base USDC -> Solana USDC — ERC-20 approve flow (dry run)

**Command:**
```bash
mayan swap \
  --from-chain 8453 \
  --to-chain 501 \
  --from-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 50 \
  --slippage 50 \
  --dry-run
```

**Expected:**
- Exits with code 0
- "ERC-20 token detected" line printed
- Approve calldata contains selector 095ea7b3
- Approve target = Base USDC (0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913)
- "Waiting 3s for approve to confirm..." printed (skipped in dry-run)
- Swap tx submitted to 0x337685fdaB40D39bd02028545a4FfA7D287cC3E2
- Source Tx Hash printed

---

## TC-06: swap Ethereum WETH -> Base USDC (ERC-20, dry run)

**Command:**
```bash
mayan swap \
  --from-chain 1 \
  --to-chain 8453 \
  --from-token 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 0.05 \
  --slippage 100 \
  --dry-run
```

**Expected:**
- Exits with code 0
- ERC-20 approve step appears (WETH on Ethereum)
- Approve target is WETH contract (0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2)
- Swap submitted to Mayan Forwarder (0x337685fdaB40D39bd02028545a4FfA7D287cC3E2)
- Source Tx Hash printed

---

## TC-07: get-status with known tx hash

**Command:**
```bash
mayan get-status \
  --tx-hash 0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef
```

**Expected:**
- Either prints swap status fields (if hash is real) OR
- Prints an HTTP error from explorer API (404/400) with a non-zero exit code
- Does NOT panic

---

## TC-08: get-quote unsupported chain error

**Command:**
```bash
mayan get-quote \
  --from-chain 9999 \
  --to-chain 8453 \
  --from-token 0x0000000000000000000000000000000000000000 \
  --to-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1.0
```

**Expected:**
- Exits with non-zero code
- Prints "Error: Unsupported from-chain: 9999"

---

## TC-09: swap with slippage at max boundary (300 bps)

**Command:**
```bash
mayan swap \
  --from-chain 501 \
  --to-chain 1 \
  --from-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --to-token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --amount 10 \
  --slippage 300 \
  --dry-run
```

**Expected:**
- Exits with code 0
- Slippage = 300 bps accepted without error
- Quote fetched and best route shown

---

## TC-10: get-quote Polygon USDC -> Solana USDC

**Command:**
```bash
mayan get-quote \
  --from-chain 137 \
  --to-chain 501 \
  --from-token 0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 25
```

**Expected:**
- Exits with code 0
- Routes displayed for Polygon -> Solana path
- MCTP or SWIFT route preferred

---

## Notes on test execution

- TC-01, TC-02, TC-07, TC-08, TC-09, TC-10: Read-only API calls, safe to run anytime.
- TC-03 through TC-06: Use --dry-run, safe to run without wallet balance.
- [LIVE] tests (no --dry-run): Require funded wallet, run in Phase 3 only.
- For Solana wallet tests, ensure onchainos is configured with a Solana keypair.
- For EVM tests, ensure onchainos is configured with an EVM keypair on the target chain.
