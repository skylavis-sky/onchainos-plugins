# Test Results Report

- Date: 2026-04-09
- DApp: pump.fun
- Supported chains: Solana only
- Solana test chain: mainnet (501)
- Wallet: `6hY15MNMZtjF15sPtuSozxjrrZPyrDmqBaC48496T8UY`
- Compile: ✅
- Lint: ✅
- Overall pass standard: Solana DApp → all Solana operations pass

---

## Summary

| Total | L1 Build | L2 Read | L3 Dry-run | L4 On-chain | Failed | Blocked |
|-------|----------|---------|------------|-------------|--------|---------|
| 11    | 1        | 5       | 3          | 2           | 0      | 0       |

---

## Detailed Results

| # | Scenario                                                         | Level | Command                                                                              | Result  | TxHash / Note                                                                 |
|---|------------------------------------------------------------------|-------|--------------------------------------------------------------------------------------|---------|-------------------------------------------------------------------------------|
| 1 | Build pump-fun plugin (release)                                  | L1    | `cargo build --release`                                                              | ✅ PASS | No warnings                                                                   |
| 2 | Get info on active bonding curve token (SaveCT)                  | L2    | `get-token-info --mint 8wLd...pump`                                                  | ✅ PASS | complete=false, graduation_progress=0.32%, price=0.0000285 SOL/token          |
| 3 | Get info on graduated token (BURNIE)                             | L2    | `get-token-info --mint CGEDT...pump`                                                 | ✅ PASS | complete=true, status="Graduated (trading on PumpSwap/Raydium)"               |
| 4 | Get buy price for 0.001 SOL of SaveCT                            | L2    | `get-price --mint ... --direction buy --amount 1000000`                              | ✅ PASS | amount_out=35,124,920,625 tokens                                               |
| 5 | Get sell price for 1,000,000 token units of SaveCT               | L2    | `get-price --mint ... --direction sell --amount 1000000 --fee-bps 100`               | ✅ PASS | amount_out=28 lamports                                                         |
| 6 | Error: invalid direction in get-price                            | L2    | `get-price --direction swap ...`                                                      | ✅ PASS | ok=false, exit 1, "direction must be 'buy' or 'sell'..."                      |
| 7 | Error: invalid mint address                                      | L2    | `get-token-info --mint not_a_valid_address`                                          | ✅ PASS | ok=false, exit 1, "Invalid Base58 string"                                     |
| 8 | Simulate buy 0.001 SOL (dry-run)                                 | L3    | `buy --mint ... --sol-amount 0.001 --slippage-bps 200 --dry-run`                    | ✅ PASS | ok=true, dry_run=true, tx_hash=""                                              |
| 9 | Simulate sell all tokens (dry-run)                               | L3    | `sell --mint ... --dry-run`                                                          | ✅ PASS | ok=true, dry_run=true, token_amount="\<full balance\>"                         |
|10 | Simulate sell specific token amount (dry-run)                    | L3    | `sell --mint ... --token-amount 1000000 --dry-run`                                  | ✅ PASS | ok=true, dry_run=true, token_amount="1000000"                                 |
|11 | **Buy 0.001 SOL of SaveCT on-chain**                             | L4    | `buy --mint 8wLd...pump --sol-amount 0.001 --slippage-bps 200`                      | ✅ PASS | tx=`5NjkVg...xuAp` — verified on Solana mainnet                               |
|12 | **Sell all SaveCT on-chain (sell-all)**                          | L4    | `sell --mint 8wLd...pump --slippage-bps 200`                                        | ✅ PASS | tx=`54aosi...yeLq`, resolved balance=34691.247378, sold via onchainos swap    |

---

## Bugs Fixed During Testing

| # | Issue                              | Root Cause                                                                                       | Fix                                                      | File              |
|---|------------------------------------|--------------------------------------------------------------------------------------------------|----------------------------------------------------------|-------------------|
| 1 | `onchainos swap execute` missing `--wallet` | swap execute requires `--wallet <address>` arg; not included in original command construction | Added `resolve_wallet_solana()` + `--wallet &wallet` arg | `src/onchainos.rs` |
| 2 | Sell-all balance lookup fails      | `get_token_balance` matched `asset["address"]` (wallet pubkey) instead of `asset["tokenAddress"]` (mint) | Changed field lookup to `asset["tokenAddress"]`          | `src/onchainos.rs` |

---

## Transactions

- **L4 Buy:** [`5NjkVgAArhjLvqPWF4N8gza8iogwztpyvwwuqC6a2B9fnRJPA9Eb8ZGbpN7Efbw6JryNL1JKGKiXkzBpf9bjxuAp`](https://solscan.io/tx/5NjkVgAArhjLvqPWF4N8gza8iogwztpyvwwuqC6a2B9fnRJPA9Eb8ZGbpN7Efbw6JryNL1JKGKiXkzBpf9bjxuAp)
- **L4 Sell:** [`54aosiyrUBh1VmsDLEYUhzLg8dF6HGgwFi1KmpMNUwJq5nYLTJVi85pbBgXRtz5MhWrK7DemnYztGuoQ4RbnyeLq`](https://solscan.io/tx/54aosiyrUBh1VmsDLEYUhzLg8dF6HGgwFi1KmpMNUwJq5nYLTJVi85pbBgXRtz5MhWrK7DemnYztGuoQ4RbnyeLq)

---

## Token Used for Testing

- **Active token:** `8wLdQgRS2rsk2UmPmL7xQjPnhCFPbb18yGJ8RQKepump` (SaveCT / Justice For CT)
  - Verified `complete: false` before all tests
  - Graduation progress: ~0.32% at test time
- **Graduated token:** `CGEDT9QZDvvH5GmVkWJH2BXiMJqMJySC9ihWyr7Spump` (BURNIE)
  - Used to verify graduated status flow

## Notes

- `create-token` removed — requires 2-signer flow (mint keypair + MPC wallet) incompatible with onchainos MPC model
- buy/sell now route through `onchainos swap execute --chain solana` — works for both bonding curve and graduated tokens
- sell-all resolves live balance from `onchainos wallet balance --chain 501` using `tokenAddress` field
