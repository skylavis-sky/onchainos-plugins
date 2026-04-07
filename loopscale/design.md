# Loopscale — Plugin Design

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `loopscale` |
| `dapp_name` | Loopscale |
| `version` | 0.1.0 |
| `target_chains` | Solana (chain 501) |
| `category` | defi-protocol |
| `description` | Order-book credit matching on Solana — lenders post offers, borrowers fill them at fixed rates with any tokenized collateral |
| `integration_path` | REST API (`https://tars.loopscale.com`) |

---

## §1 Feasibility

### Integration path: Loopscale Partner REST API

**Base URL:** `https://tars.loopscale.com`

**Authentication:**
- All confirmed endpoints use `security: []` — **no API key required** for the public-facing endpoints documented.
- Some transaction-building endpoints require a `user-wallet` or `payer` header (the caller's Solana public key), used only to customize the unsigned transaction. This is not an authentication secret — it is publicly-known data.
- The documentation notes this is "a curated set of endpoints shared with partners and integrators on an as-needed basis" and that a full public API reference is "in progress." Developers needing additional endpoints should contact `developers@loopscale.com`.

**Verdict: FEASIBLE.** No API key needed for the operations mapped below. The REST API is live and returning real data as of April 2026.

### Protocol Status Note

Loopscale suffered a $5.8M exploit on April 26, 2025 (~16 days after launch) due to a RateX PT token oracle pricing vulnerability. The attacker returned funds (10% bounty), a third-party audit (Sec3 + independent) was completed, vault withdrawals were re-enabled, and all protocol functions were fully restored. All new features now require third-party audit before deployment.

**Current status: Fully operational.** TVL exceeded $100M at peak; API returning live data confirmed.

---

## §2 Interface Mapping

### APY / Rate Unit Note

The API expresses rates in **cBPS (centi-basis-points)**:
- `1 cBPS = 0.0001%`
- `10000 cBPS = 1% APY`
- `100000 cBPS = 10% APY`
- Divide `apy_cbps / 1_000_000.0` to get a decimal (e.g., `100000 / 1_000_000.0 = 0.10 = 10%`)

### Transaction Encoding Note

All Loopscale transaction endpoints return **Base64-encoded versioned Solana transactions** in the `transaction.message` field (or `transactions[].message` for multi-tx responses). Per the Solana/onchainos pattern (see KNOWLEDGE_HUB.md §87), these must be **converted from base64 to base58** before passing to `onchainos wallet contract-call --unsigned-tx`.

```rust
fn base64_to_base58(b64: &str) -> anyhow::Result<String> {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let bytes = STANDARD.decode(b64.trim())?;
    Ok(bs58::encode(bytes).into_string())
}
```

### Off-chain Read Operations

| Operation | Method | Endpoint | Key Parameters | Response Fields |
|-----------|--------|----------|----------------|-----------------|
| `get-markets` | POST | `https://tars.loopscale.com/v1/markets/lending_vaults/deposits` | `principalMints` (optional array of mint addresses) | `vaultAddress`, `userDeposits[].amountSupplied` |
| `get-quotes` | POST | `https://tars.loopscale.com/v1/markets/quote` | `principal` (mint), `collateral` (array of mint strings), `durationType`, `duration`, `limit`, `offset` | `apy` (cBPS), `ltv` (cBPS), `liquidationThreshold` (cBPS), `maxPrincipalAvailable`, `sumPrincipalAvailable` |
| `get-best-quote` | POST | `https://tars.loopscale.com/v1/markets/quote/max` | Header: `user-wallet`; Body: `principalMint`, `collateralFilter[]{amount, assetData{Spl{mint}}}`, `durationType`, `duration` | `apy`, `strategy`, `collateralIdentifier`, `ltv`, `lqt`, `amount` |
| `get-position` | POST | `https://tars.loopscale.com/v1/markets/loans/info` | `borrowers` (array of wallet pubkeys), `filterType` (0=Active), `pageSize`, `page` | `loanInfos[]{loan, ledgers[], collateral[]}` |
| `get-loan-info` | POST | `https://tars.loopscale.com/v1/markets/loans/info` | `loanAddresses`, `filterType`, `sortType` | `loanInfos[]{loan.address, ledgers[].apy, ledgers[].principalDue, collateral[].amount, collateral[].assetMint}` |
| `get-vault-depositors` | POST | `https://tars.loopscale.com/v1/markets/lending_vaults/deposits` | `vaultAddresses` OR `principalMints` | `vaultAddress`, `userDeposits[]{userAddress, amountSupplied}` |
| `get-collateral-holders` | POST | `https://tars.loopscale.com/v1/markets/collateral/holders` | `mints` (array), `pdas` (bool) | `collateralMint`, `totalDeposits`, `userDeposits{}` |

#### `get-position` — Filter by borrower wallet:

```json
POST https://tars.loopscale.com/v1/markets/loans/info
{
  "borrowers": ["<USER_WALLET_PUBKEY>"],
  "filterType": 0,
  "pageSize": 50,
  "page": 1
}
```

Response fields per loan:
- `loan.address` — on-chain loan PDA
- `loan.borrower` — borrower pubkey
- `loan.loanStatus` — 0=Active
- `ledgers[0].principalMint` — token being borrowed
- `ledgers[0].principalDue` — total principal owed (lamports)
- `ledgers[0].principalRepaid` — amount repaid so far
- `ledgers[0].apy` — rate in cBPS
- `ledgers[0].endTime` — Unix timestamp of loan maturity
- `ledgers[0].strategy` — lender strategy address
- `collateral[0].assetMint` — collateral token mint
- `collateral[0].amount` — collateral amount

### Solana Write Operations

| Operation | API Endpoint | Method | Amount Unit | TX Encoding | Key Headers |
|-----------|-------------|--------|-------------|-------------|-------------|
| `lend` (vault deposit) | `/v1/markets/lending_vaults/deposit` | POST | lamports | base64 → base58 | `user-wallet` |
| `withdraw` (vault withdraw) | `/v1/markets/lending_vaults/withdraw` | POST | lamports | base64 → base58 | `user-wallet` |
| `borrow` (create loan) | `/v1/markets/creditbook/create` | POST | lamports | base64 → base58 | `payer` |
| `borrow-principal` | `/v1/markets/creditbook/borrow` | POST | lamports | base64 → base58 | `payer` |
| `repay` | `/v1/markets/creditbook/repay` | POST | lamports | base64 → base58 | — |

#### `lend` — Deposit into a Loopscale Vault

```
POST https://tars.loopscale.com/v1/markets/lending_vaults/deposit
Header: user-wallet: <WALLET_PUBKEY>
Body:
{
  "principalAmount": <amount_in_lamports>,
  "minLpAmount": 0,
  "vault": "<VAULT_ADDRESS>"
}
Response:
{
  "transaction": {
    "message": "<BASE64_TX>",
    "signatures": [{"publicKey": "...", "signature": "..."}]
  },
  "stakeAccount": "<LP_STAKE_ACCOUNT_OR_NULL>"
}
```

onchainos call:
```bash
onchainos wallet contract-call \
  --chain 501 \
  --to <VAULT_ADDRESS> \
  --unsigned-tx <BASE58_CONVERTED_TX> \
  --force
```

#### `withdraw` — Withdraw from a Loopscale Vault

```
POST https://tars.loopscale.com/v1/markets/lending_vaults/withdraw
Header: user-wallet: <WALLET_PUBKEY>
Body:
{
  "amountPrincipal": <amount_in_lamports>,
  "maxAmountLp": <large_number_or_0>,
  "vault": "<VAULT_ADDRESS>",
  "withdrawAll": false
}
Response: { "transaction": { "message": "<BASE64_TX>", ... } }
```

#### `borrow` — Create a Loan (fill a lend order)

Two-step process:
1. Call `/v1/markets/creditbook/create` to initialize a loan PDA and deposit collateral.
2. Call `/v1/markets/creditbook/borrow` to draw down the principal.

**Step 1: Create Loan**
```
POST https://tars.loopscale.com/v1/markets/creditbook/create
Header: payer: <WALLET_PUBKEY>
Body:
{
  "depositCollateral": [
    {
      "amount": <collateral_amount_lamports>,
      "assetData": { "Spl": { "mint": "<COLLATERAL_MINT>" } }
    }
  ],
  "borrower": "<WALLET_PUBKEY>",
  "principalRequested": [
    {
      "ledger": <ledger_index>,
      "amount": <principal_amount_lamports>,
      "mint": "<PRINCIPAL_MINT>",
      "strategy": "<STRATEGY_ADDRESS>",
      "duration": <duration_value>,
      "durationType": <0|1|2|3|4>
    }
  ]
}
Response:
{
  "transaction": { "message": "<BASE64_TX>", ... },
  "loanAddress": "<LOAN_PDA>"
}
```

**Step 2: Borrow Principal** (if not combined)
```
POST https://tars.loopscale.com/v1/markets/creditbook/borrow
Header: payer: <WALLET_PUBKEY>
Body:
{
  "loan": "<LOAN_PDA>",
  "borrowParams": {
    "amount": <principal_amount_lamports>,
    "duration": { "duration": <value>, "durationType": <enum> },
    "expectedLoanValues": { "apy": <expected_apy_cbps> }
  },
  "strategy": "<STRATEGY_ADDRESS>"
}
Response: { "transaction": { "message": "<BASE64_TX>", ... } }
```

#### `repay` — Repay a Loan

```
POST https://tars.loopscale.com/v1/markets/creditbook/repay
Body:
{
  "loan": "<LOAN_ADDRESS>",
  "repayParams": [
    {
      "amount": <repay_amount_lamports>,
      "ledgerIndex": 0,
      "repayAll": true
    }
  ],
  "collateralWithdrawalParams": [
    {
      "amount": <collateral_to_withdraw_lamports>,
      "mint": "<COLLATERAL_MINT>"
    }
  ],
  "closeIfPossible": true
}
Response:
{
  "transactions": [
    { "message": "<BASE64_TX>", "signatures": [...] }
  ],
  "expectedLoanInfo": { ... }
}
```

Note: `repay` may return **multiple transactions** (array). Each must be base64 → base58 converted and submitted sequentially.

---

## §3 User Scenarios

### Scenario 1: View available lending markets and rates

**User:** "Show me available Loopscale vaults and their deposit APYs"

**Agent action sequence:**
1. Call `./loopscale get-markets --chain 501`
2. Binary calls `POST /v1/markets/lending_vaults/deposits` with no filter (or filtered by `principalMints` for USDC/SOL)
3. For each vault, compute TVL from `sum(userDeposits[].amountSupplied)`
4. Optionally call `POST /v1/markets/quote/max` to fetch estimated borrow rates as a proxy for lend APY
5. Return: `{ vaults: [{ vault_address, principal_token, tvl_ui, estimated_apy_pct }] }`

### Scenario 2: View user's active borrow/lend positions

**User:** "Show me my Loopscale positions"

**Agent action sequence:**
1. Call `./loopscale get-position --chain 501`
2. Binary calls `onchainos wallet balance --chain 501` (no `--output json`) to resolve Solana wallet pubkey
3. Calls `POST /v1/markets/loans/info` with `{ "borrowers": [wallet], "filterType": 0, "pageSize": 50, "page": 1 }`
4. Calls `POST /v1/markets/lending_vaults/deposits` with `{ "principalMints": [...] }` and filters by user address in `userDeposits`
5. Returns: active loans with principal owed, collateral posted, APY, maturity date; plus vault deposits with amount supplied

### Scenario 3: Lend USDC to earn yield (vault deposit)

**User:** "Deposit 10 USDC into Loopscale to earn yield"

**Agent action sequence:**
1. Call `./loopscale lend --token USDC --amount 10 --chain 501 --dry-run` (preview)
2. Show estimated APY from `get-markets`, ask user to confirm
3. User confirms → call `./loopscale lend --token USDC --amount 10 --chain 501`
4. Binary resolves wallet pubkey
5. Calls `POST /v1/markets/lending_vaults/deposit` with `{ "principalAmount": 10000000, "minLpAmount": 0, "vault": "AXanCP4dJHtWd7zY4X7nwxN5t5Gysfy2uG3XTxSmXdaB" }` (USDC vault)
6. API returns `{ "transaction": { "message": "<BASE64>" } }`
7. Binary converts base64 → base58
8. Calls `onchainos wallet contract-call --chain 501 --to AXanCP4dJHtWd7zY4X7nwxN5t5Gysfy2uG3XTxSmXdaB --unsigned-tx <BASE58> --force`
9. Returns txHash with solscan link

### Scenario 4: Borrow USDC against SOL collateral

**User:** "Borrow 50 USDC against 1 SOL collateral for 7 days on Loopscale"

**Agent action sequence:**
1. Call `./loopscale get-quotes --principal USDC --collateral SOL --duration 7 --chain 501` to find best strategy/APY
2. Show estimated rate to user (e.g., 8.5% APY), ask to confirm
3. User confirms → call `./loopscale borrow --principal USDC --amount 50 --collateral SOL --collateral-amount 1 --duration 7 --chain 501`
4. Binary calls `POST /v1/markets/creditbook/create` with collateral deposit + principal request
5. Converts base64 → base58; broadcasts via onchainos
6. Calls `POST /v1/markets/creditbook/borrow` using the returned `loanAddress`
7. Broadcasts second tx
8. Returns loan address, borrowed amount, APY, maturity date

### Scenario 5: Repay a loan

**User:** "Repay my Loopscale loan at address <LOAN_ADDR>"

**Agent action sequence:**
1. Call `./loopscale get-position --chain 501` to find active loans and outstanding amounts
2. Show outstanding principal + accrued interest to user, ask to confirm
3. User confirms → call `./loopscale repay --loan <LOAN_ADDR> --chain 501`
4. Binary calls `POST /v1/markets/creditbook/repay` with `repayAll: true`, `closeIfPossible: true`
5. Response may contain multiple transactions — each is converted and broadcast sequentially
6. Returns txHash(es) and "loan closed" status

### Scenario 6: Withdraw from vault

**User:** "Withdraw my USDC from Loopscale"

**Agent action sequence:**
1. Call `get-position` to find vault deposit amounts
2. Show amount available, note instant withdrawal if liquidity buffer has capacity; otherwise a small fee applies
3. User confirms → call `./loopscale withdraw --token USDC --amount <AMT> --chain 501`
4. Binary calls `POST /v1/markets/lending_vaults/withdraw` with `withdrawAll: true`
5. Converts base64 → base58; broadcasts via onchainos
6. Returns txHash

---

## §4 External API Dependencies

| API | Purpose | Authentication | Notes |
|-----|---------|----------------|-------|
| `https://tars.loopscale.com` | All operations — quote, loan creation, vault deposit/withdraw, positions | None (no API key) | `user-wallet` header (pubkey) required for some TX-building endpoints |
| `https://api.mainnet-beta.solana.com` | Resolve wallet pubkey via `onchainos wallet balance --chain 501` | None | Standard Solana RPC |

### Confirmed Endpoints (live as of April 2026)

| Endpoint | Method | Purpose |
|----------|--------|---------|
| `/v1/markets/lending_vaults/deposits` | POST | List vaults + TVL; check user deposit |
| `/v1/markets/quote` | POST | List all available borrow quotes |
| `/v1/markets/quote/max` | POST | Get single best quote for collateral |
| `/v1/markets/loans/info` | POST | User active loans, loan details |
| `/v1/markets/collateral/holders` | POST | Collateral depositor breakdown |
| `/v1/markets/lending_vaults/deposit` | POST | Build vault deposit TX |
| `/v1/markets/lending_vaults/withdraw` | POST | Build vault withdrawal TX |
| `/v1/markets/creditbook/create` | POST | Build loan creation TX |
| `/v1/markets/creditbook/borrow` | POST | Build borrow principal TX |
| `/v1/markets/creditbook/repay` | POST | Build loan repayment TX |
| `/v1/markets/strategy/create` | POST | Build lend strategy (advanced lend) TX |

---

## §5 Config Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `--chain` | u64 | 501 | Solana mainnet |
| `--amount` | f64 | — | Amount in UI units (e.g., 10.0 USDC), converted to lamports internally |
| `--token` | String | — | Token symbol (USDC, SOL) or mint address |
| `--vault` | String | — | Vault address; defaults to largest USDC or SOL vault |
| `--collateral` | String | — | Collateral token symbol/mint |
| `--collateral-amount` | f64 | — | Collateral amount in UI units |
| `--duration` | u64 | 7 | Loan duration value |
| `--duration-type` | u8 | 0 | 0=days, 1=weeks, 2=months, 3=minutes, 4=years |
| `--loan` | String | — | Loan PDA address (for repay/get-loan-info) |
| `--dry-run` | bool | false | Build and validate TX without broadcasting |

### Known Vault Addresses (Mainnet, confirmed April 2026)

| Token | Vault Address | TVL (approx) | Notes |
|-------|--------------|--------------|-------|
| USDC | `AXanCP4dJHtWd7zY4X7nwxN5t5Gysfy2uG3XTxSmXdaB` | ~$14.8M | Largest USDC vault (3610 depositors) |
| USDC | `7PeYxZpM2dpc4RRDQovexMJ6tkSVLWtRN4mbNywsU3e6` | ~$23.2M | Second major USDC vault (1664 depositors) |
| SOL | `U1h9yhtpZgZsgVzMZe1iSpa6DSTBkSH89Egt59MXRYe` | ~65,667 SOL | Largest SOL vault (3232 depositors) |

USDC mint: `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`
SOL (wSOL) mint: `So11111111111111111111111111111111111111112`

### Supported Collateral Asset Types (from API schema)

The `assetData` field in borrow requests supports:
- `Spl` — Standard SPL tokens: `{ "Spl": { "mint": "<MINT>" } }`
- `StakedSol` — Staked SOL: `{ "StakedSol": { "stakeAccount": "...", "stakePool": "..." } }`
- `Orca` — Orca CLMM LP positions: `{ "Orca": { "positionMint": "...", "whirlpool": "...", "tokenProgram": "..." } }`
- `Meteora` — Meteora DLMM positions: `{ "Meteora": { "positionAddress": "...", "lbPair": "...", "tokenProgram": "..." } }`
- `Raydium` — Raydium LP: `{ "Raydium": { "mint": "...", "pool": "...", "tokenProgram": "..." } }`

Commonly used collateral tokens include: JitoSOL (`J1toso1uCk3RLmjorhTtrVwY9HJ7X8V9yYac6Y7kGCPn`), mSOL (`mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So`), and various DeFi LP tokens.

---

## §6 Known Risks / Gotchas

### Solana-Specific

1. **base64 → base58 conversion required.** All Loopscale transaction endpoints return base64-encoded transactions. `onchainos wallet contract-call --unsigned-tx` expects base58. Must decode base64 bytes then re-encode as base58 (do NOT do base64→base64 or skip the step).

2. **No `--output json` on chain 501.** `onchainos wallet balance --chain 501 --output json` fails with EOF. Omit `--output json` and parse wallet address from the plain-text output, specifically from `data.details[0].tokenAssets[0].address`.

3. **Multi-transaction responses.** The `repay` endpoint returns `{ "transactions": [...] }` (an array). Each transaction must be submitted sequentially. Do not use the first tx only.

4. **Amount units are lamports.** Unlike Kamino (which uses UI units), Loopscale transaction APIs use **lamports** for `principalAmount`, `amountPrincipal`, etc. Convert UI amounts: USDC (6 decimals): `amount * 1_000_000`; SOL (9 decimals): `amount * 1_000_000_000`.

5. **cBPS rate unit.** APY is expressed in centi-basis-points (cBPS). Convert to percentage: `apy_cbps / 1_000_000.0 * 100` to get APY%. For display: `100000 cBPS = 10% APY`.

6. **Borrow is two-step.** Creating a loan (`/creditbook/create`) and drawing down principal (`/creditbook/borrow`) are separate API calls producing separate transactions that must be broadcast in order. The `loanAddress` from the first response is required for the second call.

7. **Vault withdrawal liquidity.** Instant withdrawals are only available if the vault's liquidity buffer has capacity. Otherwise, users must pay an early-exit fee or queue. Implement graceful messaging for this case.

8. **Loan `filterType` enum.** When querying positions: 0=Active, 1=Closed, 2=Refinance Eligible, 3=Time-based Liquidation Eligible. Use 0 for active loans.

9. **Strategy vs. Vault.** Loopscale has two lending paths: (a) **Vaults** — passive, curator-managed, users deposit principal and earn yield automatically; (b) **Strategies** — advanced lend mode, lenders create strategies with custom terms. The plugin should primarily expose the Vault path for simplicity (`lend`/`withdraw`). Advanced strategy creation is a separate, complex flow.

10. **Protocol Security:** Post-exploit (April 2025), all market/vault/oracle parameter changes require multisig approval and new features require third-party audit. The protocol was fully restored with funds recovered. No ongoing operational risk identified beyond normal DeFi smart contract risk.

11. **reqwest proxy pattern.** As with other Solana plugins using reqwest in onchainos sandbox, must use `build_client()` helper that reads `HTTPS_PROXY` env var to route through sandbox proxy.

### API Availability Note

The API documentation (`docs.loopscale.com`) states the current API reference covers "a curated set of endpoints shared with partners and integrators on an as-needed basis" — a full public spec is "in progress." All endpoints listed above have been verified against the live API (`tars.loopscale.com`). For additional endpoints or advanced features, contact `developers@loopscale.com`.
