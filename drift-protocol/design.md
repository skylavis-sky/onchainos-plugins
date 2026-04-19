# Drift Protocol — Plugin Store 接入 PRD

> Integrate Drift Protocol with onchainos CLI so AI Agents can interact with Drift's perpetual DEX and lending platform on Solana.

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `drift-protocol` |
| dapp_name | Drift Protocol |
| dapp_repo | https://github.com/drift-labs/protocol-v2 |
| dapp_alias | Drift, Drift DEX, Drift perps |
| one_liner | Perpetual futures DEX and lending on Solana with ~$1B+ TVL (pre-exploit) |
| category | defi-protocol |
| tags | perpetuals, solana, lending, spot, orderbook |
| target_chains | Solana (501) |
| target_protocols | Drift Protocol |

---

## §1 Feasibility

> Completed pre-build due-diligence. Integration path determined from research dated 2026-04-19.

### Feasibility Table

| Check Item | Result |
|------------|--------|
| Rust SDK available? | **Yes** — `drift-rs` (alpha) at https://github.com/drift-labs/drift-rs (crates.io: `drift-rs = "1.0.0-alpha.15"`). Requires Solana keypair to sign transactions — not compatible with onchainos headless signing model. |
| SDK tech stack? | TypeScript (`@drift-labs/sdk`), Python (`driftpy`), Rust (`drift-rs` alpha). All SDKs require a local keypair for write operations. |
| REST API available? | **Partial.** Read-only public APIs exist: DLOB server (`dlob.drift.trade` — currently 503, unreliable), Data API (`data.api.drift.trade` — endpoints under development, some 404). Self-hosted gateway (`drift-labs/gateway`) provides full REST but requires running locally with a private key. No public hosted API for write operations. |
| Official Skill? | No. |
| Community Skill? | No known community Drift onchainos/plugin-store skill found. |
| Supported chains? | Solana mainnet only. Program ID: `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH`. |
| onchainos broadcast required? | **Yes** for all write operations (deposit, withdraw, place-order, cancel-order). However, Drift transactions require a locally-held keypair to build the instruction — there is no public API endpoint that returns a pre-built unsigned transaction for onchainos to sign via `--unsigned-tx`. |

### Integration Path Decision

```
Community Skill? No
  → Rust SDK? Yes (drift-rs), BUT requires keypair at build time — not onchainos-compatible for writes.
  → REST API for unsigned tx? No public API returns pre-built unsigned Solana transactions.
  → Self-hosted gateway? Yes, but requires running your own server with a private key embedded.
```

**Integration Path: API (read-only) + BLOCKED for writes**

**Critical Constraint — Drift Exploit (April 1, 2026):**
Drift Protocol was exploited for ~$285M on April 1, 2026 via a social engineering attack on multisig signers. As of April 19, 2026:
- The protocol is in **recovery mode** — trading, deposits, and withdrawals are paused.
- Drift is undergoing two independent security audits (Ottersec + Asymmetric) before relaunch.
- The settlement layer is migrating from USDC to USDT as part of a $147.5M Tether-backed recovery plan.
- No relaunch date has been announced.

**Feasibility verdict:**

| Operation | Feasibility | Reason |
|-----------|-------------|--------|
| `get-markets` | **FEASIBLE** (when API recovers) | DLOB REST API: `GET /l2?marketName=SOL-PERP` |
| `get-balance` | **FEASIBLE** | `onchainos wallet balance --chain 501` for SOL; token accounts for USDT/USDC |
| `get-positions` | **FEASIBLE** (when API recovers) | Gateway `GET /v2/positions` — needs local server, OR Solana RPC account read |
| `deposit` | **BLOCKED** | No public unsigned-tx API; requires keypair to construct Drift deposit instruction |
| `withdraw` | **BLOCKED** | Same blocker as deposit |
| `place-order` | **BLOCKED** | No public unsigned-tx API; Drift Swift API requires signed message from local keypair |
| `cancel-order` | **BLOCKED** | Same blocker as place-order |

**Recommendation:** Implement read-only operations now (get-markets, get-balance). Mark write operations as pending relaunch and pending a publicly-accessible transaction-building API. Revisit after Drift relaunches with its USDT-based system — the new architecture may include a hosted endpoint.

---

