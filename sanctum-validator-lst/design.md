# Sanctum Validator LSTs — Plugin Design

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `sanctum-validator-lst` |
| `dapp_name` | Sanctum Validator LSTs |
| `version` | 0.1.0 |
| `target_chains` | Solana (chain 501) |
| `category` | defi-protocol |
| `integration_path` | REST API (Sanctum Router API + Sanctum Extra API) |

---

## §1 Feasibility

| Check | Result |
|-------|--------|
| Rust SDK available? | No — no standalone Rust SDK for validator LSTs; Sanctum S Router API is REST-based |
| SDK stack support? | TypeScript only (`@sanctum/s-client`); Rust must use raw HTTP via reqwest |
| REST API available? | **Yes** — `https://sanctum-s-api.fly.dev` (swap/quote/stake tx) + `https://extra-api.sanctum.so` (APY, TVL, sol-value, LST list) |
| Official onchainos Skill? | No |
| Community skill? | No known plugin-store skill for this DApp |
| Supported chains | Solana only (chain 501) |
| onchainos broadcast needed? | Yes — swap and stake ops return serialized Solana tx (base64) → must convert to base58 → `onchainos wallet contract-call --chain 501 --unsigned-tx <base58> --force` |
| Overlap with `sanctum-infinity`? | **Partial** — both use the same Sanctum S Router API and Extra API. Key differences: (1) `sanctum-infinity` focuses on the INF pool token (LP deposit/withdraw); this plugin focuses on **individual validator LSTs** (jitoSOL, mSOL, bSOL, etc.) and the SOL→LST staking path. (2) `swap-lst` here uses the same `/v1/swap` endpoint but routes between validator LSTs (not into/out of the INF LP token). (3) `stake` here is the SPL Stake Pool `DepositSol` instruction called per-validator pool — NOT covered by `sanctum-infinity`. |

**Integration path**: REST API — Sanctum Router API for swap/quote; Sanctum Extra API for LST metadata, APY, TVL, SOL value. SPL Stake Pool instructions built manually for the `stake` operation (same pattern as `jito` plugin).

---

## §2 Interface Mapping

### Operations Table

| Operation | Type | Description |
|-----------|------|-------------|
| `list-lsts` | Off-chain read | List tracked validator LSTs with symbol, mint, APY, TVL, SOL-value |
| `get-quote` | Off-chain read | Quote swap between two LSTs via Sanctum Router (v2) |
| `swap-lst` | On-chain write | Swap between two validator LSTs via Sanctum Router |
| `stake` | On-chain write | Stake SOL into a specific validator LST pool (SPL DepositSol) |
| `get-position` | Off-chain read | Get user's LST balances across all tracked validator LSTs |

---

### Solana Write Operations

#### `swap-lst` — Swap between two validator LSTs via Router

| Field | Value |
|-------|-------|
| Program ID (`--to`) | `5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx` (Sanctum S Controller / SPool) |
| Quote Endpoint | `GET https://sanctum-s-api.fly.dev/v2/swap/quote` |
| Swap TX Endpoint | `POST https://sanctum-s-api.fly.dev/v1/swap` |
| TX Encoding | base64 (from API) → **must convert to base58** before `--unsigned-tx` |
| Amount Unit | Raw atomics (U64 as string); all Sanctum LSTs have 9 decimals |

**Quote request (GET)**:
```
GET https://sanctum-s-api.fly.dev/v2/swap/quote
  ?input=<input_lst_mint_b58>
  &outputLstMint=<output_lst_mint_b58>
  &amount=<amount_u64_str>
  &mode=ExactIn
```

**Quote response**:
```json
{
  "inAmount": "1000000000",
  "outAmount": "998500000",
  "swapSrc": "SPool",
  "fees": [
    { "code": "S_POOL_REMOVE_LIQUIDITY", "rate": "0.001", "amt": "1000000", "mint": "<mint>" }
  ]
}
```

**Swap TX request (POST)**:
```json
{
  "input": "<input_lst_mint>",
  "outputLstMint": "<output_lst_mint>",
  "amount": "<amount_atomics_str>",
  "quotedAmount": "<min_out_atomics_str>",
  "mode": "ExactIn",
  "signer": "<wallet_b58_pubkey>",
  "swapSrc": "SPool"
}
```

**Swap TX response**:
```json
{ "tx": "<base64_versioned_transaction>" }
```

