# Meteora DLMM Plugin — Test Cases

## L1: Binary Smoke Tests

```bash
# Verify binary is present and shows help
./target/release/meteora --help
./target/release/meteora get-pools --help
./target/release/meteora get-pool-detail --help
./target/release/meteora get-swap-quote --help
./target/release/meteora get-user-positions --help
./target/release/meteora swap --help
```

## L2: Read-Only API Tests (No Wallet Required)

### get-pools: Default list (top TVL)
```bash
./target/release/meteora get-pools --page-size 5
# Expected: ok=true, pools array with 5 items, each with address/name/tvl/apy fields
```

### get-pools: Search by token pair
```bash
./target/release/meteora get-pools --search-term "SOL-USDC" --sort-key tvl --order-by desc
# Expected: pools named "SOL-USDC" sorted by descending TVL
```

### get-pools: Sort by APY
```bash
./target/release/meteora get-pools --sort-key apr --order-by desc --page-size 10
# Expected: pools sorted by apr descending; pools with apy > 50 should have apy_risk_warning
```

### get-pool-detail: Valid pool
```bash
./target/release/meteora get-pool-detail --address 5rCf1DM8LjKTw4YqhnoLcngyZYeNnQqztScTogYHAS6
# Expected: ok=true, pool detail with token_x=SOL, token_y=USDC, pool_config.bin_step=4
```

### get-pool-detail: Invalid address
```bash
./target/release/meteora get-pool-detail --address invalidaddress123
# Expected: error with Meteora API error message
```

### get-swap-quote: SOL to USDC
```bash
./target/release/meteora get-swap-quote \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 1.0
# Expected: ok=true, quote object with from_amount_readable=1.0
```

### get-user-positions: No wallet logged in (should fail gracefully)
```bash
./target/release/meteora get-user-positions
# Expected: error message about missing wallet or empty positions list
```

### get-user-positions: Explicit wallet
```bash
./target/release/meteora get-user-positions --wallet GbE9k66MjLRQC7RnMCkRuSgHi3Lc8LJQXWdCmYFtGo2
# Expected: ok=true, positions array (may be empty if wallet has no positions)
```

## L3: Dry Run Tests (No On-Chain Execution)

### swap: Dry run preview
```bash
./target/release/meteora --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.1
# Expected: ok=true, dry_run=true, no txHash, quote data returned
```

### swap: Dry run does NOT require wallet
```bash
# Even without onchainos logged in, dry_run should succeed
./target/release/meteora --dry-run swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.1
# Expected: returns quote data, no wallet resolution attempted
```

## L4: Live Execution Tests (Requires Wallet Login)

> Prerequisites: `onchainos` wallet logged in with Solana account funded with SOL and test tokens.

### swap: Execute small SOL → USDC swap
```bash
./target/release/meteora swap \
  --from-token So11111111111111111111111111111111111111112 \
  --to-token EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v \
  --amount 0.001 \
  --slippage 1.0
# Expected: ok=true, tx_hash with Solscan URL
```

## Solana-Specific Checks

| Check | Expected |
|-------|---------|
| No `--force` flag used in swap | Solana swap execute does not need `--force` |
| `--chain solana` used (not `--chain 501`) | `onchainos swap` commands use chain name |
| `wallet balance --chain 501` used for wallet resolution | Uses numeric chain ID for wallet commands |
| dry_run guard before resolve_wallet_solana() | No wallet call during dry run |

## API Endpoints Used

| Endpoint | Command |
|----------|---------|
| `GET https://dlmm.datapi.meteora.ag/pools` | `get-pools` |
| `GET https://dlmm.datapi.meteora.ag/pools/{address}` | `get-pool-detail` |
| `GET https://dlmm.datapi.meteora.ag/positions/{wallet}` | `get-user-positions` |
| `onchainos swap quote --chain solana` | `get-swap-quote` |
| `onchainos swap execute --chain solana` | `swap` (live) |
| `onchainos wallet balance --chain 501` | wallet resolution |
