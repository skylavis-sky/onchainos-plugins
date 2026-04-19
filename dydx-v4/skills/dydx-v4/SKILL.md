---
name: dydx-v4
description: "dYdX V4 perpetuals DEX on a Cosmos appchain — query markets, view positions and account balances, bridge DYDX tokens from Ethereum. Trigger phrases: dYdX markets, dYdX positions, dYdX balance, dydx orderbook, bridge DYDX tokens, dydx perps. Do NOT use for EVM perpetuals — use gmx-v2 for Arbitrum/Avalanche perps."
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

# dYdX V4 Skill

## Overview

dYdX V4 is the largest on-chain perpetuals exchange by open interest, operating as a sovereign
Cosmos appchain (dydx-mainnet-1). It hosts 100+ perpetual markets with an off-chain orderbook
and on-chain settlement.

**Read operations** (get-markets, get-orderbook, get-positions, get-balance) use the dYdX
Indexer REST API directly — no wallet needed.

**Bridge deposit** (deposit) — after user confirmation, calls `bridge(uint256,bytes,bytes)` on
the wethDYDX contract on Ethereum mainnet via `onchainos wallet contract-call`.

**Order placement** (place-order) is informational only — it requires Cosmos gRPC
(MsgPlaceOrder), which onchainos does not support. The plugin returns the required parameters
and directs the user to the dYdX web app or TypeScript SDK.

**Do NOT use for EVM perpetuals** — use gmx-v2 for Arbitrum/Avalanche perps.

---

## Architecture

| Operation | Mechanism |
|-----------|-----------|
| get-markets, get-orderbook, get-positions, get-balance | Indexer REST (read-only, no wallet) |
| deposit | after user confirmation, submits via `onchainos wallet contract-call` on Ethereum mainnet (chain 1) |
| place-order | Informational only — Cosmos gRPC required, not executable via onchainos |

---

## Pre-flight Checks

Before any command, verify:

1. **Binary installed**: `dydx-v4 --version`
2. For deposit only: **Wallet connected**: `onchainos wallet status`

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| List perpetual markets | `dydx-v4 get-markets` |
| Show orderbook for a market | `dydx-v4 get-orderbook --market ETH-USD` |
| View open positions | `dydx-v4 get-positions --address dydx1...` |
| Check account equity / free collateral | `dydx-v4 get-balance --address dydx1...` |
| Bridge DYDX tokens from Ethereum | `dydx-v4 deposit --amount 100 --dydx-address dydx1... --dry-run` |
| Place a limit/market order | `dydx-v4 place-order --market BTC-USD --side buy --size 0.1 --price 70000` |

---

## Execution Flow for deposit

IMPORTANT: Always dry-run first, then ask user to confirm before broadcasting.

1. Run with `--dry-run` to preview calldata and parameters
2. Show the user the calldata, amount, and destination address
3. Ask user to confirm
4. After explicit user confirmation, re-run without `--dry-run`
5. Report the transaction hash

The `deposit` command — after user confirmation, submits via `onchainos wallet contract-call` to
the wethDYDX contract (`0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9`) on Ethereum mainnet (chain 1).
Always dry-run first, show output to the user, and wait for explicit user confirmation before
proceeding with the on-chain transaction.

---

## Commands

### get-markets — List all perpetual markets

**Trigger phrases:** "dYdX markets", "dYdX perps list", "available markets on dYdX"

**Usage:**
```
dydx-v4 get-markets
```

**What it does:** GET https://indexer.dydx.trade/v4/perpetualMarkets
Read-only. No wallet needed.

**Expected output:**
```json
{
  "ok": true,
  "count": 120,
  "markets": [
    {
      "ticker": "BTC-USD",
      "status": "ACTIVE",
      "indexPrice": "70000.0",
      "24hVolume": "500000000",
      "openInterest": "2500"
    }
  ]
}
```

---

### get-orderbook — L2 orderbook for a market

**Trigger phrases:** "dYdX orderbook", "dYdX bids asks", "BTC-USD orderbook on dYdX"

**Usage:**
```
dydx-v4 get-orderbook --market BTC-USD
dydx-v4 get-orderbook --market ETH-USD
```

**Key parameters:**
- `--market` — ticker (e.g. BTC-USD, ETH-USD); default: BTC-USD

**What it does:** GET https://indexer.dydx.trade/v4/orderbooks/perpetualMarket/{market}
Read-only. No wallet needed.

**Expected output:**
```json
{
  "ok": true,
  "market": "BTC-USD",
  "bids": [{"price": "69990", "size": "0.5"}],
  "asks": [{"price": "70010", "size": "0.3"}]
}
```

---

### get-positions — Open positions for a dYdX address

**Trigger phrases:** "my dYdX positions", "dYdX open positions", "what am I long on dYdX"

**Usage:**
```
dydx-v4 get-positions --address dydx1abc...
```

**Key parameters:**
- `--address` — dYdX chain address (dydx1...); if omitted, returns instructions

**What it does:** GET https://indexer.dydx.trade/v4/perpetualPositions?address={addr}&subaccountNumber=0&status=OPEN
Read-only. No wallet needed.