**onchainos invocation**:
```
onchainos wallet contract-call \
  --chain 501 \
  --to 5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx \
  --unsigned-tx <base58_converted_tx> \
  --force
```

> base64→base58 conversion: `base64::decode(tx_b64)` then `bs58::encode(bytes)`.

---

#### `stake` — Stake SOL into a specific validator LST pool

Validator-specific LSTs each have their own SPL Stake Pool account. This operation calls the SPL Stake Pool `DepositSol` instruction (instruction index 14) directly on the validator's stake pool — the **same approach used by the `jito` plugin** for JitoSOL, generalized to any tracked validator LST.

| Field | Value |
|-------|-------|
| Program ID (`--to`) | `SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy` (SPL Stake Pool Program) |
| API Endpoint | None — construct instruction manually from on-chain state |
| RPC Endpoint | `https://api.mainnet-beta.solana.com` |
| TX Encoding | Constructed in Rust as v0 versioned transaction → base64 → **convert to base58** |
| Amount Unit | UI units (SOL), converted to lamports (× 10^9) internally |

**DepositSol instruction layout** (9 bytes total):
```
[14u8] [lamports: u64 little-endian]
```

**Account keys (in order)**:
```
0. stake_pool_account     writable      (per-validator pool account address)
1. withdraw_authority     readonly      PDA([pool_addr_bytes, b"withdraw"], SPL_STAKE_POOL_PROGRAM)
2. reserve_stake          writable      (from stake pool state at offset 130)
3. from_user_lamports     writable+signer  (user wallet)
4. user_pool_token_ata    writable      ATA(user_wallet, pool_mint)
5. manager_fee_account    writable      (from stake pool state at offset 194)
6. referrer_fee_account   writable      (= user_pool_token_ata for simplicity)
7. pool_mint              writable      (LST mint address)
8. system_program         readonly      11111111111111111111111111111111
9. token_program          readonly      TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
```

**Construction flow**:
1. Resolve user wallet via `onchainos wallet balance --chain 501` (no `--output json`)
2. Fetch stake pool account via `getAccountInfo(pool_account_address)` — parse reserve_stake (offset 130), pool_mint (offset 194 area), manager_fee_account
3. Derive withdraw_authority PDA: `find_program_address([pool_addr_bytes, b"withdraw"], STAKE_POOL_PROGRAM_ID)`
4. Derive user ATA: `associated_token_address(user_wallet, pool_mint)`
5. Check if ATA exists; if not, pre-create with `getTokenAccountsByOwner` fallback (note: CreateATA instruction in same tx can fail in simulation — prefer checking first, see jito post-mortem)
6. Build DepositSol instruction
7. Serialize as **v0 versioned transaction** (prefix `0x80`, trailing `0x00` empty address table)
8. Serialize to bytes → base64 → **convert to base58** → `onchainos wallet contract-call --chain 501 --to SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy --unsigned-tx <base58> --force`

> **Note**: After staking, LST tokens appear only after the epoch boundary (~2-3 days). The `get-position` command reflects the new balance only after epoch processing. Do not attempt `unstake` L4 in the same session.

---

### Off-chain Read Operations

#### `list-lsts` — List available validator LSTs

**API calls** (parallel):
1. `GET https://extra-api.sanctum.so/v1/lsts`
   - Response: `{ "lsts": [{ "mint": "<b58>", "symbol": "jitoSOL", "name": "...", "decimals": 9 }, ...] }`
2. `GET https://extra-api.sanctum.so/v1/apy/latest?lst=<mint1>,<mint2>,...`
   - Response: `{ "apys": { "<mint>": 0.0769, ... }, "errs": {} }`
3. `GET https://extra-api.sanctum.so/v1/tvl/current?lst=<mint1>,<mint2>,...`
   - Response: `{ "tvls": { "<mint>": "1234567890000", ... } }`
4. `GET https://extra-api.sanctum.so/v1/sol-value/current?lst=<mint1>,<mint2>,...`
   - Response: `{ "solValues": { "<mint>": "1050000000", ... }, "errs": {} }`

**Output**: table of validator LSTs with symbol, mint address, APY%, TVL (SOL), SOL-value-per-token.

> **Fallback**: If `GET /v1/lsts` returns 403 or empty, fall back to a hardcoded list of the top tracked LSTs (see §5 Config Parameters / known mints table).

