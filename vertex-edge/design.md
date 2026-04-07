# Vertex Edge — Plugin Design

## §0 Plugin Meta

- **plugin_name**: vertex-edge
- **dapp_name**: Vertex Edge
- **version**: 0.1.0
- **target_chains**: arbitrum (chain 42161) — primary; Base (8453), Mantle (5000), Sei (1329), Sonic (146) supported with same contract pattern
- **category**: defi-protocol (perpetual DEX / cross-margin orderbook)
- **integration_path**: REST API (engine gateway + indexer/archive) for queries and order execution; direct Ethereum contract call for collateral deposit only

---

## §1 Feasibility

| # | Check | Result |
|---|-------|--------|
| 1 | Protocol is live on target chain | Arbitrum mainnet: Endpoint `0xbbEE07B3e8121227AfCFe1E2B82772246226128e`, TVL ~$81M |
| 2 | Read operations available without auth | Engine gateway `/query` (POST) and indexer archive (`/v1` POST) require no API key; no auth needed for public market data or account data with known address |
| 3 | Write operations path clear | Order placement: EIP-712 signed JSON payload → POST `{gateway}/v1/execute`. Collateral deposit only: ERC-20 approve + `depositCollateral(bytes12,uint32,uint128)` on-chain. No on-chain tx needed for orders themselves |
| 4 | No KYC / institutional gate | Vertex Edge is permissionless; no KYC requirement; no `requiresAuth` modifier on order flow |
| 5 | Execution fee within GUARDRAILS | No execution fee for placing orders via engine gateway. `submitSlowModeTransaction` (on-chain fallback) not needed for normal order flow |
| 6 | Rust SDK available | `vertex-protocol/vertex-rust-sdk` on crates.io (`vertex_sdk = "0.3.7"`). Full SDK exists; OR can use pure HTTP. Integration will use REST HTTP calls directly |
| 7 | Product IDs deterministic | USDC=0 (spot); BTC=1 (spot) / 2 (perp); ETH=3 (spot) / 4 (perp); even IDs = perp, odd IDs = spot (per protocol convention confirmed in SDK source). Dynamic lookup via `symbols` query |

---

## §2 Interface Mapping

### Operations Table

| Operation | Type | Method | Notes |
|-----------|------|--------|-------|
| `get-markets` | read | REST (engine `/query`) | Returns all products with prices, funding rates, open interest |
| `get-positions` | read | REST (engine `/query`) | Returns perp_balances + health from SubaccountInfo |
| `get-orderbook` | read | REST (gateway `/v2/orderbook`) | GET with ticker_id and depth params |
| `place-order` | write | REST (engine `/execute`) | EIP-712 signed order struct; no on-chain tx |
| `cancel-order` | write | REST (engine `/execute`) | EIP-712 signed cancellation by digest |
| `close-position` | write | REST (engine `/execute`) | Place opposite-side reduce-only order |

---

### On-chain Write Operations

Only **collateral deposit** requires an on-chain transaction. All order operations go through the off-chain engine gateway.

| Operation | Contract | Function Signature | Selector | Params |
|-----------|----------|--------------------|----------|--------|
| Deposit collateral (standard) | Endpoint (`0xbbEE07B3e8121227AfCFe1E2B82772246226128e`) | `depositCollateral(bytes12,uint32,uint128)` | `0x8e5d588c` ✅ | `subaccount_name` (bytes12, right-padded UTF-8), `product_id` (uint32), `amount` (uint128, scaled by token decimals) |
| Deposit collateral w/ referral | Endpoint | `depositCollateralWithReferral(bytes32,uint32,uint128,string)` | `0x221f0939` ✅ | `subaccount` (bytes32 = address + name), `product_id`, `amount`, `referral_code` |
| Submit slow mode tx (fallback) | Endpoint | `submitSlowModeTransaction(bytes)` | `0xe604ed9e` ✅ | `tx` (ABI-encoded transaction; used only when engine is down) |

