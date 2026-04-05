# Across Plugin Test Cases

## L2 Read Operation Tests (No Wallet Required)

### TC-001: get-routes — All routes (no filter)

**Command:**
```bash
across get-routes
```

**Expected:**
- Exit code: 0
- Output lists multiple routes with originChainId, destinationChainId, originTokenSymbol, destinationTokenSymbol
- Output includes at least one USDC route between Ethereum and Optimism

---

### TC-002: get-routes — Filtered by origin and destination chain

**Command:**
```bash
across get-routes \
  --origin-chain-id 8453 \
  --destination-chain-id 137
```

**Expected:**
- Exit code: 0
- Only routes from chain 8453 (Base) to chain 137 (Polygon) are shown
- At least one USDC route is present

---

### TC-003: get-limits — USDC from Ethereum to Optimism

**Command:**
```bash
across get-limits \
  --input-token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 \
  --origin-chain-id 1 \
  --destination-chain-id 10
```

**Expected:**
- Exit code: 0
- minDeposit is a non-zero numeric string
- maxDeposit > maxDepositInstant
- liquidReserves is a numeric string

---

### TC-004: get-quote — 100 USDC from Ethereum to Optimism

**Command:**
```bash
across get-quote \
  --input-token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 \
  --origin-chain-id 1 \
  --destination-chain-id 10 \
  --amount 100000000
```

**Expected:**
- Exit code: 0
- outputAmount is a numeric string less than 100000000 (fees deducted)
- isAmountTooLow: false
- estimatedFillTimeSec is a small integer (1-120)
- spokePoolAddress is a valid Ethereum address

---

### TC-005: get-quote — Amount too low (1 unit = 0.000001 USDC)

**Command:**
```bash
across get-quote \
  --input-token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 \
  --origin-chain-id 1 \
  --destination-chain-id 10 \
  --amount 1
```

**Expected:**
- Exit code: non-zero (error)
- Error message contains "Amount too low" and the minimum deposit amount
- No on-chain transaction attempted

---

## L3 Dry-Run Tests (Wallet Needed, No On-Chain Tx)

### TC-006: bridge dry-run — ERC-20 USDC from Ethereum to Optimism

**Command:**
```bash
across bridge \
  --input-token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --output-token 0x0b2C639c533813f4Aa9D7837CAf62653d097Ff85 \
  --origin-chain-id 1 \
  --destination-chain-id 10 \
  --amount 100000000 \
  --dry-run
```

**Expected:**
- Exit code: 0
- Output shows "DRY RUN" label
- No transactions submitted to onchainos
- Approve calldata is shown (starts with `0x095ea7b3`)
- Bridge calldata is shown (starts with `0x7b939232`)
- Simulated txHash: `0x0000...0000`
- outputAmount displayed

---

### TC-007: bridge dry-run — Native ETH from Ethereum to Optimism

**Command:**
```bash
across bridge \
  --input-token 0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE \
  --output-token 0x4200000000000000000000000000000000000006 \
  --origin-chain-id 1 \
  --destination-chain-id 10 \
  --amount 10000000000000000 \
  --dry-run
```

**Expected:**
- Exit code: 0
- Output shows "Native ETH bridge — skipping ERC-20 approve"
- ETH value (wei): Some(10000000000000000)
- Bridge calldata starts with `0x7b939232`
- No approve calldata shown

---

### TC-008: bridge dry-run — WETH from Arbitrum to Base

**Command:**
```bash
across bridge \
  --input-token 0x82aF49447D8a07e3bd95BD0d56f35241523fBab1 \
  --output-token 0x4200000000000000000000000000000000000006 \
  --origin-chain-id 42161 \
  --destination-chain-id 8453 \
  --amount 10000000000000000 \
  --dry-run
```

**Expected:**
- Exit code: 0
- Approve calldata shown for WETH on Arbitrum
- SpokePool shown as `0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A` (or API-returned address)
- Bridge calldata starts with `0x7b939232`

---

### TC-009: get-status — Status lookup by tx hash (not yet filled)

**Command:**
```bash
across get-status \
  --tx-hash 0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef \
  --origin-chain-id 1
```

**Expected:**
- Exit code: 0 (API may return status or 404-style error gracefully)
- If found: prints status field (pending/filled/expired)
- If not found: prints API error message without panic

---

### TC-010: get-status — Missing required parameters

**Command:**
```bash
across get-status
```

**Expected:**
- Exit code: non-zero (error)
- Error message: "Must provide at least one of: --tx-hash, --deposit-id, or --relay-data-hash"
- No API call made

---

## ABI Encoding Unit Tests

These are covered by `cargo test` in `src/abi.rs`:

### TC-011: encode_approve produces correct length
- `encode_approve("0x5c7B...", u128::MAX)` → length = 138 chars (2 + 8 + 64 + 64)
- Starts with `0x095ea7b3`

### TC-012: encode_deposit_v3 produces correct length
- 13 ABI words (12 static + 2 for dynamic bytes header) → 842 chars total
- Starts with `0x7b939232`

Run with:
```bash
cargo test
```