---

#### `get-quote` — Quote swap between two LSTs

`GET https://sanctum-s-api.fly.dev/v2/swap/quote?input=<from_mint>&outputLstMint=<to_mint>&amount=<atomics>&mode=ExactIn`

Output: in_amount_ui, out_amount_ui, min_out_ui (with slippage), rate, swap_src, fees.

---

#### `get-position` — Get user LST balances

For each tracked validator LST mint:
1. `onchainos wallet balance --chain 501` → parse Solana wallet address (no `--output json`)
2. For each mint: query token account balance via `getTokenAccountsByOwner(wallet, { mint: mint_address })` — handles non-ATA token accounts
3. `GET https://extra-api.sanctum.so/v1/sol-value/current?lst=<mints...>` → convert balances to SOL equivalent

**Output**: list of LST holdings with balance, SOL equivalent value, and current SOL-per-token rate.

---

## §3 User Scenarios

### Scenario 1: List available validator LSTs with yields

**User**: "Show me all validator LSTs on Sanctum with their APY"

**Action sequence**:
1. [Off-chain] `GET /v1/lsts` → get LST list
2. [Off-chain] `GET /v1/apy/latest?lst=<all_mints>` → get APY per LST
3. [Off-chain] `GET /v1/tvl/current?lst=<all_mints>` → get TVL per LST
4. Output: sorted table by TVL descending, showing symbol, mint (truncated), APY%, TVL

---

### Scenario 2: Get a quote to swap jitoSOL to mSOL

**User**: "How much mSOL will I get for 0.005 jitoSOL via Sanctum?"

**Action sequence**:
1. [Off-chain] Resolve mint addresses for jitoSOL and mSOL from symbol table
2. [Off-chain] `GET /v2/swap/quote?input=J1toso...&outputLstMint=mSoLz...&amount=5000000&mode=ExactIn`
3. Output: in_amount, out_amount, rate, fees, min_out at 0.5% slippage

---

### Scenario 3: Swap jitoSOL to bSOL via Sanctum Router

**User**: "Swap 0.005 jitoSOL to bSOL using Sanctum"

**Action sequence**:
1. [Off-chain] Resolve mint addresses from symbols
2. [Off-chain] `GET /v2/swap/quote` → get expected output
3. Display quote and **ask user to confirm**
4. [Off-chain] `resolve_wallet_solana()` → Solana wallet address
5. [On-chain] `POST /v1/swap` with signer, amount, quotedAmount (with 0.5% slippage) → get base64 tx
6. Convert base64 → base58
7. `onchainos wallet contract-call --chain 501 --to 5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx --unsigned-tx <base58> --force`
8. Output: txHash, amounts, solscan.io link

---

### Scenario 4: Stake SOL into a specific validator LST

**User**: "Stake 0.002 SOL into jupSOL via Sanctum"

**Action sequence**:
1. [Off-chain] Resolve stake pool account address for jupSOL
2. [Off-chain] `resolve_wallet_solana()` → user wallet
3. [Off-chain] `getAccountInfo(jupSOL_pool_account)` → parse stake pool state
4. Derive withdraw authority PDA and user ATA
5. [Off-chain] `getTokenAccountsByOwner(wallet, {mint: jupSOL_mint})` → verify/find token account
6. Build DepositSol instruction (instruction 14), construct v0 versioned transaction
7. Serialize → base64 → base58
8. **Ask user to confirm** before broadcast
9. `onchainos wallet contract-call --chain 501 --to SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy --unsigned-tx <base58> --force`
10. Output: txHash, SOL staked, expected LST tokens (note: credited at next epoch boundary)

---

### Scenario 5: Check all LST positions

**User**: "Show my Sanctum validator LST holdings"

**Action sequence**:
1. [Off-chain] `resolve_wallet_solana()` → wallet address
2. [Off-chain] `getTokenAccountsByOwner(wallet)` → all token accounts
3. Match token accounts against known validator LST mints
4. [Off-chain] `GET /v1/sol-value/current?lst=<held_mints>` → SOL equivalent
5. Output: list of non-zero holdings with symbol, balance, SOL value

---

## §4 External API Dependencies

