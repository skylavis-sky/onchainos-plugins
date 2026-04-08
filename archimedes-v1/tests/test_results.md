# Archimedes Finance Plugin — Phase 3 Test Results

**Date:** 2026-04-05  
**Tester:** Tester Agent  
**Plugin:** archimedes v0.1.0  
**Chain:** Ethereum Mainnet (chain 1)

---

## Summary

| Phase | Result | Notes |
|-------|--------|-------|
| L1 Build + Lint | PASS | Clean build, lint passes |
| L2 Read Ops | PASS | protocol-info and get-positions work correctly |
| L3 Dry-Run | PASS (after bug fix) | Bug found and fixed in close-position |
| L4 Live | SKIP | Insufficient funds (5 USDT only; min collateral 750 OUSD) |

**Overall Verdict: READY** (pending note on CLI flag naming vs test spec)

---

## L1 — Build + Lint

**Result: PASS**

- `cargo build --release` completed successfully (0.16s on warm cache, ~15s cold).
- `plugin-store lint .` output: `✓ Plugin 'archimedes' passed all checks!`

---

## L2 — Read Operations

**Result: PASS**

### protocol-info

Returns meaningful on-chain data with no external API dependency:

```json
{
  "archToLevRatio": "80000000000000000000000",
  "availableLvUSD": "90616.079054",
  "availableLvUSDRaw": "90616079054795186349153",
  "chain": "Ethereum Mainnet",
  "chainId": 1,
  "contracts": { ... },
  "maxCycles": 13,
  "minPositionCollateralOUSD": "750.000000",
  "minPositionCollateralRaw": "750000000000000000000",
  "ok": true,
  "originationFeeRate": "1000000000000000"
}
```

All values sourced via direct `eth_call` to Ethereum mainnet.

### get-positions

Returns gracefully with empty list (no crash, no error):

```json
{
  "ok": true,
  "positionCount": 0,
  "positions": [],
  "wallet": "0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9"
}
```

- `resolve_wallet()` correctly reads from `onchainos wallet balance --chain 1` without `--output json` flag.
- Empty position list handled as `"positionCount": 0` with empty array — not an error.

---

## L3 — Dry-Run

**Result: PASS (after fix)**

### Bug Found and Fixed

**Bug:** `close-position --dry-run` crashed with `"Failed to fetch NFT owner"` when called with a dummy token ID (e.g., `--token-id 1`). The ownership check via `rpc::owner_of` was called unconditionally before checking `dry_run`, causing an RPC revert for non-existent tokens.

**Fix applied** in `/src/commands/close_position.rs`:
- Wrapped ownership verification in `if !dry_run { ... }` guard.
- Read operations for position value (getOUSDTotalIncludeInterest, getLvUSDBorrowed) remain best-effort (return 0 if token doesn't exist) — appropriate for dry-run simulation.

### open-position dry-run

Command tested:
```bash
./target/release/archimedes open-position \
  --token 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --amount 10 \
  --cycles 3 \
  --dry-run
```

Calldata verification:
- Step 1 approve: `0x095ea7b3...` ✓ (ERC-20 approve selector)
- Step 2 zapIn: `0x657d81f7...` ✓ (Zapper.zapIn selector)

Preview data fetched from on-chain:
- `previewOUSDOut`: 9.992553 OUSD
- `previewARCHNeeded`: 0.706486 ARCH

### close-position dry-run

Command tested:
```bash
./target/release/archimedes close-position \
  --token-id 1 \
  --min-return 0 \
  --dry-run
```

Calldata verification:
- Step 1 setApprovalForAll: `0xa22cb465...` ✓ (ERC-721 setApprovalForAll selector)
- Step 2 unwindLeveragedPosition: `0xdafccdd9...` ✓ (LeverageEngine.unwindLeveragedPosition selector)

---

## L4 — Live Execution

**Result: SKIP**

Wallet balance on chain 1 (`0xee385ac7ac70b5e7f12aa49bf879a441bed0bae9`):
- ETH: 0.002947 (~$6.00) — low gas, borderline
- USDT: 5.0 — below minimum position collateral of 750 OUSD
- No USDC, no DAI

Protocol minimum position collateral: 750 OUSD. Skipping live execution.

---

## Key Behaviors Verified

| Behavior | Status |
|----------|--------|
| `get-positions` gracefully handles empty list | PASS — returns `positionCount: 0` |
| `protocol-info` reads from on-chain (no external API) | PASS — direct eth_call to publicnode.com |
| dry-run shows calldata without broadcasting | PASS — both commands output full steps |
| `resolve_wallet` works for chain 1 without `--output json` | PASS — uses `wallet balance --chain 1` natively |

---

## Contract Address Verification

All addresses in `src/config.rs` match the provided spec:

| Contract | Expected | Config |
|----------|----------|--------|
| LeverageEngine (proxy) | `0x03dc7Fa99B986B7E6bFA195f39085425d8172E29` | ✓ |
| Zapper (proxy) | `0x624f570C24d61Ba5BF8FBFF17AA39BFc0a7b05d8` | ✓ |
| PositionToken (ERC-721) | `0x14c6A3C8DBa317B87ab71E90E264D0eA7877139D` | ✓ |

---

## Notes

- **CLI flag naming**: The test spec references `--collateral` and `--leverage` flags, but the plugin was implemented with `--token` and `--cycles` respectively (consistent with the design.md spec). This is not a bug — the test spec used informal flag names. The plugin's actual interface is correct.
- **Min collateral**: Protocol enforces 750 OUSD minimum; 10 USDC dry-run test is for calldata validation only (would revert live).
- **ABI selectors**: All four selectors (`0x095ea7b3`, `0x657d81f7`, `0xa22cb465`, `0xdafccdd9`) confirmed correct via direct computation and on-chain cross-reference.