**Prerequisite for deposit**: ERC-20 `approve(endpoint_address, amount)` must be called first on the collateral token (USDC = `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` on Arbitrum).

**Note**: Withdraw collateral is also done via the engine gateway (EIP-712 signed `withdraw_collateral` execute), NOT on-chain directly.

---

### Off-chain Read Operations (Engine Gateway)

**Base URL (Arbitrum)**: `https://gateway.prod.vertexprotocol.com/v1`

All engine queries are `POST {base_url}/query` with a JSON body `{"type": "<query_type>", ...fields}`.

#### Get All Products / Markets

```
POST /query
{"type": "all_products"}
```

Response contains `spot_products[]` and `perp_products[]`, each with `product_id`, `oracle_price_x18` (fixed-point 1e18), risk weights, config.

#### Get Market Symbols (with funding rates)

```
POST /query
{"type": "symbols", "product_type": "perp"}
```

Response: `{"data": {"symbols": {"<product_id>": {"product_id": N, "symbol": "BTC-PERP", ...}}}}`

#### Get Market Price

```
POST /query
{"type": "market_price", "product_id": 2}
```

Response: `{"data": {"bid_x18": "...", "ask_x18": "..."}}`

#### Get Subaccount Info (Positions + Margin)

```
POST /query
{
  "type": "subaccount_info",
  "subaccount": "<bytes32_hex>"  // address (20 bytes) + subaccount_name (12 bytes, right-padded)
}
```

Response: `SubaccountInfoResponse` with:
- `perp_balances[]: {product_id, balance: {amount: i128_x18, v_quote_balance: i128_x18}, lp_balance}`
- `healths[]: {assets: i128, liabilities: i128, penalty: i128}` — index 0 = initial health, index 1 = maintenance health
- `spot_balances[]`

#### Get Open Orders for Subaccount

```
POST /query
{"type": "subaccount_orders", "sender": "<bytes32_hex>", "product_id": 2}
```

Response: `{"data": {"orders": [{"digest": "0x...", "price_x18": "...", "amount": "...", "expiration": "..."}]}}`

#### Get Market Liquidity (Orderbook)

```
POST /query
{"type": "market_liquidity", "product_id": 2, "depth": 10}
```

Response: `{"data": {"bids": [[price_x18, qty_x18], ...], "asks": [...], "timestamp": N}}`

#### Get Orderbook V2 (CoinGecko format)

```
GET /v2/orderbook?ticker_id=BTC-PERP_USDC&depth=10
```

Note: gateway_url for v2 is `https://gateway.prod.vertexprotocol.com/v2` (drop `/v1`).

Response: `{"ticker_id": "BTC-PERP_USDC", "bids": [[price, qty], ...], "asks": [...], "timestamp": N}`

---

### Off-chain Read Operations (Indexer / Archive)

**Base URL (Arbitrum)**: `https://archive.prod.vertexprotocol.com/v1`

All indexer queries are `POST {archive_url}` with JSON body.

#### Get Funding Rates

```json
{"type": "funding_rate", "product_id": 2}
```

Response: `{"product_id": 2, "funding_rate_x18": "<i64>", "update_time": N}`

#### Get Historical Orders

```json
{
  "type": "orders",
  "subaccount": "<bytes32_hex>",
  "product_ids": [2, 4],
  "limit": 20
}
```

#### Get Perp Prices (oracle mark prices)

```json
{"type": "perp_prices", "product_ids": [2, 4, 6]}
```

Response: map of `product_id -> {index_price_x18, mark_price_x18, update_time}`

---

### Off-chain Write Operations (Engine Gateway Execute)

**Endpoint**: `POST https://gateway.prod.vertexprotocol.com/v1/execute`

All execute calls require an EIP-712 signature over the struct. The execute type is indicated by the `type` field embedded in the JSON (via `#[serde(rename_all = "snake_case", tag = "type")]`).

#### Place Order

