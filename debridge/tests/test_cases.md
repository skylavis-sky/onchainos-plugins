# deBridge DLN Plugin — Test Cases

## Overview

All tests below are read-only (no live transactions). The `--dry-run` flag must be passed for bridge tests to prevent on-chain execution.

---

## TC-01: get-chains — List supported chains

**Command:**
```
debridge get-chains
```

**Expected behavior:**
- Calls `GET https://dln.debridge.finance/v1.0/supported-chains-info`
- Prints a table of chain IDs and names
- Solana appears as chain ID 7565164 with note "(onchainos chain ID: 501)"
- Output includes at least: Ethereum (1), Arbitrum (42161), Base (8453), Solana (7565164)

**Expected exit code:** 0

---

## TC-02: get-quote — EVM to EVM (Arbitrum USDC -> Base USDC)

**Command:**
```
debridge get-quote \
  --src-chain-id 42161 \
  --dst-chain-id 8453 \
  --src-token 0xaf88d065e77c8cc2239327c5edb3a432268e5831 \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000
```

**Expected behavior:**
- Calls `GET /v1.0/dln/order/create-tx` with srcChainId=42161, dstChainId=8453 (no authority/recipient params)
- Prints quote with input amount, estimated output amount, fix fee (wei), fill time
- No `tx` field in response (quote-only mode)
- Output amount is slightly less than input (fees deducted)
- Fill time ~10 seconds

**Expected exit code:** 0

---

## TC-03: get-quote — Solana USDC -> Base USDC

**Command:**
```
debridge get-quote \
  --src-chain-id 501 \
  --dst-chain-id 8453 \
  --src-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000
```

**Expected behavior:**
- Converts onchainos chain 501 to deBridge API chain ID 7565164
- Calls API with srcChainId=7565164
- Returns estimation without tx data

**Expected exit code:** 0

---

## TC-04: get-quote — EVM to Solana (Base USDC -> Solana USDC)

**Command:**
```
debridge get-quote \
  --src-chain-id 8453 \
  --dst-chain-id 501 \
  --src-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --dst-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 1000000
```

**Expected behavior:**
- dst chain ID 501 maps to deBridge ID 7565164
- Returns estimation

**Expected exit code:** 0

---

## TC-05: get-quote — Native ETH (Ethereum -> Base USDC)

**Command:**
```
debridge get-quote \
  --src-chain-id 1 \
  --dst-chain-id 8453 \
  --src-token 0x0000000000000000000000000000000000000000 \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000000000000
```

**Expected behavior:**
- Native ETH (zero address) accepted by API
- Returns USDC estimation on Base
- No approve needed for native ETH

**Expected exit code:** 0

---

## TC-06: bridge --dry-run — EVM to EVM

**Command:**
```
debridge bridge \
  --src-chain-id 42161 \
  --dst-chain-id 8453 \
  --src-token 0xaf88d065e77c8cc2239327c5edb3a432268e5831 \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000 \
  --dry-run
```

**Expected behavior:**
- Resolves EVM wallet address via onchainos (chain 42161)
- Fetches quote (quote-only call first)
- Builds full tx (with authority/recipient)
- Checks USDC allowance via eth_call to Arbitrum RPC
- If allowance insufficient: prints approve calldata, skips actual approve (dry-run)
- Prints createOrder calldata and simulated txHash
- Prints "DRY RUN COMPLETE" and order ID
- No transactions submitted on-chain

**Expected exit code:** 0

---

## TC-07: bridge --dry-run — Solana to EVM

**Command:**
```
debridge bridge \
  --src-chain-id 501 \
  --dst-chain-id 8453 \
  --src-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000 \
  --dry-run
```

**Expected behavior:**
- Resolves Solana wallet via `onchainos wallet balance --chain 501` (no --output json)
- Resolves Base EVM wallet for destination
- Calls API with srcChainId=7565164 to get hex-encoded VersionedTransaction
- Converts hex tx to base58 (hex_to_base58)
- Prints dry-run result without submitting

**Expected exit code:** 0

---

## TC-08: bridge --dry-run — EVM to Solana

**Command:**
```
debridge bridge \
  --src-chain-id 8453 \
  --dst-chain-id 501 \
  --src-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --dst-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 1000000 \
  --dry-run
```

