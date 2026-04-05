---
name: marinade
description: "Marinade Finance liquid staking on Solana. Stake SOL to receive mSOL and earn ~7% APY. Trigger phrases: stake SOL, get mSOL, marinade stake, unstake mSOL, liquid unstake, marinade rates, mSOL price, marinade positions, how much mSOL. Chinese: 质押SOL获取mSOL, 查询mSOL余额, 解质押mSOL, Marinade质押利率"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Architecture

- Read ops (`rates`, `positions`) → direct REST API + Solana RPC; no wallet signature needed
- Write ops (`stake`, `unstake`) → after user confirmation, submits via `onchainos swap execute --chain 501`
- All on-chain operations route through Jupiter aggregator for best execution
- Supports `--dry-run` for safe previewing

## Commands

### rates — Query mSOL/SOL exchange rate

**Trigger phrases:** "marinade rates", "mSOL exchange rate", "mSOL APY", "marinade staking yield", "what is mSOL price"

**Usage:**
```
marinade rates
```

**Output:** mSOL/SOL price ratio, approximate staking APY (~7%), total mSOL supply

**Example response:**
```json
{
  "ok": true,
  "data": {
    "msol_per_sol": 1.3714,
    "sol_per_msol": 0.7291,
    "total_msol_supply": 12345678.0,
    "staking_apy": "~7%",
    "protocol": "Marinade Finance"
  }
}
```

---

### positions — Query mSOL holdings

**Trigger phrases:** "marinade positions", "how much mSOL do I have", "my mSOL balance", "marinade holdings"

**Usage:**
```
marinade positions
```

**Output:** mSOL balance and SOL-equivalent value for the current wallet

**Example response:**
```json
{
  "ok": true,
  "data": {
    "wallet": "DTEqFXy...",
    "msol_balance": 0.001234,
    "sol_value": 0.001692,
    "msol_mint": "mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So"
  }
}
```

---

### stake — Stake SOL to receive mSOL

**Trigger phrases:** "stake SOL marinade", "get mSOL", "deposit SOL marinade", "marinade liquid stake"

**Usage:**
```
marinade stake --amount <SOL_AMOUNT> [--slippage <PCT>] [--dry-run]
```

**Parameters:**
- `--amount` (required): Amount of SOL to stake, e.g. `0.001`
- `--slippage` (optional): Slippage tolerance in percent (default: 1.0)
- `--dry-run` (optional): Preview without executing

**Execution flow:**
1. Run `--dry-run` to preview the transaction
2. **Ask user to confirm** before proceeding with on-chain execution
3. After explicit user approval, execute via `onchainos swap execute --chain 501`
4. Report txHash with Solscan explorer link

**Example response:**
```json
{
  "ok": true,
  "data": {
    "txHash": "5VfG...",
    "action": "stake",
    "from_token": "SOL",
    "to_token": "mSOL",
    "amount_sol": "0.001",
    "explorer": "https://solscan.io/tx/5VfG..."
  }
}
```

---

### unstake — Unstake mSOL back to SOL

**Trigger phrases:** "unstake mSOL", "swap mSOL to SOL", "liquid unstake marinade", "redeem mSOL"

**Usage:**
```
marinade unstake --amount <MSOL_AMOUNT> [--slippage <PCT>] [--dry-run]
```

**Parameters:**
- `--amount` (required): Amount of mSOL to unstake, e.g. `0.001`
- `--slippage` (optional): Slippage tolerance in percent (default: 1.0)
- `--dry-run` (optional): Preview without executing

**Execution flow:**
1. Run `--dry-run` to preview
2. **Ask user to confirm** before proceeding with on-chain execution
3. After explicit user approval, execute via `onchainos swap execute --chain 501`
4. Report txHash with Solscan explorer link

**Note:** Unstake uses Jupiter routing for best execution. A small fee (typically 0.1–0.3%) applies.

---

## Chain Support

| Chain | Chain ID | Supported |
|-------|----------|-----------|
| Solana | 501 | ✅ |

## Fund Limits (Testing)

- SOL per transaction: 0.001 SOL
- Hard reserve: 0.002 SOL (never spend below)
