---
name: fenix
description: "Fenix Finance plugin for swapping tokens and managing concentrated liquidity on Blast (chain 81457). Trigger phrases: swap on Fenix, Fenix swap, Fenix Finance swap, add liquidity Fenix, Fenix pools, get Fenix quote, Fenix DEX, trade on Blast, Fenix add-liquidity, concentrated liquidity Blast, Algebra swap Blast."
version: "0.1.0"
author: "skylavis-sky"
tags:
  - dex
  - swap
  - liquidity
  - blast
  - algebra
  - clmm
  - concentrated-liquidity
  - ve33
---

## Architecture

- Read ops (`get-quote`, `get-pools`) -- direct `eth_call` via Blast RPC or GraphQL; no confirmation needed
- Write ops (`swap`, `add-liquidity`) -- after user confirmation, submits via `onchainos wallet contract-call`

## Chain

Fenix Finance is deployed on **Blast only** (chain ID: 81457).

All commands target Blast. There is no `--chain` flag; the chain is fixed.

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata and expected output
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and Blastscan link

## Contract Addresses (Blast)

| Contract | Address |
|----------|---------|
| SwapRouter | 0x2df37Cb897fdffc6B4b03d8252d85BE7C6dA9d00 |
| QuoterV2 | 0x94Ca5B835186A37A99776780BF976fAB81D84ED8 |
| Algebra Factory | 0x7a44CD060afC1B6F4c80A2B9b37f4473E74E25Df |
| NFPM | 0x8881b3Fb762d1D50e6172f621F107E24299AA1Cd |
| WETH | 0x4300000000000000000000000000000000000004 |
| USDB | 0x4300000000000000000000000000000000000003 |
| FNX | 0x52f847356b38720B55ee18Cb3e094ca11C85A192 |

## Token Symbols

Use `WETH`, `USDB`, or `FNX` as shorthand, or pass full 0x addresses.

## Command Routing

| User Intent | Command |
|-------------|---------|
| "Quote 1 WETH to USDB on Fenix" | `get-quote` |
| "Swap 100 USDB for WETH on Fenix" | `swap` |
| "List Fenix pools" | `get-pools` |
| "Add liquidity to WETH/USDB on Fenix" | `add-liquidity` |

---

## get-quote -- Swap Quote

**Trigger phrases:** Fenix quote, how much WETH for USDB Fenix, Fenix price, get quote Fenix

**Usage:**
```
fenix get-quote --token-in <symbol|address> --token-out <symbol|address> --amount-in <minimal_units>
```

**Parameters:**
- `--token-in` -- Input token symbol (WETH, USDB, FNX) or 0x address
- `--token-out` -- Output token symbol or address
- `--amount-in` -- Input amount in minimal units (e.g. 1000000000000000000 = 1 WETH)

**Expected output:**
```json
{
  "ok": true,
  "chain": "blast",
  "token_in": { "symbol": "WETH", "amount_human": "1.000000" },
  "token_out": { "symbol": "USDB", "amount_human": "3421.500000" },
  "rate": "3421.500000"
}
```

**No user confirmation required** -- read-only eth_call.

---

## swap -- Execute Swap

**Trigger phrases:** swap on Fenix, Fenix swap, trade WETH for USDB Fenix, exchange tokens Fenix

**Usage:**
```
fenix [--dry-run] swap --token-in <symbol|address> --token-out <symbol|address> --amount-in <minimal_units> [--slippage 0.005] [--deadline-secs 300]
```

**Parameters:**
- `--token-in` -- Input token symbol or address
- `--token-out` -- Output token symbol or address
- `--amount-in` -- Input amount in minimal units
- `--slippage` -- Slippage tolerance fraction (default: 0.005 = 0.5%)
- `--deadline-secs` -- Deadline offset from now in seconds (default: 300)
- `--dry-run` -- Preview calldata without broadcasting

