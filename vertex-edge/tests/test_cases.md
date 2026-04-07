# Vertex Edge Plugin - Test Cases

## L1: Unit / Dry-run Tests (no network, no wallet)

### L1-1: get-markets basic
```bash
./target/release/vertex-edge get-markets --chain 42161
# Expected: ok=true, markets array with BTC-PERP, ETH-PERP entries
# Check: market_count > 0, each market has product_id, type, symbol, oracle_price_usd
```

### L1-2: get-prices basic
```bash
./target/release/vertex-edge get-prices --chain 42161
# Expected: ok=true, prices array for default perp product IDs
# Check: prices not empty, index_price_usd and mark_price_usd present
```

### L1-3: get-prices with specific product IDs
```bash
./target/release/vertex-edge get-prices --chain 42161 --product-ids 2,4
# Expected: ok=true, prices for BTC (2) and ETH (4) perps
```

### L1-4: get-orderbook by product ID
```bash
./target/release/vertex-edge get-orderbook --chain 42161 --product-id 2 --depth 5
# Expected: ok=true, bids and asks arrays with 5 levels each
# Check: best_bid < best_ask (valid spread)
```

### L1-5: get-orderbook by market name
```bash
./target/release/vertex-edge get-orderbook --chain 42161 --market BTC-PERP --depth 5
# Expected: same as L1-4, product_id=2
```

### L1-6: deposit dry-run
```bash
./target/release/vertex-edge deposit --chain 42161 --amount 100.0 --dry-run
# Expected: ok=true, dryRun=true, simulatedCommand contains depositCollateral calldata
# Check: no actual transactions submitted
```

### L1-7: get-positions dry-run (no wallet)
```bash
./target/release/vertex-edge get-positions --chain 42161 --address 0x0000000000000000000000000000000000000001
# Expected: ok=true, empty perp_positions and spot_balances (no positions for zero address)
```

### L1-8: Unsupported chain error
```bash
./target/release/vertex-edge get-markets --chain 1
# Expected: exit code 1, error contains "Unsupported chain ID: 1"
```

### L1-9: Missing market/product-id error
```bash
./target/release/vertex-edge get-orderbook --chain 42161
# Expected: exit code 1, error about missing --market or --product-id
```

---

## L2: API Integration Tests (live network, no wallet)

### L2-1: get-markets live
```bash
./target/release/vertex-edge get-markets --chain 42161
# Expected: 20+ markets returned
# Check: BTC-PERP (product_id=2), ETH-PERP (product_id=4) present
# Check: oracle_price_usd > 0 for major markets
```

### L2-2: get-prices live
```bash
./target/release/vertex-edge get-prices --chain 42161 --product-ids 2,4
# Expected: BTC price ~$50k-$120k range, ETH price ~$1k-$5k range
# Check: mark_price and index_price within 1% of each other
```

### L2-3: get-orderbook live BTC-PERP
```bash
./target/release/vertex-edge get-orderbook --chain 42161 --market BTC-PERP --depth 10
# Expected: 10 bid levels, 10 ask levels
# Check: bids[0].price < asks[0].price (valid spread < 0.5% for BTC)
```

### L2-4: get-orderbook live ETH-PERP
```bash
./target/release/vertex-edge get-orderbook --chain 42161 --product-id 4 --depth 5
# Expected: 5 levels on each side
```

### L2-5: get-positions with known address
```bash
./target/release/vertex-edge get-positions --chain 42161 --address 0x0000000000000000000000000000000000000001
# Expected: ok=true, may have empty positions (new/unused address)
# Check: subaccount field is 32-byte hex (0x + 40 + 24 chars)
```

---

## L3: Wallet Read Tests (active onchainos wallet required)

### L3-1: get-positions for active wallet
```bash
./target/release/vertex-edge get-positions --chain 42161
# Expected: ok=true, resolves wallet address automatically
# Check: address field populated with valid 0x address
```

### L3-2: deposit dry-run with real wallet
```bash
./target/release/vertex-edge deposit --chain 42161 --amount 10.0 --dry-run
# Expected: ok=true, dryRun=true, address resolved from wallet
# Check: simulatedCommand contains correct USDC address and endpoint address
```

---

## L4: On-chain Write Tests (requires funded wallet on Arbitrum)

### L4-1: deposit USDC collateral
```bash
./target/release/vertex-edge deposit --chain 42161 --amount 5.0
# Expected: ok=true, approve_txHash and deposit_txHash both valid 0x hashes
# Prerequisites: wallet must have >= 5 USDC on Arbitrum
# Note: Two transactions submitted. Confirm both in wallet.
```

**IMPORTANT**: After deposit, verify on Vertex explorer or via get-positions that balance appears.

---

## Known Limitations (v0.1)

- **place-order**: NOT IMPLEMENTED. Requires EIP-712 signing. Use Vertex web UI at app.vertexprotocol.com.
- **cancel-order**: NOT IMPLEMENTED. Requires EIP-712 signing. Use Vertex web UI.
- **withdraw-collateral**: NOT IMPLEMENTED. Requires EIP-712 signed message. Use Vertex web UI.
- **close-position**: NOT IMPLEMENTED. Place opposite-side reduce-only order via Vertex web UI.

These operations will be added in v0.2 once EIP-712 signing support is available in onchainos.