```json
{
  "type": "place_order",
  "product_id": 2,
  "order": {
    "sender": "<bytes32_hex>",
    "priceX18": "<i128_as_string>",
    "amount": "<i128_as_string>",
    "expiration": "<u64_as_string>",
    "nonce": "<u64_as_string>"
  },
  "signature": "<hex_bytes>",
  "digest": "<bytes32_hex>",
  "spot_leverage": null
}
```

**Key fields**:
- `sender`: 32 bytes = `address (20 bytes) + subaccount_name_utf8_right_padded (12 bytes)`
- `priceX18`: price × 10^18 as signed 128-bit integer string (positive only for orders)
- `amount`: quantity × 10^18; **positive = long/bid, negative = short/ask**
- `expiration`: unix timestamp as u64; bits 62-63 encode order type: `00=default(GTC), 01=IOC, 10=FOK, 11=PostOnly`; bit 61 = reduce_only flag
- `nonce`: microsecond timestamp; use `gen_order_nonce()` pattern (current time in µs)
- `signature`: EIP-712 signature over the `Order` struct using the book contract as verifying contract

**Response**: `{"status": "success", "data": {"digest": "<bytes32_hex>"}}`

**EIP-712 Domain for order signing**:
- `name`: `"Vertex"`
- `version`: `"0.0.1"`
- `chainId`: 42161 (Arbitrum)
- `verifyingContract`: book address for the product (fetched from `get_contracts()` — `book_addrs[product_id]`)

#### Cancel Order(s)

```json
{
  "type": "cancel_orders",
  "tx": {
    "sender": "<bytes32_hex>",
    "productIds": [2],
    "digests": ["<bytes32_hex>"],
    "nonce": "<u64_as_string>"
  },
  "signature": "<hex_bytes>"
}
```

**Response**: `{"status": "success", "data": {"cancelled_orders": [...]}}`

#### Cancel All Orders for Product

```json
{
  "type": "cancel_product_orders",
  "tx": {
    "sender": "<bytes32_hex>",
    "productIds": [2],
    "nonce": "<u64_as_string>"
  },
  "signature": "<hex_bytes>"
}
```

#### Withdraw Collateral (off-chain signed, not a direct contract call)

```json
{
  "type": "withdraw_collateral",
  "tx": {
    "sender": "<bytes32_hex>",
    "productId": 0,
    "amount": "<u128_as_string>",
    "nonce": "<u64_as_string>"
  },
  "signature": "<hex_bytes>",
  "spot_leverage": null
}
```

---

## §3 User Scenarios

### Scenario 1 — Trader opens a BTC long position
1. User calls `get-markets` → sees BTC-PERP (product_id=2) trading at $95,000 with 0.01% funding
2. User calls `get-positions --address 0xUSER` → sees current USDC collateral balance and no open positions
3. User calls `place-order --market BTC-PERP --side long --size 0.1 --price 95000 --order-type limit`
   - Plugin builds `Order` struct: `amount = 0.1 * 1e18 = 100000000000000000`, `priceX18 = 95000 * 1e18`
   - Signs with EIP-712 using book address for product 2 as verifying contract
   - Posts to `gateway.prod.vertexprotocol.com/v1/execute`
   - Returns `{"digest": "0xABC..."}` — the order identifier
4. User calls `get-orderbook --market BTC-PERP --depth 5` to verify order appears in book

### Scenario 2 — Trader monitors and closes an ETH short position
1. User calls `get-positions --address 0xUSER` → plugin calls `subaccount_info` → shows ETH-PERP position with `amount = -500000000000000000` (short 0.5 ETH), `v_quote_balance = 2100...` (unrealized PnL)
2. User calls `get-markets` → sees ETH-PERP mark price from `perp_prices` query to archive
3. User calls `close-position --market ETH-PERP`
   - Plugin builds a long (positive amount) reduce-only order at market price
   - Sets `expiration` bits 62-63 = `01` (IOC) + bit 61 = 1 (reduce_only)
   - Posts to engine execute endpoint
4. Position closed, collateral freed

