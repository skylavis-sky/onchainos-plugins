# pump-fun Plugin — Test Cases

## Test Environment

- Chain: Solana mainnet (chain 501)
- Program: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`
- RPC: `https://api.mainnet-beta.solana.com` (or set `HELIUS_RPC_URL` for higher rate limits)
- onchainos must be installed and wallet configured for write ops

---

## TC-01: get-token-info — Active bonding curve token

**Command:**
```bash
pump-fun get-token-info --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R
```

**Expected output (JSON):**
```json
{
  "ok": true,
  "mint": "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
  "virtual_token_reserves": <non-zero u64>,
  "virtual_sol_reserves": <non-zero u64>,
  "real_token_reserves": <u64>,
  "real_sol_reserves": <u64>,
  "token_total_supply": <non-zero u64>,
  "complete": false,
  "creator": "<base58 pubkey>",
  "price_sol_per_token": <positive f64>,
  "market_cap_sol": <non-zero u64>,
  "final_market_cap_sol": <non-zero u64>,
  "graduation_progress_pct": <0.0 to 100.0>,
  "status": "Active (bonding curve)"
}
```

**Validation:**
- `ok == true`
- `complete == false`
- `price_sol_per_token > 0`
- `graduation_progress_pct` between 0 and 100
- All reserve fields are non-zero

---

## TC-02: get-token-info — Graduated token

**Command:**
```bash
# Use a known graduated token mint
pump-fun get-token-info --mint <graduated_mint>
```

**Expected output:**
```json
{
  "ok": true,
  "complete": true,
  "status": "Graduated (trading on PumpSwap/Raydium)",
  ...
}
```

**Validation:**
- `complete == true`
- `status` contains "Graduated"

---

## TC-03: get-price — Buy direction

**Command:**
```bash
pump-fun get-price \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --direction buy \
  --amount 100000000
```
*(100000000 lamports = 0.1 SOL)*

**Expected output:**
```json
{
  "ok": true,
  "direction": "buy",
  "amount_in": 100000000,
  "amount_out": <positive u64 tokens>,
  "amount_out_ui": <positive f64>,
  "price_sol_per_token": <positive f64>,
  "market_cap_sol": <non-zero u64>,
  "bonding_complete": false
}
```

**Validation:**
- `amount_out > 0`
- `amount_out_ui > 0`
- No `graduated_warning` field (token is active)

---

## TC-04: get-price — Sell direction

**Command:**
```bash
pump-fun get-price \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --direction sell \
  --amount 1000000 \
  --fee-bps 100
```
*(1000000 token units)*

**Expected output:**
```json
{
  "ok": true,
  "direction": "sell",
  "amount_in": 1000000,
  "amount_out": <positive u64 lamports>,
  "amount_out_ui": <positive f64 SOL>
}
```

**Validation:**
- `amount_out > 0`
- `amount_out_ui > 0`

---

## TC-05: get-price — Invalid direction

**Command:**
```bash
pump-fun get-price --mint <any_mint> --direction swap --amount 100000000
```

**Expected:** Non-zero exit code, error JSON:
```json
{"ok": false, "error": "direction must be 'buy' or 'sell'..."}
```

---

## TC-06: buy — Dry run

**Command:**
```bash
pump-fun buy \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --sol-amount 100000000 \
  --slippage-bps 200 \
  --dry-run
```

**Expected output:**
```json
{
  "ok": true,
  "mint": "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
  "sol_amount_lamports": 100000000,
  "slippage_bps": 200,
  "tx_hash": "",
  "dry_run": true
}
```

**Validation:**
- No onchainos call made
- `dry_run == true`
- `tx_hash` is empty string

---

## TC-07: buy — Live execution (requires onchainos wallet)

**Pre-requisites:**
- onchainos wallet configured with SOL balance ≥ 0.1 SOL + fees
- Token must be on active bonding curve (`complete == false`)

**Command:**
```bash
pump-fun buy \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --sol-amount 100000000 \
  --slippage-bps 200
```

**Expected output:**
```json
{
  "ok": true,
  "tx_hash": "<44-character Solana signature>",
  "sol_amount_lamports": 100000000
}
```

