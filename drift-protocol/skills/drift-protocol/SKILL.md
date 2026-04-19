---
name: drift-protocol
description: "Drift Protocol perpetual futures DEX and lending on Solana. Read operations: get wallet balance, query perpetual market orderbook, get funding rates. Write operations (place-order, deposit, cancel-order) are paused pending protocol relaunch after 2026-04-01 security incident. Trigger phrases: drift balance, drift markets, drift funding rate, drift perps, drift protocol, drift SOL-PERP. Chinese: 查询Drift余额, Drift永续合约行情, Drift资金费率"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Architecture

- `get-balance` — calls `onchainos wallet balance --chain 501` (no --output json); parses SOL, USDC, USDT balances
- `get-markets` — calls `https://dlob.drift.trade/l2?marketName=...&depth=...`; returns graceful error if 503 (protocol paused)
- `get-funding-rates` — calls `https://data.api.drift.trade/fundingRates?marketName=...`; returns graceful error if unavailable
- `place-order`, `deposit`, `cancel-order` — **PAUSED** stubs; return descriptive error explaining protocol recovery status
- Chain: Solana mainnet (chain ID 501)
- APIs: `https://dlob.drift.trade` (DLOB orderbook), `https://data.api.drift.trade` (funding/historical data)

> **Protocol status:** Drift Protocol is in recovery mode after a $285M exploit on 2026-04-01. The DLOB server currently returns 503. Write operations have no public unsigned-transaction API and are additionally blocked by the protocol pause. Relaunch ETA: post-audit (no date announced). Track: https://drift.trade

## Do NOT use for

- Do NOT use for EVM perpetuals — use `gmx-v2` for Arbitrum perps, `hyperliquid` for Hyperliquid perps
- Do NOT use for Solana spot swaps — use `raydium` or `jupiter` for SPL token swaps
- Do NOT attempt write operations (place-order, deposit, cancel-order) — they will return a paused error

## Commands

### get-balance — Check Solana wallet balances

Returns SOL, USDC, and USDT balances for the active onchainos wallet on Solana. Works regardless of Drift's operational status.

```bash
drift-protocol get-balance
```

Example output:
```json
{
  "ok": true,
  "wallet": "7xKXtg2CW87d97TXJSDpbD5jBkheTqA83TZRuJosgAsU",
  "sol": "1.234",
  "usdc": "50.00",
  "usdt": "0.00",
  "chain": "solana"
}
```

### get-markets — Fetch L2 orderbook for a perpetual market

Fetches bid/ask levels from the Drift DLOB server. Returns a graceful error while the protocol is paused.

```bash
# Default: SOL-PERP, depth 10
drift-protocol get-markets

# Specify market and depth
drift-protocol get-markets --market SOL-PERP --depth 5
drift-protocol get-markets --market BTC-PERP --depth 10
drift-protocol get-markets --market ETH-PERP --depth 5
```

Example output when protocol is live:
```json
{
  "ok": true,
  "market": "SOL-PERP",
  "depth": 5,
  "bids": [{"price": "140.50", "size": "10.0"}],
  "asks": [{"price": "140.55", "size": "8.5"}],
  "slot": 312345678
}
```

Output when protocol is paused (current state):
```json
{
  "ok": false,
  "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Track status: https://drift.trade",
  "note": "Read operations (get-markets, get-funding-rates) will return data when the protocol relaunches."
}
```

### get-funding-rates — Fetch perpetual funding rates

Fetches funding rate data from the Drift data API. Returns a graceful error while the protocol is paused.

```bash
# Default: SOL-PERP
drift-protocol get-funding-rates

# Specific market
drift-protocol get-funding-rates --market SOL-PERP
drift-protocol get-funding-rates --market BTC-PERP
```

### place-order — [PENDING RELAUNCH]

Place a market or limit order on Drift perpetuals. Currently returns a paused error.

```bash
# These commands accept the arguments but always return a paused error
drift-protocol place-order --market SOL-PERP --side buy --size 1.0 --price 140.00
drift-protocol place-order --market SOL-PERP --side sell --size 0.5
```

Current response:
```json
{
  "ok": false,
  "error": "Drift Protocol is currently paused following a security incident on 2026-04-01. Trading will resume after independent security audits complete. Track status: https://drift.trade",
  "note": "When Drift relaunches with a public transaction API, this command will be fully implemented."
}
```

### deposit — [PENDING RELAUNCH]

Deposit USDT or SOL into Drift. Currently returns a paused error.

```bash
drift-protocol deposit --token USDT --amount 100.0
```

### cancel-order — [PENDING RELAUNCH]

Cancel an open order. Currently returns a paused error.

```bash
drift-protocol cancel-order --order-id <ORDER_ID>
```

## Key Constants

| Constant | Value |
|----------|-------|
| Drift Program ID | `dRiftyHA39MWEi3m9aunc5MzRF1JYuBsbn6VPcn33UH` |
| USDT Mint (Solana) | `Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB` |
| USDC Mint (legacy) | `EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v` |
| onchainos chain ID | `501` (Solana mainnet) |

## Notes

- `get-balance` works today regardless of Drift's recovery status — it reads directly from Solana via onchainos
- All three DLOB and data API endpoints are expected to recover at protocol relaunch
- USDT (not USDC) is the settlement token in the relaunched protocol
- Write operations require a public transaction-building API from Drift — none exists today; this is not an onchainos limitation