## §2 Interface Mapping (Solana Format)

### Operations List

| # | Operation | Type | Feasibility |
|---|-----------|------|-------------|
| 1 | get-markets | Read (off-chain) | Feasible (DLOB REST) |
| 2 | get-balance | Read (on-chain) | Feasible (onchainos wallet) |
| 3 | get-positions | Read (off-chain/on-chain) | Feasible with limitations |
| 4 | deposit | Write (on-chain) | BLOCKED — no unsigned-tx API |
| 5 | withdraw | Write (on-chain) | BLOCKED — no unsigned-tx API |
| 6 | place-order | Write (on-chain) | BLOCKED — no unsigned-tx API |
| 7 | cancel-order | Write (on-chain) | BLOCKED — no unsigned-tx API |

---

### Off-Chain Read Operations

| Operation | API Endpoint | Key Parameters | Return Value |
|-----------|-------------|----------------|--------------|
| get-markets | `GET https://dlob.drift.trade/l2` | `marketName` (e.g. `SOL-PERP`), `depth` (int), `includeVamm=true` | `{ bids: [{price, size}], asks: [{price, size}], slot }` |
| get-markets (L3) | `GET https://dlob.drift.trade/l3` | `marketName`, `includeOracle=true` | Individual order entries with maker addresses |
| get-markets (top makers) | `GET https://dlob.drift.trade/topMakers` | `marketName`, `side` (`bid`/`ask`), `limit` | Top maker addresses and sizes |
| get-funding-rate | `GET https://data.api.drift.trade/fundingRates` | `marketName=SOL-PERP` | Historical funding rate data |
| get-rate-history | `GET https://data.api.drift.trade/rateHistory` | `marketIndex=0` | Rate history for spot market 0 (USDC/USDT) |

**API Status Note:** As of 2026-04-19, `dlob.drift.trade` returns HTTP 503 (server down — protocol in recovery). `data.api.drift.trade` returns 404 on many paths. These will recover when the protocol relaunches.

---

### On-Chain Write Operations (Solana)

| Operation | Program ID | API Endpoint | Request Body Key Fields | Amount Unit | TX Encoding |
|-----------|-----------|-------------|------------------------|-------------|-------------|
| deposit | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` | **NONE — no public endpoint** | N/A | N/A | N/A |
| withdraw | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` | **NONE — no public endpoint** | N/A | N/A | N/A |
| place-order | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` | Swift: `POST https://swift.drift.trade/orders` (requires local keypair to sign order message) | `{ orderMessage, signature, signingAuthority }` | N/A | N/A |
| cancel-order | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` | **NONE — no public endpoint** | N/A | N/A | N/A |

**Swift API Note:** The Swift API at `swift.drift.trade/orders` still requires the user to cryptographically sign an order message with their local keypair before submission. This is not onchainos-compatible — onchainos handles signing internally and does not expose the keypair to the plugin.

**Self-hosted Gateway Note:** The `drift-labs/gateway` (Rust binary) provides a full REST API including `POST /v2/orders`, `DELETE /v2/orders`, `GET /v2/positions`. However, it requires `DRIFT_GATEWAY_KEY` (a Solana keypair) as an environment variable at startup. This is incompatible with the onchainos trust model where plugins never hold private keys.

**Balance check via onchainos:**
```bash
# Check SOL balance
onchainos wallet balance --chain 501
# Parse: json["data"]["details"][0]["tokenAssets"][0]["address"]

# NOTE: Do NOT add --output json for chain 501 (Solana returns JSON natively)
```

---

## §3 User Scenarios

### Scenario 1: Check Available Perpetual Markets on Drift

- **User says:** "What perpetual markets are available on Drift Protocol?"
- **Agent action sequence:**
  1. [Off-chain read] Call `GET https://dlob.drift.trade/l2?marketName=SOL-PERP&depth=5&includeVamm=true` to get SOL-PERP orderbook.
  2. [Off-chain read] Call `GET https://dlob.drift.trade/l2?marketName=BTC-PERP&depth=5` for BTC-PERP.
  3. [Off-chain read] Call `GET https://dlob.drift.trade/l2?marketName=ETH-PERP&depth=5` for ETH-PERP.
  4. Parse each response for best bid/ask prices.
  5. Return formatted table: market name, best bid, best ask, spread.