**Validation:**
- `tx_hash` is a 44-88 character base58 Solana signature
- Verify on solscan.io: `https://solscan.io/tx/<tx_hash>`

---

## TC-08: buy — Graduated token (bonding complete)

**Command:**
```bash
pump-fun buy --mint <graduated_mint> --sol-amount 100000000
```

**Expected output:**
```json
{
  "ok": false,
  "graduated_warning": "Token has graduated from bonding curve. Use: onchainos dex swap execute --chain 501"
}
```

**Validation:**
- `ok == false`
- `graduated_warning` present

---

## TC-09: sell — Dry run (sell all)

**Command:**
```bash
pump-fun sell \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --dry-run
```

**Expected output:**
```json
{
  "ok": true,
  "sell_all": true,
  "token_amount": null,
  "dry_run": true
}
```

---

## TC-10: sell — Dry run (specific amount)

**Command:**
```bash
pump-fun sell \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --token-amount 5000000 \
  --dry-run
```

**Expected output:**
```json
{
  "ok": true,
  "sell_all": false,
  "token_amount": 5000000,
  "dry_run": true
}
```

---

## TC-11: sell — Live execution (requires onchainos wallet + token balance)

**Pre-requisites:**
- onchainos wallet holds tokens of the specified mint
- Token must be on active bonding curve

**Command:**
```bash
pump-fun sell \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --token-amount 1000000
```

**Expected output:**
```json
{
  "ok": true,
  "tx_hash": "<44-88 char Solana signature>",
  "sell_all": false,
  "token_amount": 1000000
}
```

---

## TC-12: create-token — Dry run

**Command:**
```bash
pump-fun create-token \
  --name "Test Token" \
  --symbol "TEST" \
  --description "A test token" \
  --image-path /tmp/test.png \
  --initial-buy-sol 0 \
  --dry-run
```

**Expected output:**
```json
{
  "ok": true,
  "mint_address": "<base58 pubkey>",
  "name": "Test Token",
  "symbol": "TEST",
  "initial_buy_sol_lamports": 0,
  "tx_hash": "",
  "dry_run": true
}
```

**Validation:**
- `mint_address` is a valid base58 pubkey (fresh keypair each run)
- `dry_run == true`
- No IPFS upload attempted

---

## TC-13: create-token — With initial buy, dry run

**Command:**
```bash
pump-fun create-token \
  --name "Moon Cat" \
  --symbol "MCAT" \
  --description "The cats are going to the moon" \
  --image-path /tmp/cat.png \
  --initial-buy-sol 500000000 \
  --slippage-bps 200 \
  --twitter "https://twitter.com/mooncattoken" \
  --dry-run
```

**Expected output:**
```json
{
  "ok": true,
  "name": "Moon Cat",
  "symbol": "MCAT",
  "initial_buy_sol_lamports": 500000000,
  "dry_run": true
}
```

---

## TC-14: Invalid mint address

**Command:**
```bash
pump-fun get-token-info --mint not_a_valid_address
```

**Expected:** Exit code 1, stderr JSON:
```json
{"ok": false, "error": "Invalid mint address 'not_a_valid_address': ..."}
```

---

## TC-15: Custom RPC URL via environment variable

**Command:**
```bash
HELIUS_RPC_URL=https://mainnet.helius-rpc.com/?api-key=<key> \
pump-fun get-token-info --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R
```

**Validation:**
- Uses Helius RPC instead of public mainnet-beta
- Same output format as TC-01

---

## TC-16: Custom RPC URL via --rpc-url flag

**Command:**
```bash
pump-fun get-token-info \
  --mint 4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R \
  --rpc-url https://mainnet.helius-rpc.com/?api-key=<key>
```

**Validation:**
- `--rpc-url` flag takes precedence over `HELIUS_RPC_URL` env var
- Same output format as TC-01

---

## Notes

- All write ops (buy, sell, create-token) check `bonding_complete` before submitting
- Blockhash is fetched immediately before tx construction; onchainos is called immediately after serialization (no caching)
- `--dry-run` never calls `onchainos wallet balance` (dry_run guard before wallet resolution)
- `sell --token-amount` omitted → `sell_all: true` → passes `None` to `get_sell_instructions`