### Scenario 3 — User queries orderbook depth for arbitrage
1. User calls `get-orderbook --market ETH-PERP --depth 20`
   - Plugin calls `GET /v2/orderbook?ticker_id=ETH-PERP_USDC&depth=20`
   - Returns bid/ask levels with price and quantity
2. User identifies spread and calls `place-order --market ETH-PERP --side short --size 1 --price 3100 --order-type post-only`
   - Builds `Order` with bits 62-63 = `11` (PostOnly) on expiration
3. User calls `cancel-order --digest 0xABC...` if price moves against them
   - Plugin builds `Cancellation` struct, signs EIP-712, posts `cancel_orders` execute

---

## §4 External API Dependencies

| Service | URL | Auth | Used for |
|---------|-----|------|----------|
| Engine Gateway | `https://gateway.prod.vertexprotocol.com/v1` | None | Order execution, market queries, subaccount info |
| Indexer Archive | `https://archive.prod.vertexprotocol.com/v1` | None | Funding rates, historical orders, perp prices, market snapshots |
| Arbitrum RPC | `https://arb1.arbitrum.io/rpc` | None | ERC-20 approve, depositCollateral on-chain tx |
| Arbitrum RPC (fallback) | `https://arbitrum.publicnode.com` | None | Fallback if Arbitrum official RPC rate-limits |

**Chain-specific gateway URLs** (same pattern):
- Base: `https://gateway.base-prod.vertexprotocol.com/v1`
- Mantle: `https://gateway.mantle-prod.vertexprotocol.com/v1`
- Sei: `https://gateway.sei-prod.vertexprotocol.com/v1`
- Sonic: `https://gateway.sonic-prod.vertexprotocol.com/v1`

---

## §5 Config Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `chain_id` | u64 | 42161 | Arbitrum mainnet |
| `gateway_url` | String | `https://gateway.prod.vertexprotocol.com/v1` | Engine gateway base URL |
| `archive_url` | String | `https://archive.prod.vertexprotocol.com/v1` | Indexer archive base URL |
| `endpoint_contract` | address | `0xbbEE07B3e8121227AfCFe1E2B82772246226128e` | Vertex Endpoint (Arbitrum) |
| `querier_contract` | address | `0x1693273B443699bee277eCbc60e2C8027E91995d` | Vertex Querier (Arbitrum) |
| `clearinghouse_contract` | address | `0xAE1ec28d6225dCE2ff787dcb8CE11cF6D3AE064f` | Clearinghouse (Arbitrum) |
| `usdc_address` | address | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` | Native USDC on Arbitrum (product_id=0) |
| `subaccount_name` | String | `"default"` | 12-byte subaccount name (right-padded with nulls); users can have multiple subaccounts |
| `default_orderbook_depth` | u32 | 10 | Default depth for orderbook queries |

**All-chain contract addresses** (from `vertex-rust-sdk` deployment.json files):

| Chain | Endpoint | Quote Token |
|-------|----------|-------------|
| Arbitrum (42161) | `0xbbEE07B3e8121227AfCFe1E2B82772246226128e` | USDC `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |
| Base (8453) | `0x92C2201D48481e2d42772Da02485084A4407Bbe2` | USDC `0x833589fcd6edb6e08f4c7c32d4f71b54bda02913` |
| Mantle (5000) | `0x526D7C7ea3677efF28CB5bA457f9d341F297Fd52` | USDC `0x09Bc4E0D864854c6aFB6eB9A9cdF58aC190D0dF9` |
| Sei (1329) | `0x2777268EeE0d224F99013Bc4af24ec756007f1a6` | USDC `0x3894085Ef7Ff0f0aeDf52E2A2704928d1Ec074F1` |
| Sonic (146) | `0x2f5F835d778eBE8c28fC743E50EB9a68Ca93c2Fa` | USDC `0x29219dd400f2Bf60E5a23d13Be72B486D4038894` |

---

## §6 Known Risks / Gotchas