- **Blockers:** DLOB server currently 503 (protocol in recovery). Will work after relaunch.
- **Fallback if DLOB down:** Return error message explaining Drift is in recovery and link to drift.trade for status.

---

### Scenario 2: Check My Wallet Balance Before Depositing

- **User says:** "Show my SOL and USDT balances before I deposit to Drift."
- **Agent action sequence:**
  1. [onchainos] Run `onchainos wallet balance --chain 501` — do NOT add `--output json`.
  2. Parse JSON response: `json["data"]["details"][0]["tokenAssets"]`.
  3. Filter for SOL (native) and USDT (`Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` on Solana mainnet).
  4. Return formatted balances to user.
  5. [Informational] Note that Drift deposits are currently paused (recovery mode).
- **onchainos command:**
  ```bash
  onchainos wallet balance --chain 501
  ```
- **Notes:** This works today regardless of Drift's operational status. USDT mint on Solana: `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB`. USDC mint (legacy, pre-exploit): `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v`.

---

### Scenario 3: Check Drift Funding Rates for SOL-PERP

- **User says:** "What is the current funding rate for SOL perpetuals on Drift?"
- **Agent action sequence:**
  1. [Off-chain read] Call `GET https://data.api.drift.trade/fundingRates?marketName=SOL-PERP`.
  2. Parse the response for current funding rate, 1h rate, and annualized rate.
  3. Also call `GET https://dlob.drift.trade/l2?marketName=SOL-PERP&depth=1&includeOracle=true` to get oracle price and mark price.
  4. Calculate funding rate premium (mark vs oracle).
  5. Return: funding rate %, direction (longs pay / shorts pay), oracle price, mark price.
- **Blockers:** Both endpoints currently unreachable (protocol in recovery).
- **Fallback:** Explain Drift is paused post-exploit, direct user to drift.trade for status updates.

---

### Scenario 4: Place a Limit Order on Drift (BLOCKED — documented for future)

- **User says:** "Place a limit order to buy 1 SOL-PERP at $140 on Drift."
- **Why this is blocked:**
  1. Drift requires transactions signed by the user's local keypair.
  2. No public endpoint returns a pre-built unsigned transaction (base64/base58).
  3. onchainos `--unsigned-tx` requires a fully serialized, unsigned Solana transaction from an external source — plugins cannot construct this without the keypair.
  4. The self-hosted `drift-labs/gateway` could serve this role but requires embedding a keypair in the local server, violating onchainos trust model.