**Execution flow:**
1. Run `--dry-run` to preview expected output and calldata
2. **Ask user to confirm** the swap parameters
3. Resolve wallet address via `onchainos wallet balance --chain 81457`
4. Check ERC-20 allowance for SwapRouter; approve if needed (3s wait)
5. Execute `exactInputSingle` via `onchainos wallet contract-call --force`
6. Report `txHash` and Blastscan link

**Example:**
```
fenix swap --token-in USDB --token-out WETH --amount-in 100000000000000000000 --slippage 0.005
```

---

## get-pools -- List Fenix V3 Pools

**Trigger phrases:** list Fenix pools, Fenix pool list, Fenix TVL, Fenix liquidity pools

**Usage:**
```
fenix get-pools [--limit 20]
```

**Parameters:**
- `--limit` -- Max number of pools to display sorted by TVL (default: 20)

**Expected output:**
```json
{
  "ok": true,
  "chain": "blast",
  "count": 20,
  "pools": [
    {
      "address": "0x...",
      "token0": { "symbol": "WETH" },
      "token1": { "symbol": "USDB" },
      "tvl_usd": "1234567.89",
      "volume_usd": "456789.00",
      "fees_usd": "1234.56"
    }
  ]
}
```

**No user confirmation required** -- read-only GraphQL query.

---

## add-liquidity -- Add Concentrated Liquidity

**Trigger phrases:** add liquidity Fenix, provide liquidity Fenix, LP Fenix, Fenix mint position

**Usage:**
```
fenix [--dry-run] add-liquidity --token0 <symbol|address> --token1 <symbol|address> --amount0 <minimal_units> --amount1 <minimal_units> --tick-lower <i32> --tick-upper <i32> [--deadline-secs 300]
```

**Parameters:**
- `--token0` -- First token symbol or address
- `--token1` -- Second token symbol or address
- `--amount0` -- Desired amount of token0 in minimal units
- `--amount1` -- Desired amount of token1 in minimal units
- `--tick-lower` -- Lower tick of price range (negative values allowed; use e.g. `-887220`)
- `--tick-upper` -- Upper tick of price range (e.g. `887220` for full range)
- `--deadline-secs` -- Deadline offset in seconds (default: 300)
- `--dry-run` -- Preview without broadcasting

**Execution flow:**
1. Run `--dry-run` to preview calldata
2. **Ask user to confirm** the amounts and tick range
3. Resolve wallet address
4. Approve token0 for NFPM if allowance < amount0 (5s wait)
5. Approve token1 for NFPM if allowance < amount1 (5s wait)
6. **Ask user to confirm** the mint before broadcasting
7. Execute NFPM `mint` via `onchainos wallet contract-call --force`
7. Report `txHash`, position NFT token ID, and Blastscan link

**Tick calculation guide:**
- Full range: `--tick-lower -887220 --tick-upper 887220`
- +/-10% price range: calculate from current price using `log(1.0001, price_ratio)`
- Use `get-quote` to find current price before setting ticks

**Example -- add full-range WETH/USDB liquidity:**
```
fenix add-liquidity --token0 WETH --token1 USDB --amount0 100000000000000000 --amount1 341000000000000000000 --tick-lower -887220 --tick-upper 887220
```

---

## Troubleshooting

| Error | Cause | Fix |
|-------|-------|-----|
| `No Fenix pool found for pair` | Pool not deployed for this pair | Check `get-pools` output |
| `Quote returned 0` | Pool has no liquidity | Try smaller amount or different pair |
| `Cannot determine wallet address` | Not logged in to onchainos | Run `onchainos wallet login` |
| `eth_call error` | RPC issue or wrong calldata | Check Blast RPC availability |
| `txHash: pending` | onchainos did not confirm | `--force` flag is applied automatically |

## Security Notes

- ERC-20 allowance is checked before every approve to avoid duplicate transactions
- Slippage defaults to 0.5% -- always pass explicit `--slippage` for large swaps
- Use `--dry-run` to preview all write operations before execution
- Tick values may be negative; use `--tick-lower -887220` syntax