| API | Purpose | Base URL | Auth |
|-----|---------|----------|------|
| Sanctum Router API | Swap quotes, swap transactions | `https://sanctum-s-api.fly.dev` | None |
| Sanctum Extra API | LST list, APY, TVL, SOL-value | `https://extra-api.sanctum.so` | None |
| Solana JSON-RPC | getAccountInfo, getTokenAccountsByOwner, getLatestBlockhash | `https://api.mainnet-beta.solana.com` | None |

> **Note**: `sanctum-s-api.fly.dev` runs on Fly.io and has experienced 502 downtime in past deployments (see sanctum-infinity retrospective). Implement retry logic and surface clear error messages when the router is unavailable.

> **Note**: `extra-api.sanctum.so` returns HTTP 403 from some environments. If `/v1/lsts` is inaccessible, fall back to the hardcoded LST list in §5. APY/TVL calls use the same host and may similarly fail — treat as non-fatal (display "N/A" for those fields).

---

## §5 Config Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--chain` | `501` | Solana mainnet chain ID |
| `--dry-run` | `false` | Simulate without broadcasting |
| `--slippage` | `0.5` | Slippage tolerance in percent (0.5 = 0.5%) |
| `--lst` | — | LST symbol (e.g. `jitoSOL`) or mint address (B58) |
| `--from` | — | Input LST symbol or mint for swap |
| `--to` | — | Output LST symbol or mint for swap |
| `--amount` | — | Amount in UI units (SOL for stake, LST tokens for swap) |

### Known Validator LST Mint Addresses (Top by TVL)

| Symbol | Name | Mint Address | Pool Program | Stake Pool Account |
|--------|------|--------------|-------------|---------------------|
| jitoSOL | Jito MEV Staked SOL | `J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn` | SPL Stake Pool | `Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb` |
| mSOL | Marinade Staked SOL | `mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So` | Marinade native | N/A (Marinade has custom program) |
| jupSOL | Jupiter Staked SOL | `jupSoLaHXQiZZTSfEWMTRRgpnyFm8f6sZdosWBjx93v` | SanctumSplMulti | Fetch from RPC at runtime |
| bSOL | BlazeStake Staked SOL | `bSo13r4TkiE4KumL71LsHTPpL2euBYLFx6h9HP3piy1` | SPL Stake Pool | Fetch from RPC at runtime |
| compassSOL | Compass Staked SOL | `Comp4ssDzXcLev2MnLuGNNFC4cmLPMng8qWHPvzAMU1h` | SanctumSpl | Fetch from RPC at runtime |
| hubSOL | SolanaHub Staked SOL | `HUBsveNpjo5pWqNkH57QzxjQASdTVXcSK7bVKTSZtcSX` | SanctumSpl | Fetch from RPC at runtime |
| bonkSOL | BONK Staked SOL | `BonK1YhkXEGLZzwtcvRTip3gAL9nCeQD7ppZBLXhtTs` | SanctumSpl | Fetch from RPC at runtime |
| stakeSOL | Stake City SOL | `st8QujHLPsX3d6HG9uQg9kJ91jFxUgruwsb1hyYXSNd` | SanctumSpl | Fetch from RPC at runtime |
| INF | Sanctum Infinity | `5oVNBeEEQvYi1cX3ir8Dx5n1P7pdxydbGF2X4TxVusJm` | SPool (Infinity) | N/A (covered by sanctum-infinity) |
| wSOL | Wrapped SOL | `So11111111111111111111111111111111111111112` | N/A | N/A |

> **Note on mSOL**: Marinade uses a custom staking program (`MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD`), not the SPL Stake Pool program. The `stake` command should initially support SPL-based LSTs (jitoSOL, jupSOL, bSOL, compassSOL, hubSOL, bonkSOL) and mark mSOL as **not directly stakeable via this plugin** (use the `marinade` plugin instead). The `swap-lst` command works for mSOL via the Sanctum Router.

> **Note on INF**: Excluded from `list-lsts` scope here; covered entirely by `sanctum-infinity` plugin. Include as a swap target only.

### Key Program Addresses

| Program | Address |
|---------|---------|
| SPL Stake Pool Program | `SPoo1Ku8WFXoNDMHPsrGSTSG1Y47rzgn41SLUNakuHy` |
| Sanctum S Controller (SPool) | `5ocnV1qiCgaQR8Jb8xWnVbApfaygJ8tNoZfgPwsgx9kx` |
| Associated Token Program | `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bx8` |
| Token Program | `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA` |
| System Program | `11111111111111111111111111111111` |
| Solana RPC | `https://api.mainnet-beta.solana.com` |