**Expected behavior:**
- API called with dstChainId=7565164
- skipSolanaRecipientValidation=true passed to API
- Solana wallet resolved for destination
- Base USDC approve calldata generated (skipped in dry-run)
- EVM createOrder calldata printed

**Expected exit code:** 0

---

## TC-09: bridge --dry-run — Native ETH source (no approve)

**Command:**
```
debridge bridge \
  --src-chain-id 1 \
  --dst-chain-id 8453 \
  --src-token 0x0000000000000000000000000000000000000000 \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000000000000 \
  --dry-run
```

**Expected behavior:**
- src_token is NATIVE_EVM (zero address)
- Skips ERC-20 approve entirely
- Prints "Native token — skipping ERC-20 approve"
- tx.value will include protocol fee + ETH amount

**Expected exit code:** 0

---

## TC-10: bridge --dry-run — with explicit recipient override

**Command:**
```
debridge bridge \
  --src-chain-id 42161 \
  --dst-chain-id 8453 \
  --src-token 0xaf88d065e77c8cc2239327c5edb3a432268e5831 \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000 \
  --recipient 0x1234567890123456789012345678901234567890 \
  --dry-run
```

**Expected behavior:**
- Destination wallet set to 0x1234... (not auto-resolved)
- dstChainOrderAuthorityAddress and dstChainTokenOutRecipient both set to 0x1234...

**Expected exit code:** 0

---

## TC-11: get-status — Query order status

**Command:**
```
debridge get-status --order-id 0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890
```

**Expected behavior:**
- Calls `GET /v1.0/dln/order/0xabcdef.../status`
- Prints order ID, status string, human-readable description
- If order not found: API returns error; plugin shows API error message

**Expected exit code:** 0 (or non-zero with clear error message if order not found)

---

## TC-12: hex_to_base58 unit behavior

**Test:** Verify the hex -> base58 conversion used for Solana transactions.

**Input (example short hex):** `"0x0100000000000000"`

**Expected behavior:**
- `hex::decode("0100000000000000")` -> bytes `[1, 0, 0, 0, 0, 0, 0, 0]`
- `bs58::encode(bytes)` -> valid base58 string
- Resulting string passed to `--unsigned-tx` parameter

**Verification:** Run a Solana bridge dry-run and observe that the base58 length printed is a valid encoded length (not zero, not "error").

---

## TC-13: Chain ID mapping verification

**Verify** `onchainos_to_debridge_chain` produces correct mappings:

| onchainos ID | Expected deBridge API ID |
|-------------|--------------------------|
| 1 | "1" |
| 42161 | "42161" |
| 8453 | "8453" |
| 10 | "10" |
| 56 | "56" |
| 137 | "137" |
| 501 | "7565164" |

**Test:** Run `get-quote` with `--src-chain-id 501` and verify the API is called with srcChainId=7565164 (check via API logs or response content).

---

## TC-14: ERC-20 approve calldata encoding

**Verify** `encode_approve` produces correct ABI-encoded calldata.

**Input:**
- spender: `0xeF4fB24aD0916217251F553c0596F8Edc630EB66`
- amount: `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` (max uint256)

**Expected output:**
```
0x095ea7b3
000000000000000000000000eF4fB24aD0916217251F553c0596F8Edc630EB66  (padded to 32 bytes)
ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff  (padded to 32 bytes)
```

Total: 68 bytes = `0x095ea7b3` + 64 hex chars spender + 64 hex chars amount

---

## TC-15: Error handling — invalid chain ID

**Command:**
```
debridge get-quote \
  --src-chain-id 99999 \
  --dst-chain-id 8453 \
  --src-token 0xaf88d065e77c8cc2239327c5edb3a432268e5831 \
  --dst-token 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
  --amount 1000000
```

**Expected behavior:**
- API returns HTTP error (unsupported chain)
- Plugin prints API error with status code
- Exits with non-zero code

**Expected exit code:** non-zero

---

## Notes

- All amounts are in token base units (e.g., 1000000 = 1 USDC with 6 decimals, 1000000000000000 = 0.001 ETH)
- Do NOT run bridge tests without `--dry-run` unless in a controlled test environment
- The deBridge DLN API is unauthenticated; rate limit is 50 RPM
- Tx quotes expire ~30s after creation — do not delay between quote and bridge submission in production