### 1. EIP-712 domain uses per-product BOOK address, not endpoint
Each product has its own "book" contract address as the `verifyingContract` in the EIP-712 domain. This is fetched from `get_contracts()` response as `book_addrs[product_id]`. Using the endpoint address as verifying contract will produce an invalid signature that the engine rejects silently with `error_code: -32602`.

### 2. Numeric fields are i128/u128 serialized as strings
All price and amount fields in JSON payloads are represented as decimal strings (e.g. `"95000000000000000000000"` not `95000000000000000000000`). Sending JSON numbers exceeding JavaScript's `Number.MAX_SAFE_INTEGER` will cause parse errors in the gateway. Use `serialize_i128_as_string` pattern from KNOWLEDGE_HUB.

### 3. Subaccount encoding: address + 12-byte name
The `sender` field in orders and queries is 32 bytes: `wallet_address (20 bytes) || subaccount_name_padded (12 bytes)`. The default subaccount name is `"default"` encoded as UTF-8 right-padded with null bytes to 12 bytes. Getting this encoding wrong causes "subaccount not found" errors.

### 4. Expiration field encodes order type in top 2 bits
The `expiration` u64 field is overloaded:
- Bits 0-60: unix timestamp (seconds)
- Bit 61: reduce-only flag
- Bits 62-63: order type (00=GTC, 01=IOC, 10=FOK, 11=PostOnly)
Setting `expiration = u32::MAX` with GTC bits gives a far-future GTC order. Forgetting to apply order type bits will result in unexpected GTC behavior for IOC/PostOnly orders.

### 5. Amount sign encodes direction; no explicit side field
`amount > 0` = long/bid; `amount < 0` = short/ask. There is no separate `side` field. Units are fixed-point x18 (multiply by `10^18`). Minimum size: `min_size` from symbols query (typically 0.001 BTC or 0.01 ETH).

### 6. Position close requires reduce-only IOC order at market price
There is no explicit "close position" API call. To close, place a market IOC order in the opposite direction with `reduce_only=true` bit set in expiration. Get the mark price from `perp_prices` indexer query and add/subtract slippage buffer.

### 7. Collateral deposit requires on-chain tx; orders do not
Only `depositCollateral` and ERC-20 approve are on-chain. All order operations (place, cancel, withdraw) are signed messages sent to the off-chain engine and never hit the chain directly. This means:
- Place/cancel order L4 tests work without on-chain gas (beyond initial deposit)
- The engine may reject orders if subaccount has insufficient margin (health check failure, not a revert)

### 8. Gateway SSL may block in sandbox (reqwest proxy pattern)
If the `HTTPS_PROXY` environment variable is set in the onchainos sandbox, must configure reqwest to use it explicitly. Use `build_client()` pattern from KNOWLEDGE_HUB gotchas: `if let Ok(url) = std::env::var("HTTPS_PROXY") { builder = builder.proxy(...) }`.

### 9. Nonce generation: microsecond timestamp
Order nonces must be unique and monotonically increasing per subaccount. The SDK uses `SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_micros() as u64`. Using a static nonce or milliseconds may cause nonce collision errors (`error_code: -32002`).

### 10. Product IDs: even = perp, odd = spot (except 0 = USDC)
- `0`: USDC (spot collateral)
- `1`: WBTC spot, `2`: BTC-PERP
- `3`: WETH spot, `4`: ETH-PERP
- Higher IDs: ARB, SOL, MATIC, etc. perps
Fetch dynamically via `symbols` query since Vertex regularly adds new markets. Do not hardcode product IDs beyond BTC/ETH for robustness.

### 11. `reqwest` HTTP client fails HTTPS in sandbox without proxy config
Same pattern as in KNOWLEDGE_HUB: the `reqwest` default client does not read system proxy environment variables. Build client with explicit proxy support.

### 12. L4 place-order blocked if subaccount has no deposited collateral
The engine gateway returns `{"status": "failure", "error": "insufficient_balance"}` (not a 4xx HTTP error) if margin is too low. The L4 test must either have pre-deposited USDC or use a sufficiently small order size with realistic price to pass health check. Consider testing with `dry_run` path that returns early.