- **Future path:** If Drift relaunches with a public transaction-building API (similar to Jupiter's swap API that returns an unsigned transaction), this operation becomes feasible with the standard base64→base58 conversion pattern:
  ```bash
  onchainos wallet contract-call --chain 501 \
    --to dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH \
    --unsigned-tx <base58_encoded_tx>
  ```
- **Pattern to implement when API is available:**
  ```rust
  // Cargo.toml needs: base64 = "0.22" and bs58 = "0.5"
  let tx_bytes = base64::decode(&tx_data_from_api)?;
  let tx_b58 = bs58::encode(tx_bytes).into_string();
  // Then pass tx_b58 to onchainos --unsigned-tx
  ```

---

## §4 External API Dependencies

| API | Base URL | Purpose | Auth Required? |
|-----|----------|---------|----------------|
| Drift DLOB Server | `https://dlob.drift.trade` | Orderbook data (L2/L3), top makers, market prices | No — public |
| Drift DLOB WebSocket | `wss://dlob.drift.trade/ws` | Real-time orderbook and trade streams | No — public |
| Drift Data API | `https://data.api.drift.trade` | Funding rates, rate history, auction params, historical data | No — public |
| Drift Swift API | `https://swift.drift.trade/orders` | Off-chain order submission (write) | Yes — requires local keypair signature |
| Solana RPC (mainnet) | `https://api.mainnet-beta.solana.com` | On-chain account reads (user positions, balances) | No (rate-limited) |

**API Health Status (as of 2026-04-19):**
- `dlob.drift.trade` — HTTP 503, server down (protocol in recovery mode)
- `data.api.drift.trade/fundingRates` — HTTP 404 (endpoint may have changed)
- `swift.drift.trade` — Status unknown (not tested)

---

## §5 Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `default_chain` | `501` | Solana mainnet chain ID in onchainos |
| `default_market` | `SOL-PERP` | Default perpetual market for queries |
| `orderbook_depth` | `10` | Number of orderbook levels to fetch |
| `dry_run` | `true` | Simulate write operations without broadcasting |
| `dlob_url` | `https://dlob.drift.trade` | DLOB server base URL (may change post-relaunch) |
| `data_api_url` | `https://data.api.drift.trade` | Data API base URL |
| `settlement_token` | `USDT` | Settlement token (migrating from USDC to USDT post-relaunch) |

---

## §6 Key Constants

| Constant | Value | Notes |
|----------|-------|-------|
| Drift Program ID | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` | Same on mainnet and devnet |
| Drift Vaults Program | `JCNCMFXo5M5qwUPg2Utu1u6YWp3MbygxqBsBeXXJfrw` | Main vault authority |
| USDC Spot Market Index | `0` | Legacy — being replaced by USDT post-relaunch |
| USDT Mint (Solana) | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` | New settlement token |
| USDC Mint (Solana) | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` | Legacy, may be deprecated post-relaunch |
| User Account Init Cost | ~0.035 SOL | One-time Solana rent for `initializeUserAccount` |
| onchainos Chain ID | `501` | Do NOT add `--output json` to `wallet balance --chain 501` |

---

## §7 Open Questions

- [ ] **Relaunch timeline**: When will Drift Protocol resume trading? Two independent audits (Ottersec + Asymmetric) must complete first.
- [ ] **New API surface**: Will the USDT-based Drift v3 relaunch include a public transaction-building API similar to Jupiter's? This is the key blocker for write operations.
- [ ] **USDT market index**: What will the USDT spot market index be in the relaunched protocol? (USDC was market index 0.)
- [ ] **DLOB URL**: Will `dlob.drift.trade` remain the canonical DLOB endpoint post-relaunch?
- [ ] **Account initialization**: With the new security architecture, will `initializeUserAccount` flow change?
- [ ] **Data API authentication**: Will `data.api.drift.trade` add API key requirements post-relaunch?

---

## §8 Implementation Plan (Phased)

### Phase A — Implement Now (Read-Only, Protocol-Agnostic)
These work regardless of Drift's recovery status:

1. **`get-balance`** — `onchainos wallet balance --chain 501`, parse USDT/SOL balances.
2. **`get-markets`** (with graceful degradation) — Attempt `dlob.drift.trade/l2`, return meaningful error if 503.
3. **`get-funding-rates`** — Attempt `data.api.drift.trade/fundingRates`, return meaningful error if unavailable.

### Phase B — Implement After Relaunch (Write Operations)
Implement once Drift Protocol relaunches and provides a public transaction-building API:

4. **`deposit`** — Construct USDT transfer to Drift vault via unsigned Solana tx.
5. **`withdraw`** — Reverse of deposit.
6. **`place-order`** — If Swift API or similar returns an unsigned tx, use base64→base58 pattern.
7. **`cancel-order`** — Cancel by order ID using unsigned tx.

### Dependency Chain for Writes
```
Drift relaunch (post-audit) 
  → Public transaction-building API (if provided)
  → base64→base58 conversion pattern (already solved in Batch 4)
  → onchainos wallet contract-call --unsigned-tx
```

---

## §9 Research Sources

- Drift Protocol v2 Teacher docs: https://drift-labs.github.io/v2-teacher/
- Drift Developer docs: https://docs.drift.trade/developers
- Drift SDK (TypeScript): https://drift-labs.github.io/protocol-v2/sdk/
- drift-rs (Rust): https://docs.rs/drift-rs/latest/drift_rs/
- drift-labs/gateway (self-hosted REST): https://github.com/drift-labs/gateway
- drift-labs/dlob-server: https://github.com/drift-labs/dlob-server
- Drift Program Vault Addresses: https://docs.drift.trade/about-v2/program-vault-addresses
- Incident Recovery Update (2026-04-16): https://www.drift.trade/updates/incident-recovery-update-april-16-2026-now
- Exploit coverage (CoinDesk): https://www.coindesk.com/business/2026/04/16/drift-gets-usd148-million-funding-from-tether-and-partners-as-it-replaces-circle-stablecoin-with-usdt-after-massive-exploit
