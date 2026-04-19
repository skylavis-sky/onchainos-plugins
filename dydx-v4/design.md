# dYdX V4 — Plugin Store 接入 PRD (design.md)

> Integrating dYdX V4 with onchainos CLI so AI Agents can query markets, inspect positions,
> and deposit collateral from Ethereum. Order placement and cancellation require Cosmos signing
> (gRPC broadcast) which onchainos does not support; those operations are documented as
> informational commands that surface the required API call without attempting on-chain execution.

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `dydx-v4` |
| dapp_name | dYdX V4 |
| dapp_repo | https://github.com/dydxprotocol/v4-clients |
| dapp_alias | `dydx, dydxv4, dydx-perps` |
| one_liner | Decentralized perpetuals exchange built as a Cosmos appchain; the largest on-chain perp DEX by open interest |
| category | trading-strategy |
| tags | `perps, derivatives, cosmos, orderbook, leverage` |
| target_chains | Ethereum mainnet (chain 1) for DYDX-token bridge deposits; dYdX Cosmos chain (`dydx-mainnet-1`) for all order ops — **not directly supported by onchainos** |
| target_protocols | dYdX V4 |

---

## 1. Background

### What this DApp is

dYdX V4 is a fully decentralised perpetuals exchange operating as a sovereign Cosmos appchain (`dydx-mainnet-1`). It launched mainnet in October 2023, hosts 100+ perpetual markets, and regularly ranks as the highest-volume on-chain perp venue by daily notional. The off-chain orderbook matches orders; fills settle on-chain. Users fund accounts by bridging USDC (Noble/IBC) or DYDX tokens (wethDYDX bridge on Ethereum).

### Feasibility research