**Expected output:**
```json
{
  "ok": true,
  "address": "dydx1abc...",
  "positions": [
    {
      "market": "BTC-USD",
      "side": "LONG",
      "size": "0.1",
      "entryPrice": "69000.0",
      "unrealizedPnl": "100.0"
    }
  ]
}
```

---

### get-balance — Account equity and free collateral

**Trigger phrases:** "my dYdX balance", "dYdX account equity", "free collateral on dYdX"

**Usage:**
```
dydx-v4 get-balance --address dydx1abc...
```

**Key parameters:**
- `--address` — dYdX chain address (dydx1...); if omitted, returns instructions

**What it does:** GET https://indexer.dydx.trade/v4/addresses/{addr}/subaccountNumber/0
Read-only. No wallet needed.

**Expected output:**
```json
{
  "ok": true,
  "address": "dydx1abc...",
  "subaccountNumber": 0,
  "equity": "5000.0",
  "freeCollateral": "3500.0",
  "marginUsage": "0.30",
  "assetPositions": []
}
```

---

### deposit — Bridge DYDX tokens from Ethereum to dYdX chain

**Trigger phrases:** "bridge DYDX tokens", "deposit DYDX to dYdX", "send DYDX from Ethereum to dYdX"

IMPORTANT: Always dry-run first, then ask user to confirm before broadcasting.

**Usage:**
```
# Step 1: always dry-run first
dydx-v4 deposit --amount 100 --dydx-address dydx1abc... --dry-run

# Step 2: after user confirms, broadcast
dydx-v4 deposit --amount 100 --dydx-address dydx1abc...
```

**Key parameters:**
- `--amount` — DYDX token amount (e.g. 100 or 0.5); DYDX has 18 decimals
- `--dydx-address` — destination dYdX chain address (dydx1..., bech32 format)
- `--chain` — EVM chain ID (default 1 = Ethereum mainnet)
- `--dry-run` — simulate without broadcasting

**What it does:**
1. Encodes `bridge(uint256 amount, bytes accAddress, bytes memo)` calldata
   - amount = input * 1e18 (DYDX 18 decimals)
   - accAddress = UTF-8 bytes of the dydx bech32 address, ABI-encoded as bytes
   - memo = empty bytes
2. After user confirmation, submits via `onchainos wallet contract-call` to `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` on chain 1 with `--force`
3. The wethDYDX contract emits a Bridge event; dYdX validators credit the dydx1... address

**Prerequisites:**
- User must hold wethDYDX tokens at `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` on Ethereum
- No separate approval needed (bridge is called on the token contract itself)
- USDC deposits are NOT supported here (Noble/IBC path only)

**Expected dry-run output:**
```json
{
  "ok": true,
  "dryRun": true,
  "operation": "deposit",
  "amount": "100 DYDX",
  "amountWei": "100000000000000000000",
  "dydxAddress": "dydx1abc...",
  "chain": 1,
  "to": "0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9",
  "calldata": "0x1d45e29c...",
  "note": "Remove --dry-run and add --confirm to broadcast the transaction"
}
```

**Expected live output:**
```json
{
  "ok": true,
  "operation": "deposit",
  "txHash": "0xabc...",
  "amount": "100 DYDX",
  "dydxAddress": "dydx1abc...",
  "chain": 1,
  "note": "DYDX tokens bridged to dYdX chain. Crediting typically takes a few minutes after Ethereum confirmation."
}
```

---

### place-order — Informational only

**Trigger phrases:** "place order on dYdX", "buy BTC on dYdX", "limit order dYdX"

**Usage:**
```
dydx-v4 place-order --market BTC-USD --side buy --size 0.1 --price 70000
```

**What it does:** Returns informational response only. Does NOT broadcast any transaction.
Order placement requires Cosmos gRPC (MsgPlaceOrder) — not supported by onchainos.

**Expected output:**
```json
{
  "ok": false,
  "info": "dYdX V4 order placement requires Cosmos transaction signing (gRPC MsgPlaceOrder), which is not supported by onchainos CLI. To place orders, use: (1) dYdX web app at https://dydx.trade, (2) dYdX TypeScript SDK @dydxprotocol/v4-client-js, or (3) dYdX Python client.",
  "market": "BTC-USD",
  "side": "buy",
  "size": "0.1",
  "price": "70000"
}
```

---

## Safety Rules

1. **Dry-run first**: Always simulate deposit with `--dry-run` before broadcasting
2. **Ask user to confirm**: Show calldata and parameters, wait for explicit confirmation
3. **place-order is informational**: Never attempt to broadcast order placement via onchainos
4. **USDC deposits out of scope**: Only DYDX token bridging is supported via this plugin

---

## Troubleshooting

| Error | Solution |
|-------|---------|
| `Failed to fetch markets` | Check network connectivity to indexer.dydx.trade |
| `Invalid amount` | Ensure amount is a valid number (e.g. 100 or 0.5) |
| `Could not determine active wallet` | Run `onchainos wallet login` |
| `onchainos: command not found` | Install onchainos CLI |
| Positions/balance returns empty | Verify the address format starts with dydx1... |
