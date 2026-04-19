# L0 Routing Validation — dydx-v4

**Date:** 2026-04-19  
**Plugin version:** 0.1.0  
**SKILL.md source:** `skills/dydx-v4/SKILL.md`

---

## Routing Table (from SKILL.md)

| User Intent | Command | Mechanism |
|-------------|---------|-----------|
| List perpetual markets | `dydx-v4 get-markets` | Indexer REST GET /v4/perpetualMarkets |
| Show orderbook for a market | `dydx-v4 get-orderbook --market ETH-USD` | Indexer REST GET /v4/orderbooks/perpetualMarket/{market} |
| View open positions | `dydx-v4 get-positions --address dydx1...` | Indexer REST GET /v4/perpetualPositions?address=...&subaccountNumber=0&status=OPEN |
| Check account equity / free collateral | `dydx-v4 get-balance --address dydx1...` | Indexer REST GET /v4/addresses/{addr}/subaccountNumber/0 |
| Bridge DYDX tokens from Ethereum | `dydx-v4 deposit --amount 100 --dydx-address dydx1... --dry-run` | EVM contract-call on chain 1 — bridge(uint256,bytes,bytes) @ 0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9 |
| Place a limit/market order | `dydx-v4 place-order --market BTC-USD --side buy --size 0.1 --price 70000` | Informational stub only (Cosmos gRPC not supported) |

---

## Routing Decisions

### Read commands (no wallet required)
- `get-markets`, `get-orderbook`, `get-positions`, `get-balance` → all use dYdX Indexer REST API
- No authentication needed
- `get-positions` and `get-balance` require `--address` parameter; if omitted, should return helpful guidance

### Write command — deposit
- Routes to `onchainos wallet contract-call` on chain 1
- Contract: `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` (wethDYDX)
- Function selector: `0x1d45e29c` = `bridge(uint256,bytes,bytes)`
- DYDX has 18 decimals (amount * 1e18)
- Requires `--dry-run` first; SKILL.md mandates user confirmation before live broadcast

### Stub command — place-order
- Always returns informational JSON with `ok: false`
- No transaction broadcast; directs user to dYdX web app or TypeScript SDK

---

## Safety Properties Validated

| Property | Status |
|----------|--------|
| deposit has `--dry-run` gate | PASS (documented in SKILL.md) |
| deposit requires explicit user confirmation before live | PASS (documented) |
| place-order explicitly non-executable | PASS (informational stub) |
| No wallet needed for read commands | PASS |
| USDC deposit out of scope (documented) | PASS |
| No EVM perps — gmx-v2 redirect documented | PASS |

---

## Potential Routing Gaps

| Gap | Severity |
|-----|----------|
| SKILL.md deposit dry-run example shows `--dry-run` but live example omits `--confirm` (says to "re-run without --dry-run") | Minor — could cause confusion; SKILL.md live flow note says "remove --dry-run and add --confirm" but the step-by-step text says "re-run without --dry-run"; needs to be reconciled with actual binary behavior |
| No `--chain` default documented in command routing table (only in parameters section) | Minor |

---

## Verdict: PASS (with Minor notes)