| Check | Result |
|-------|--------|
| Rust SDK? | No official Rust SDK; community-maintained `v4-client-rs` exists (https://github.com/dydxprotocol/v4-clients/tree/main/v4-client-rs) |
| SDK tech stacks | TypeScript (`@dydxprotocol/v4-client-js`), Python (`v4-client-py-v2` by Nethermind), Rust (community) |
| REST API? | **Yes** — Indexer HTTP API at `https://indexer.dydx.trade/v4` (no auth required for reads) |
| Official Skill? | No |
| Community Skill? | No |
| Supported chains | dYdX Cosmos chain (`dydx-mainnet-1`) for trades; Ethereum mainnet for DYDX-token bridge; Noble for USDC IBC transfers |
| Needs onchainos broadcast? | **Partially** — EVM bridge deposit: **Yes** (Ethereum); Order placement/cancellation: **No** — requires Cosmos gRPC, not supported by onchainos |

### Integration path

```
No community Skill exists.
No official Rust SDK.
REST API exists for all reads → use API directly (Rust reqwest/ureq).
EVM deposit bridge exists on Ethereum → wallet contract-call via onchainos.
Order ops → Cosmos gRPC; onchainos cannot broadcast; expose as informational output only.
```

**Integration path: REST API + EVM contract-call (partial)**

Reference structure: https://github.com/ganlinux/plugin-store/tree/main/official/hyperliquid

---

## 2. DApp Core Capabilities & Interface Mapping

### Operations to integrate

| # | Operation | Description | On-chain / Off-chain |
|---|-----------|-------------|----------------------|
| 1 | `get-markets` | List all active perpetual markets with price, volume, OI | Off-chain (REST read) |
| 2 | `get-orderbook` | L2 orderbook for a single market | Off-chain (REST read) |
| 3 | `get-positions` | Perpetual positions for a dYdX address/subaccount | Off-chain (REST read) |
| 4 | `get-balance` | USDC and asset balances for a dYdX subaccount | Off-chain (REST read) |
| 5 | `deposit` | Bridge DYDX tokens from Ethereum to dYdX chain | On-chain (EVM, Ethereum) |
| 6 | `place-order` | Show required Cosmos tx parameters; no onchainos broadcast | Informational only |
| 7 | `cancel-order` | Show required Cosmos tx parameters; no onchainos broadcast | Informational only |

---

### Off-chain queries (REST API calls)

All endpoints use the mainnet Indexer base URL: **`https://indexer.dydx.trade/v4`**

| Operation | Method + Endpoint | Key Params | Response Fields |
|-----------|-------------------|------------|-----------------|
| `get-markets` | `GET /perpetualMarkets` | `market` (optional ticker filter), `limit` | `ticker`, `status`, `oraclePrice`, `priceChange24H`, `volume24H`, `openInterest`, `clobPairId`, `stepBaseQuantums`, `subticksPerTick` |
| `get-orderbook` | `GET /orderbooks/perpetualMarket/{market}` | `market` (path, e.g. `BTC-USD`) | `asks[]` and `bids[]` arrays of `{price, size}` |
| `get-positions` | `GET /perpetualPositions` | `address` (dYdX bech32), `subaccountNumber` (default 0), `status` (`OPEN`/`CLOSED`), `limit` | `market`, `side` (`LONG`/`SHORT`), `size`, `entryPrice`, `unrealizedPnl`, `realizedPnl`, `status` |
| `get-balance` | `GET /addresses/{address}/subaccountNumber/{subaccountNumber}` | `address` (path), `subaccountNumber` (path, default 0) | `equity`, `freeCollateral`, `assetPositions[]` → `{symbol, size, side}` |
| `get-orders` | `GET /orders` | `address`, `subaccountNumber`, `ticker`, `status` (`OPEN`/`FILLED`/`CANCELED`) | `id`, `clientId`, `ticker`, `side`, `type`, `size`, `price`, `status`, `goodTilBlock` |
| `get-fills` | `GET /fills` | `address`, `subaccountNumber`, `market`, `limit` | `market`, `side`, `price`, `size`, `fee`, `createdAt` |
| `get-historical-pnl` | `GET /historical-pnl` | `address`, `subaccountNumber`, `limit` | `equity`, `totalPnl`, `netTransfers`, `createdAt` |

---

### On-chain write operations (via onchainos)

> All on-chain transactions must be executed via `wallet contract-call` through onchainos.
> Order ops are NOT routable through onchainos — see §2 note below.

#### EVM on-chain operations (Ethereum, chain 1)

| Operation | Contract Address (source) | Function Signature (canonical) | Selector (Keccak-256 ✅) | ABI Parameter Order |
|-----------|--------------------------|-------------------------------|--------------------------|---------------------|
| `deposit` (DYDX token bridge) | `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` (WrappedEthereumDydxToken, verified on Etherscan) | `bridge(uint256,bytes,bytes)` | `0x1d45e29c` | amount (uint256, in DYDX base units 1e18), accAddress (bytes, dYdX bech32 address encoded as bytes), memo (bytes, usually empty `0x`) |
| `approve` (pre-requisite: wethDYDX allowance for itself — not needed; bridge is called directly on wethDYDX contract) | N/A — bridge is on the token contract itself | N/A | N/A | User must hold wethDYDX; no separate spender approval needed |

**Selector verification:**
```
bridge(uint256,bytes,bytes)  → keccak256[:4] = 0x1d45e29c  ✅ (computed via eth-hash, Keccak-256)
approve(address,uint256)     → 0x095ea7b3 (standard ERC-20, for reference)
```

**Important notes on the deposit flow:**
- The wethDYDX contract (`0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9`) is an ERC-20 token on Ethereum that wraps the native DYDX token and exposes a `bridge()` function.
- Calling `bridge(amount, accAddress, memo)` emits a `Bridge` event; a dYdX chain validator picks this up via an Ethereum event watcher and credits the corresponding `dydx1...` address.
- `accAddress` must be the ABI-encoded bytes of the dYdX bech32 address (NOT the 0x Ethereum address).
- USDC deposits are handled via Noble/IBC (Cosmos-native, not EVM) — **not supported via onchainos**. Only DYDX token bridging is EVM-callable.

#### Cosmos order operations (NOT supported by onchainos)

| Operation | Cosmos Message Type | gRPC Service | Required Fields | onchainos Status |
|-----------|---------------------|--------------|-----------------|------------------|
| `place-order` | `MsgPlaceOrder` | `dydxprotocol.clob.Msg/PlaceOrder` | `subaccountId`, `clientId`, `clobPairId`, `side`, `quantums`, `subticks`, `timeInForce`, `orderFlags`, `goodTilBlock` or `goodTilBlockTime` | **BLOCKED — Cosmos gRPC, not EVM** |
| `cancel-order` | `MsgCancelOrder` | `dydxprotocol.clob.Msg/CancelOrder` | `subaccountId`, `clientId`, `orderFlags`, `clobPairId`, `goodTilBlock` | **BLOCKED — Cosmos gRPC, not EVM** |

> For `place-order` and `cancel-order`, the plugin will display the required API call parameters and direct the user to the dYdX TypeScript/Python client SDK or the dYdX web app. Do NOT attempt to fake Cosmos signing through onchainos.

---

## 3. User Scenarios

**Scenario 1: Check available markets**
- User says: "Show me the top dYdX perpetual markets"
- Agent action sequence:
  1. REST read: `GET https://indexer.dydx.trade/v4/perpetualMarkets?limit=20`
  2. Parse response: extract `ticker`, `oraclePrice`, `volume24H`, `openInterest`, `priceChange24H`
  3. Sort by `volume24H` descending
  4. Return formatted table of top markets to user

**Scenario 2: Check my dYdX positions and account balance**
- User says: "What are my open positions on dYdX? My address is dydx1abc..."
- Agent action sequence:
  1. REST read: `GET https://indexer.dydx.trade/v4/addresses/dydx1abc.../subaccountNumber/0` → get equity, freeCollateral
  2. REST read: `GET https://indexer.dydx.trade/v4/perpetualPositions?address=dydx1abc...&subaccountNumber=0&status=OPEN`
  3. Parse positions: market, side (LONG/SHORT), size, entryPrice, unrealizedPnl
  4. Return: account balance summary + open positions table

**Scenario 3: Deposit DYDX tokens from Ethereum to dYdX chain**
- User says: "I want to bridge 100 DYDX tokens to my dYdX account dydx1xyz..."
- Agent action sequence:
  1. Confirm user holds wethDYDX at `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` on Ethereum
  2. Inform user: DYDX token bridging sends tokens to dYdX chain; USDC deposits require the Noble/IBC path (not supported here)
  3. Compute calldata:
     - `amount` = 100 × 10^18 = `0x56BC75E2D63100000`
     - `accAddress` = ABI-encoded bytes of `dydx1xyz...` (UTF-8 encoded as bytes)
     - `memo` = `0x` (empty)
  4. Execute on-chain: `wallet contract-call --chain 1 --address 0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9 --data <calldata> --gas-limit 150000`
  5. Wait for tx confirmation on Ethereum; inform user the dYdX chain typically credits within a few minutes

**Scenario 4: Get the orderbook for ETH-USD**
- User says: "Show me the dYdX orderbook for ETH-USD"
- Agent action sequence:
  1. REST read: `GET https://indexer.dydx.trade/v4/orderbooks/perpetualMarket/ETH-USD`
  2. Parse `bids[]` and `asks[]` arrays
  3. Display top 10 bids and asks with price and size
  4. Calculate mid-price and spread, return to user

**Scenario 5: Place a limit order (informational)**
- User says: "Place a limit buy order for 0.1 BTC-USD at $70,000 on dYdX"
- Agent action sequence:
  1. REST read: `GET https://indexer.dydx.trade/v4/perpetualMarkets?market=BTC-USD` → get `clobPairId`, `stepBaseQuantums`, `subticksPerTick`
  2. Compute quantums and subticks from size/price using market params
  3. Inform user: "dYdX order placement requires Cosmos signing via gRPC and is not supported by onchainos. Use the dYdX TypeScript SDK (`@dydxprotocol/v4-client-js`) or the dYdX web app (https://dydx.trade)."
  4. Display the constructed order parameters for reference:
     - clobPairId, side: BUY, quantums, subticks, orderFlags: 0 (short-term), timeInForce, goodTilBlock

---

## 4. External API Dependencies

| API | Base URL | Purpose | API Key Required? |
|-----|----------|---------|-------------------|
| dYdX Indexer REST (mainnet) | `https://indexer.dydx.trade/v4` | All read ops: markets, orderbook, positions, balances, orders, fills, historical PnL | No |
| dYdX Indexer REST (testnet) | `https://indexer.v4testnet.dydx.exchange/v4` | Testing only | No |
| dYdX Chain gRPC (order ops) | `grpc://oegs.dydx.trade:443` (primary); `https://dydx-dao-grpc-1.polkachu.com:443` (fallback) | place-order, cancel-order — NOT used by onchainos; documented for reference | No (but Cosmos key signing required) |
| Ethereum RPC | User-configured / onchainos default | Read wethDYDX balance, send bridge() tx | onchainos manages |

---

## 5. Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `indexer_url` | `https://indexer.dydx.trade/v4` | Indexer base URL (swap for testnet during dev/test) |
| `dydx_address` | (required for account ops) | dYdX bech32 address (`dydx1...`) |
| `subaccount_number` | `0` | Subaccount index (most users use 0) |
| `default_market` | `BTC-USD` | Default market for orderbook / market queries |
| `chain_id` | `1` (Ethereum) | EVM chain for bridge deposit |
| `bridge_contract` | `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` | wethDYDX bridge contract on Ethereum (fixed) |
| `order_limit` | `20` | Default row limit for list endpoints |
| `dry_run` | `true` | Simulate mode; do not broadcast real transactions |

---

## 6. Agent Execution Guide

### Phase 1: Requirements Analysis (Researcher Agent) — COMPLETE

1. Plugin meta confirmed: `dydx-v4`, Cosmos appchain with EVM deposit bridge
2. Indexer REST API documented: base URL `https://indexer.dydx.trade/v4`, all endpoints mapped
3. EVM bridge contract confirmed: `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9`, selector `0x1d45e29c`
4. Order ops scoped as informational only (Cosmos gRPC, not EVM)
5. Interface mapping table complete in §2

### Phase 2: Code Implementation (Developer Agent)

1. Read Plugin Store dev guide: https://github.com/skylavis-sky/plugin-store-demo/blob/main/PLUGIN_DEVELOPMENT_GUIDE_ZH.md
2. Read onchainos skills: https://github.com/okx/onchainos-skills/tree/main/skills
3. Create Rust project in `~/projects/plugin-store-dev/dydx-v4/`
4. Implement commands:
   - `get-markets` — `GET /perpetualMarkets`, format table output
   - `get-orderbook <market>` — `GET /orderbooks/perpetualMarket/{market}`, display bids/asks
   - `get-positions --address <dydx1...>` — `GET /perpetualPositions`, display open positions
   - `get-balance --address <dydx1...>` — `GET /addresses/{addr}/subaccountNumber/0`
   - `deposit --amount <N> --to <dydx1...> --dry-run` — build calldata for `bridge(uint256,bytes,bytes)` and route through `wallet contract-call --chain 1`
   - `place-order` / `cancel-order` — informational output only; display parameters, no tx broadcast
5. Add `--confirm` gate on `deposit` before any real tx
6. Run `plugin-store lint` locally before PR

### Phase 3: Testing (Tester Agent)

1. Generate test cases from §3 user scenarios
2. Test all reads against live indexer (no auth needed)
3. Test `deposit --dry-run` calldata construction and encoding
4. Verify `place-order` / `cancel-order` return informational output and do NOT attempt tx broadcast
5. Verify `--confirm` gate on deposit

### Phase 4: PR Submission (Submitter Agent)

1. Target: skylavis-sky/plugin-store-demo
2. PR title: `feat: dydx-v4 plugin — REST reads + DYDX token bridge deposit`
3. Wait for CI; resolve review feedback

---

## 7. Open Questions

- [ ] **USDC deposit path**: The standard deposit route is Noble/IBC (Cosmos-native), not EVM. Should the plugin also document the Skip Go Fast API path for USDC deposits from Ethereum? Skip Go Fast uses a `submitOrder` function on a Skip Protocol contract — requires locating the contract address.
- [ ] **wethDYDX vs native DYDX**: Users holding native DYDX (ERC-20 at a different address) may first need to wrap it. Confirm whether wethDYDX wrapping is in scope.
- [ ] **dYdX address derivation**: A dYdX V4 address (`dydx1...`) is derived from an Ethereum key via a deterministic algorithm. Should the plugin include a `derive-address --eth-address 0x...` helper command?
- [ ] **Testnet indexer URL**: Use `https://indexer.v4testnet.dydx.exchange/v4` during CI testing to avoid rate limits on mainnet indexer.
- [ ] **Rate limits**: The mainnet indexer has no documented rate limit but community validators (Polkachu: 300 req/min, KingNodes: 250 req/min) do. The first-party indexer should be preferred and may impose unlisted limits.
