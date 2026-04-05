---
name: maple
description: Maple Finance institutional lending — deposit USDC/USDT into syrup pools to earn yield
version: "0.1.0"
---

# Maple Finance Skill

> ⚠️ **Protocol Status: KYC/Authorization Required for Deposits**
> Maple Finance is an institutional lending protocol. Deposits require wallet authorization via `PoolPermissionManager` (off-chain KYC/allowlist). The `deposit` and `withdraw` commands will produce correct calldata but on-chain execution requires your wallet to be whitelisted by a pool delegate. Contact Maple Finance at https://maple.finance to apply.

Maple Finance is an institutional lending protocol on Ethereum. Users can deposit USDC or USDT into syrup pool vaults (ERC-4626) to earn yield.

Supported chains: **Ethereum (chain 1)**

## Architecture

- Read ops (pools, positions, rates) → direct `eth_call` via public RPC
- Write ops → after user confirmation, submits via `onchainos wallet contract-call`

---

## Commands

### pools — List syrup pools

Lists all Maple Finance syrup pools with TVL.

**Trigger examples:**
- "Show Maple Finance pools"
- "List Maple lending pools"
- "What pools does Maple have?"

**Command:**
```bash
maple pools --chain 1
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "pools": [
      {
        "name": "syrupUSDC",
        "pool_address": "0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b",
        "underlying_symbol": "USDC",
        "total_assets_formatted": "1750000.00 USDC",
        "exchange_rate": "1.158428"
      }
    ]
  }
}
```

---

### positions — Show lending positions

Shows the user's current lending positions (shares held and their underlying value).

**Trigger examples:**
- "Show my Maple Finance positions"
- "How much USDC do I have in Maple?"
- "Check my Maple lending balance"

**Command:**
```bash
maple positions --chain 1
# or with explicit wallet:
maple positions --chain 1 --from 0x...
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "wallet": "0x...",
    "positions": [
      {
        "pool": "syrupUSDC",
        "shares_formatted": "0.863291",
        "underlying_value_formatted": "1.000000 USDC"
      }
    ]
  }
}
```

---

### rates — Show pool exchange rates

Shows exchange rates and TVL for all pools.

**Trigger examples:**
- "Show Maple Finance rates"
- "What's the APY on Maple?"
- "Show Maple pool TVL"

**Command:**
```bash
maple rates --chain 1
```

**Output:**
```json
{
  "ok": true,
  "data": {
    "rates": [
      {
        "pool": "syrupUSDC",
        "tvl_formatted": "1750000.00 USDC",
        "exchange_rate": "1.15842800"
      }
    ]
  }
}
```

---

### deposit — Deposit into a syrup pool

Deposits USDC or USDT into a Maple Finance syrup pool.

**Trigger examples:**
- "Deposit 0.01 USDC into Maple"
- "Add 0.01 USDT to Maple syrupUSDT pool"
- "Invest USDC in Maple Finance"

**Command:**
```bash
# Dry-run (preview only)
maple deposit --pool usdc --amount 0.01 --chain 1 --dry-run

# Live (ask user to confirm before executing)
maple deposit --pool usdc --amount 0.01 --chain 1
```

**Flow (on-chain):**
1. Run `--dry-run` to preview calldata
2. **Ask user to confirm** before proceeding with on-chain transactions
3. Submit ERC-20 approve via `onchainos wallet contract-call` (selector `0x095ea7b3`)
4. Wait 3 seconds
5. Submit SyrupRouter.deposit via `onchainos wallet contract-call` (selector `0xc9630cb0`)

**Output:**
```json
{
  "ok": true,
  "data": {
    "pool": "syrupUSDC",
    "token": "USDC",
    "amount": 0.01,
    "calldata": "0xc9630cb0...",
    "txHash": "0x..."
  }
}
```

**Supported pools:** `syrupUSDC` / `usdc`, `syrupUSDT` / `usdt`

---

### withdraw — Request redemption from a pool

Initiates a withdrawal by calling `requestRedeem` on the pool contract. This enqueues shares in the withdrawal queue — funds are released after queue processing (timing depends on pool liquidity).

**Trigger examples:**
- "Withdraw my USDC from Maple"
- "Request redemption from Maple syrupUSDT"
- "Exit my Maple position"

**Command:**
```bash
# Dry-run (preview only)
maple withdraw --pool usdc --chain 1 --dry-run

# Specific amount
maple withdraw --pool usdc --shares 0.5 --chain 1 --dry-run

# Live (ask user to confirm before executing)
maple withdraw --pool usdc --chain 1
```

**Flow (on-chain):**
1. Run `--dry-run` to preview calldata
2. **Ask user to confirm** before proceeding with on-chain transactions
3. Fetch current share balance via `eth_call` (balanceOf)
4. Submit Pool.requestRedeem via `onchainos wallet contract-call` (selector `0x107703ab`)

**Output:**
```json
{
  "ok": true,
  "data": {
    "pool": "syrupUSDC",
    "shares_formatted": "0.863291",
    "txHash": "0x...",
    "note": "requestRedeem enqueues your shares for withdrawal. Funds will be available after the withdrawal queue processes."
  }
}
```

---

## Notes

- Maple Finance is an **institutional lending** protocol. Deposits go to vetted institutional borrowers.
- Withdrawal is a 2-step process: `requestRedeem` → wait for queue → `redeem`. This skill handles the first step.
- USDT deposits require setting allowance to 0 first if a prior allowance exists (USDT race condition). The deposit command handles this automatically.
- Exchange rate > 1.0 means your USDC/USDT has grown since deposit.
