---
name: pump-fun
description: "Interact with pump.fun bonding curves on Solana: buy tokens, sell tokens, and check prices/bonding progress. Trigger phrases: buy pump.fun token, sell pump.fun token, check pump.fun price, pump.fun bonding curve. Chinese: 购买pump.fun代币, 出售pump.fun代币, 查询pump.fun价格"
license: MIT
metadata:
  author: skylavis-sky
  version: "0.2.0"
---

## Architecture

- **Read ops** (`get-token-info`, `get-price`) → query Solana RPC directly via `pumpfun` Rust crate; no confirmation needed
- **Write ops** (`buy`, `sell`) → route through `onchainos swap execute --chain solana`; works for both bonding curve tokens and graduated tokens (PumpSwap/Raydium)

> **Not supported:** `create-token` requires two signers (mint keypair + MPC wallet), which is incompatible with the onchainos MPC wallet model. Token creation is not available.

## Chain

Solana mainnet (chain 501). Program: `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview the operation
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash (Solana signature) and outcome

---

## Operations

### get-token-info — Fetch bonding curve state

Reads on-chain `BondingCurveAccount` for a token and returns reserves, price, market cap, and graduation progress.

```bash
pump-fun get-token-info --mint <MINT_ADDRESS>
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--rpc-url` (optional): Solana RPC URL (default: mainnet-beta public; set `HELIUS_RPC_URL` env var for production)

**Output fields:**
- `virtual_token_reserves`, `virtual_sol_reserves`, `real_token_reserves`, `real_sol_reserves`
- `token_total_supply`, `complete` (bonding curve graduated?), `creator`
- `price_sol_per_token`, `market_cap_sol`, `final_market_cap_sol`
- `graduation_progress_pct` (0–100%), `status`

---

### get-price — Get buy or sell price

Calculates the expected output for a given buy (SOL→tokens) or sell (tokens→SOL) amount.

```bash
pump-fun get-price --mint <MINT_ADDRESS> --direction buy --amount 100000000
pump-fun get-price --mint <MINT_ADDRESS> --direction sell --amount 5000000
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--direction` (required): `buy` or `sell`
- `--amount` (required): SOL lamports for buy; token units for sell
- `--fee-bps` (optional): Fee basis points for sell calculation (default: 100)
- `--rpc-url` (optional): Solana RPC URL

---

### buy — Buy tokens on bonding curve

Purchases tokens on a pump.fun bonding curve via `onchainos swap execute`. Works for both bonding curve tokens and graduated tokens. Run `--dry-run` to preview, then **ask user to confirm** before proceeding.

```bash
# Preview
pump-fun buy --mint <MINT> --sol-amount 0.01 --dry-run

# Execute after user confirms
pump-fun buy --mint <MINT> --sol-amount 0.01 --slippage-bps 200
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--sol-amount` (required): SOL amount in readable units (e.g. `0.01` = 0.01 SOL)
- `--slippage-bps` (optional): Slippage tolerance in bps (default: 100)
- `--dry-run` (optional): Preview without broadcasting

---

### sell — Sell tokens back to bonding curve

Sells tokens back to a pump.fun bonding curve (or DEX if graduated) for SOL via `onchainos swap execute`. Run `--dry-run` to preview, then **ask user to confirm** before proceeding.

```bash
# Preview
pump-fun sell --mint <MINT> --token-amount 1000000 --dry-run

# Sell a specific amount after user confirms
pump-fun sell --mint <MINT> --token-amount 1000000

# Sell ALL tokens after user confirms
pump-fun sell --mint <MINT>
```

**Parameters:**
- `--mint` (required): Token mint address (base58)
- `--token-amount` (optional): Readable token amount to sell (e.g. `1000000`); omit to sell entire balance
- `--slippage-bps` (optional): Slippage tolerance in bps (default: 100)
- `--dry-run` (optional): Preview without broadcasting

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `HELIUS_RPC_URL` | Helius RPC endpoint (recommended for production; higher rate limits than public mainnet-beta) |

## Configuration Defaults

| Parameter | Default | Description |
|-----------|---------|-------------|
| `slippage_bps` | 100 | 1% slippage tolerance |
| `fee_bps` | 100 | pump.fun trade fee (1%) |