---

## §6 Known Risks / Gotchas

### Critical: base64 → base58 conversion

**The Sanctum Router API (`/v1/swap`) returns the serialized transaction as base64.
`onchainos wallet contract-call --unsigned-tx` requires base58.**

Must convert before passing to onchainos:
```rust
use base64::{engine::general_purpose::STANDARD, Engine};
let bytes = STANDARD.decode(tx_b64.trim())?;
let tx_b58 = bs58::encode(bytes).into_string();
```
Failure to convert causes silent failure or a malformed transaction error.

---

### Critical: `wallet balance --chain 501` — no `--output json`

Solana (chain 501) does **not** support `--output json` flag. The command returns JSON natively:
```
onchainos wallet balance --chain 501        ← CORRECT
onchainos wallet balance --chain 501 --output json  ← WRONG (causes EOF failure)
```
Parse the wallet address from `data.details[0].tokenAssets[0].address` or fall back to `data.address`.

---

### Solana RPC stability

- Use `https://api.mainnet-beta.solana.com` as the primary RPC for stake pool state reads.
- Avoid protocol-specific RPCs (e.g., Jito's own RPC) for general `getAccountInfo` calls — they have partial `jsonParsed` support and can return null.
- Implement at least one retry on 429 / connection errors.

---

### v0 Versioned Transaction format required

Onchainos rejects legacy Solana transactions. All manually constructed transactions must use v0 versioned format:
- Set message version byte to `0x80` (versioned prefix)
- Append `0x00` trailing byte (empty address lookup table list)
- Reference: jito post-mortem — "onchainos rejects legacy-format Solana transactions"

---

### ATA vs non-ATA token accounts

Do NOT create ATA inline in the same transaction for the `stake` operation. The `ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bx8` program is native and fails in simulation, which causes onchainos to reject the transaction. Instead:
1. Pre-check existence via `getTokenAccountsByOwner`
2. If ATA doesn't exist, create it in a **separate** prior transaction
3. Jito post-mortem: "Remove CreateATA instruction; resolve existing token account via getTokenAccountsByOwner instead"

---

### LST staking epoch delay

After a successful `stake`, LST tokens are credited at the **next epoch boundary** (~2-3 days on Solana mainnet). The transaction itself confirms immediately, but `get-position` will show 0 balance until epoch processes. Always warn the user about this delay in the `stake` output.

---

### mSOL: Marinade custom program (not SPL Stake Pool)

mSOL uses Marinade's custom staking program, not SPL Stake Pool. The `stake --lst mSOL` command must return a clear error: "mSOL staking is not supported in this plugin — use the `marinade` plugin instead." mSOL is supported for `swap-lst` and `get-quote` via the Sanctum Router.

---

### Sanctum Router 502 downtime

`sanctum-s-api.fly.dev` runs on Fly.io and has experienced 502 errors (router backend unavailable). Implement retry with backoff (3 attempts, 2s between). Surface clear error: "Sanctum Router API is temporarily unavailable (502). Please try again."

---

### `swapSrc` field validation

The swap quote response includes `swapSrc` which may be `"SPool"` or `"SanctumInfinity"` or others. The `POST /v1/swap` body should include `"swapSrc": "<value_from_quote>"` — do not hardcode `"SPool"` unconditionally. The Router picks the best route; passing the wrong `swapSrc` may cause transaction construction failure.

---

### Slippage and `quotedAmount`

`quotedAmount` in the swap POST body is the **minimum acceptable output** (slippage floor), not the expected output. Calculate as:
```
min_out = floor(quote.outAmount * (1 - slippage_pct / 100))
```
Passing `quotedAmount = outAmount` (no tolerance) will cause swaps to fail on any price movement.

---

### Distinction from `sanctum-infinity`

This plugin does NOT implement:
- INF pool LP `deposit` / `withdraw` operations (those are in `sanctum-infinity`)
- `pools` — Infinity pool allocation (in `sanctum-infinity`)

This plugin adds operations NOT in `sanctum-infinity`:
- `stake` — direct SOL→LST via SPL Stake Pool (Jito-style DepositSol, generalized)
- `list-lsts` — enumerate validator LSTs (not Infinity pool internals)
- `get-position` — multi-LST balance aggregation across tracked validator mints
