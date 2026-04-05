# Test Cases — solv-solvbtc

## TC-01: get-nav (offline query)

**Command:**
```
solv-solvbtc get-nav
```

**Expected behavior:**
- Calls DeFiLlama Coins API for SolvBTC (Arbitrum + Ethereum) and xSolvBTC
- Calls DeFiLlama TVL API for Solv Protocol total TVL
- Prints SolvBTC price in USD (e.g. ~$65,000-$100,000 range)
- Prints xSolvBTC price in USD
- Prints xSolvBTC NAV ratio (should be >= 1.000, currently ~1.034)
- Prints xSolvBTC accumulated yield percentage
- Prints Solv Protocol TVL in $M
- Prints yield note about variable strategy-dependent yield
- Exits with code 0

**Pass criteria:** All prices > 0, NAV >= 1.0, no panic.

---

## TC-02: get-balance on Arbitrum

**Command:**
```
solv-solvbtc get-balance --chain 42161
```

**Expected behavior:**
- Resolves wallet address via `onchainos wallet balance --chain 42161`
- Performs eth_call `balanceOf(wallet)` on SolvBTC token (0x3647c54c4c2C65bC7a2D63c0Da2809B399DBBDC0) via Arbitrum RPC
- Prints wallet address and chain
- Prints SolvBTC balance in human-readable form (e.g. "0.001 SolvBTC")
- Does NOT print xSolvBTC balance (Arbitrum has no xSolvBTC)
- Exits with code 0

**Pass criteria:** No error, balance displayed correctly (may be 0.0 if wallet has none).

---

## TC-03: mint dry-run on Arbitrum

**Command:**
```
solv-solvbtc mint --amount 0.001 --chain 42161 --dry-run
```

**Expected behavior:**
- Prints chain info: Arbitrum (42161)
- Prints WBTC raw amount: 100000 (0.001 * 1e8)
- Prints approve calldata starting with `0x095ea7b3`
- Prints deposit calldata starting with `0x672262e5`
- Both approve and deposit return dry-run txHash: `0x000...000`
- Prints "[DRY-RUN]" notice
- Does NOT broadcast any transaction
- Exits with code 0

**Pass criteria:** Calldata selectors are correct, no live transactions sent.

---

## TC-04: redeem dry-run on Arbitrum (with redemption warning)

**Command:**
```
solv-solvbtc redeem --amount 0.001 --chain 42161 --dry-run
```

**Expected behavior:**
- Prints prominent "WARNING: Redemption is NOT instant!" message
- Mentions ERC-3525 SFT redemption ticket
- Mentions OpenFundMarket queue
- Mentions cancel-redeem option
- Prints approve calldata for SolvBTC (selector 0x095ea7b3)
- Prints withdrawRequest calldata (selector 0xd2cfd97d)
- SolvBTC raw amount = 1000000000000000000 (0.001 * 1e18)
- Does NOT broadcast any transaction
- Exits with code 0

**Pass criteria:** Non-instant warning displayed, correct selectors used.

---

## TC-05: wrap dry-run (Ethereum only)

**Command:**
```
solv-solvbtc wrap --amount 0.05 --dry-run
```

**Expected behavior:**
- Prints chain: Ethereum (1)
- Fetches current xSolvBTC NAV from DeFiLlama
- Prints estimated xSolvBTC output (~0.05 / 1.034 ≈ 0.0484 xSolvBTC)
- Prints approve calldata (SolvBTC -> XSolvBTCPool, selector 0x095ea7b3)
- Prints XSolvBTCPool.deposit calldata (selector 0xb6b55f25)
- Both return dry-run txHash
- Exits with code 0

**Pass criteria:** Chain is forced to Ethereum (1), correct pool address used.

---

## TC-06: unwrap dry-run (Ethereum only)

**Command:**
```
solv-solvbtc unwrap --amount 0.05 --dry-run
```

**Expected behavior:**
- Prints chain: Ethereum (1)
- Prints xSolvBTC raw amount: 50000000000000000 (0.05 * 1e18)
- Prints 0.05% fee deduction (fee_amount > 0)
- Prints estimated SolvBTC after fee (slightly less than NAV * 0.05)
- Prints approve calldata (xSolvBTC -> XSolvBTCPool, selector 0x095ea7b3)
- Prints XSolvBTCPool.withdraw calldata (selector 0x2e1a7d4d)
- Exits with code 0

**Pass criteria:** Fee is applied, correct token addresses used (xSolvBTC, not SolvBTC).

---

## TC-07: get-balance on Ethereum (shows both SolvBTC and xSolvBTC)

**Command:**
```
solv-solvbtc get-balance --chain 1
```

**Expected behavior:**
- Resolves wallet on chain 1
- Queries SolvBTC balance (0x7a56e1c57c7475ccf742a1832b028f0456652f97)
- Queries xSolvBTC balance (0xd9d920aa40f578ab794426f5c90f6c731d159def)
- Prints both balances
- Exits with code 0

**Pass criteria:** Both token balances displayed (may be 0.0 if wallet has none).

---

## TC-08: cancel-redeem dry-run

**Command:**
```
solv-solvbtc cancel-redeem \
  --redemption-addr 0xabcdef1234567890abcdef1234567890abcdef12 \
  --redemption-id 42 \
  --chain 42161 \
  --dry-run
```

**Expected behavior:**
- Prints redemption address and token ID
- Prints cancelWithdrawRequest calldata (selector 0x42c7774b)
- Returns dry-run txHash
- Exits with code 0

**Pass criteria:** Correct selector (0x42c7774b) in calldata, no live transaction.

---

## TC-09: mint amount precision (edge case)

**Command:**
```
solv-solvbtc mint --amount 0.00001 --chain 42161 --dry-run
```

**Expected behavior:**
- WBTC raw amount = 1000 (0.00001 * 1e8)
- Approve calldata encodes 1000 correctly (padded to 32 bytes: ...000003e8)
- Exits with code 0

**Pass criteria:** No rounding error, correct u128 encoding.

---

## TC-10: invalid chain ID

**Command:**
```
solv-solvbtc mint --amount 0.001 --chain 137 --dry-run
```

**Expected behavior:**
- Prints error: "Unsupported chain ID 137. Supported: 1 (Ethereum), 42161 (Arbitrum)"
- Exits with non-zero code

**Pass criteria:** Graceful error message, no panic.
