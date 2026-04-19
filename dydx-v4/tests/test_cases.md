# Test Cases — dydx-v4

**Date:** 2026-04-19  
**Plugin version:** 0.1.0  
**Tester:** Phase 3 Agent

---

## TC-01: get-markets — list all perpetual markets

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-markets` |
| Level | L2 (read) |
| Expected | `ok: true`, `count` >= 1, `markets` array with `ticker`, `status`, `indexPrice` fields |

---

## TC-02: get-orderbook BTC-USD

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-orderbook --market BTC-USD` |
| Level | L2 (read) |
| Expected | `ok: true`, `market: "BTC-USD"`, `bids` and `asks` arrays with `price`/`size` objects |

---

## TC-03: get-orderbook ETH-USD

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-orderbook --market ETH-USD` |
| Level | L2 (read) |
| Expected | `ok: true`, `market: "ETH-USD"`, `bids` and `asks` non-empty |

---

## TC-04: get-positions — no address (guidance path)

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-positions` |
| Level | L2 (read) |
| Expected | `ok: true`, `info` field with guidance to provide `--address dydx1...` |

---

## TC-05: get-balance — no address (guidance path)

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-balance` |
| Level | L2 (read) |
| Expected | `ok: true`, `info` field with guidance to provide `--address dydx1...` |

---

## TC-06: place-order — informational stub

| Field | Value |
|-------|-------|
| Command | `dydx-v4 place-order --market BTC-USD --side buy --size 0.001 --price 50000` |
| Level | L2 (read/stub) |
| Expected | `ok: false`, `info` explains Cosmos gRPC requirement, echoes `market`/`side`/`size`/`price` params |

---

## TC-07: deposit — dry-run calldata verification

| Field | Value |
|-------|-------|
| Command | `dydx-v4 deposit --amount 1.0 --dydx-address dydx1abc123testaddressfordryruntesting00000 --chain 1 --dry-run` |
| Level | L3 (dry-run) |
| Expected | `ok: true`, `dryRun: true`, `operation: "deposit"`, calldata starts with `0x1d45e29c`, `to: "0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9"`, `amountWei: "1000000000000000000"` |

---

## TC-08: deposit — live broadcast (L4, requires DYDX tokens)

| Field | Value |
|-------|-------|
| Command | `dydx-v4 deposit --amount 1.0 --dydx-address dydx1... --chain 1 --confirm` |
| Level | L4 (write) |
| Expected | `ok: true`, `txHash` present, `operation: "deposit"` |
| Blocker | Requires DYDX tokens on Ethereum mainnet (symbol "DYDX" at `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9`) |

---

## TC-09: get-positions — with valid address

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-positions --address dydx1abc...` |
| Level | L2 (read) |
| Expected | `ok: true`, `address` echoed, `positions` array (may be empty for unused address) |

---

## TC-10: get-balance — with valid address

| Field | Value |
|-------|-------|
| Command | `dydx-v4 get-balance --address dydx1abc...` |
| Level | L2 (read) |
| Expected | `ok: true`, `address` echoed, `equity`, `freeCollateral`, `subaccountNumber: 0` |
