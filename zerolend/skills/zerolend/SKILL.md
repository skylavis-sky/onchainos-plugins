---
name: zerolend
description: "Supply, borrow, repay, and withdraw on ZeroLend — an Aave V3 fork on zkSync Era, Linea, and Blast. Trigger phrases: zerolend supply, zerolend borrow, zerolend repay, zerolend deposit, zerolend withdraw, lend on zerolend, borrow on linea, borrow on blast, zerolend position, zerolend health factor."
version: "0.1.0"
author: "skylavis-sky"
tags:
  - zerolend
  - lending
  - aave-fork
  - zksync
  - linea
  - blast
  - defi
---

# ZeroLend Skill

## Overview

ZeroLend is a decentralized lending protocol and verified Aave V3 fork. The audit (Mundus) confirms no ABI changes from Aave V3. This skill lets users supply assets to earn yield, borrow against collateral, manage health factors, and monitor positions — all via the `zerolend` binary and `onchainos` CLI.

**Supported chains:**

| Chain | Chain ID |
|-------|----------|
| zkSync Era | 324 |
| Linea | 59144 (default) |
| Blast | 81457 |

**Architecture:**
- Supply / Withdraw / Borrow / Repay / Set Collateral / Set E-Mode → `zerolend` binary constructs ABI calldata, submits via `onchainos wallet contract-call` directly to ZeroLend Pool
  - **Always ask the user to confirm before executing** any `wallet contract-call` that broadcasts a transaction on-chain.
- Supply / Repay first approve the ERC-20 token via `wallet contract-call` before the Pool call
  - **Show the user the approval details and ask for confirmation** before submitting the approve transaction.
- Claim Rewards → `onchainos defi collect --platform-id <id>` (platform-id from `defi positions`)
- Health Factor / Reserves / Positions → `zerolend` binary makes read-only `eth_call` via public RPC
- Pool address is always resolved at runtime via `PoolAddressesProvider.getPool()` — never hardcoded

## Do NOT use for
- Aave V3 on Ethereum/Optimism/Arbitrum (use aave-v3 skill instead)
- ZeroLend staking/governance (ZERO token operations)
- Chains not in the supported list (zkSync, Linea, Blast)

---

## Pre-flight Checks

Before executing any command, verify:

1. **Binary installed**: `zerolend --version` — if not found, instruct user to install the plugin
2. **Wallet connected**: `onchainos wallet status` — confirm logged in and active address is set
3. **Chain supported**: chain ID must be one of 324, 59144, 81457

If the wallet is not connected, output:
```
Please connect your wallet first: run `onchainos wallet login`
```

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| Supply / deposit / lend asset | `zerolend supply --asset <ADDRESS> --amount <AMOUNT>` |
| Withdraw / redeem aTokens | `zerolend withdraw --asset <SYMBOL> --amount <AMOUNT>` |
| Borrow asset | `zerolend borrow --asset <ADDRESS> --amount <AMOUNT>` |
| Repay debt | `zerolend repay --asset <ADDRESS> --amount <AMOUNT>` |
| Repay all debt | `zerolend repay --asset <ADDRESS> --all` |
| Check health factor | `zerolend health-factor` |
| View positions | `zerolend positions` |
| List reserve rates / APYs | `zerolend reserves` |
| Enable/disable collateral | `zerolend set-collateral --asset <ADDRESS> --enable <true/false>` |
| Set E-Mode | `zerolend set-emode --category <ID>` |
| Claim rewards | `zerolend claim-rewards` |

**Global flags (always available):**
- `--chain <CHAIN_ID>` — target chain (default: 59144 Linea)
- `--from <ADDRESS>` — wallet address (defaults to active onchainos wallet)
- `--dry-run` — simulate without broadcasting

---

## Health Factor Rules

The health factor (HF) is a numeric value representing the safety of a borrowing position:
- **HF >= 1.1** → `safe` — position is healthy
- **1.05 <= HF < 1.1** → `warning` — elevated liquidation risk
- **HF < 1.05** → `danger` — high liquidation risk

**Rules:**
- **Always** check health factor before borrow or set-collateral operations
- **Warn** when post-action estimated HF < 1.1
- **Block** (require explicit user confirmation) when current HF < 1.05
- **Never** execute borrow if HF would drop below 1.0

To check health factor:
```bash
zerolend --chain 59144 health-factor --from 0xYourAddress
```

---

## Commands

### supply — Deposit to earn interest

**Trigger phrases:** "supply to zerolend", "deposit to zerolend", "lend on zerolend", "earn yield on zerolend", "zerolend supply", "zerolend deposit"

**Usage:**
```bash
zerolend --chain 59144 supply --asset USDC --amount 1000
zerolend --chain 59144 --dry-run supply --asset USDC --amount 1000
zerolend --chain 324 supply --asset WETH --amount 0.5
```

**Key parameters:**
- `--asset` — token symbol (e.g. USDC, WETH) or ERC-20 address
- `--amount` — human-readable amount (e.g. 1000 for 1000 USDC)

**Expected output:**
```json
{
  "ok": true,
  "approveTxHash": "0xabc...",
  "supplyTxHash": "0xdef...",
  "asset": "USDC",
  "tokenAddress": "0x176211...",
  "amount": 1000,
  "poolAddress": "0x..."
}
```

---

### withdraw — Redeem aTokens

**Trigger phrases:** "withdraw from zerolend", "redeem zerolend", "take out from zerolend", "zerolend withdraw"

**Usage:**
```bash
zerolend --chain 59144 withdraw --asset USDC --amount 500
zerolend --chain 59144 withdraw --asset USDC --all
zerolend --chain 81457 withdraw --asset WETH --all
```

**Key parameters:**
- `--asset` — token symbol or ERC-20 address
- `--amount` — partial withdrawal amount
- `--all` — withdraw the full balance

**Expected output:**
```json
{
  "ok": true,
  "txHash": "0xabc...",
  "asset": "USDC",
  "amount": "500"
}
```

---

### borrow — Borrow against collateral

**Trigger phrases:** "borrow from zerolend", "borrow on linea", "borrow on blast", "borrow on zksync via zerolend", "zerolend borrow"

**IMPORTANT:** Always run with `--dry-run` first, then confirm with user before executing.

**Usage:**
```bash
# Always dry-run first
zerolend --chain 59144 --dry-run borrow --asset 0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e --amount 0.1
# Then execute after user confirms
zerolend --chain 59144 borrow --asset 0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e --amount 0.1
# On zkSync
zerolend --chain 324 --dry-run borrow --asset 0x5AEa5775959fBC2557Cc8789bC1bf90A239D9a91 --amount 0.1
```

**Key parameters:**
- `--asset` — ERC-20 contract address (checksummed). Borrow and repay require the address, not symbol.
- `--amount` — human-readable amount in token units (0.1 WETH = `0.1`)

**Notes:**
- Interest rate mode is always 2 (variable) — stable rate is deprecated
- Pool address is resolved at runtime from PoolAddressesProvider; never hardcoded
- zkSync Era (chain 324) uses native account abstraction — verify `onchainos wallet contract-call --chain 324` is supported before live write operations (always confirm with user before executing)

**Expected output:**
```json
{
  "ok": true,
  "txHash": "0xabc...",
  "asset": "0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e",
  "borrowAmount": 0.1,
  "currentHealthFactor": "1.8500",
  "healthFactorStatus": "safe",
  "availableBorrowsUSD": "1240.50"
}
```

---

### repay — Repay borrowed debt

**Trigger phrases:** "repay zerolend loan", "pay back zerolend debt", "zerolend repay"

**IMPORTANT:** Always run with `--dry-run` first.

**Usage:**
```bash
# Repay specific amount
zerolend --chain 59144 --dry-run repay --asset 0x176211869cA2b568f2A7D4EE941E073a821EE1ff --amount 1000
# Repay all debt
zerolend --chain 59144 repay --asset 0x176211869cA2b568f2A7D4EE941E073a821EE1ff --all
```

**Key parameters:**
- `--asset` — ERC-20 contract address of the debt token
- `--amount` — partial repay amount
- `--all` — repay full outstanding balance (uses wallet balance, not uint256.max, to avoid revert)

**Notes:**
- ERC-20 approval is checked automatically; if insufficient, an approve tx is submitted first
- `--all` uses the wallet's actual token balance (NOT uint256.max) to avoid revert when accrued interest exceeds wallet balance

**Expected output:**
```json
{
  "ok": true,
  "txHash": "0xabc...",
  "asset": "0x176211...",
  "repayAmount": "all (1005230000)",
  "totalDebtBefore": "1005.23",
  "approvalExecuted": true
}
```

---

### health-factor — Check account health

**Trigger phrases:** "zerolend health factor", "zerolend liquidation risk", "check zerolend position"

**Usage:**
```bash
zerolend --chain 59144 health-factor
zerolend --chain 59144 health-factor --from 0xSomeAddress
zerolend --chain 81457 health-factor
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Linea",
  "healthFactor": "1.85",
  "healthFactorStatus": "safe",
  "totalCollateralUSD": "10000.00",
  "totalDebtUSD": "5400.00",
  "availableBorrowsUSD": "2000.00",
  "currentLiquidationThreshold": "82.50%",
  "loanToValue": "75.00%"
}
```

---

### reserves — List market rates and APYs

**Trigger phrases:** "zerolend interest rates", "zerolend supply rates", "zerolend borrow rates", "zerolend markets"

**Usage:**
```bash
# All reserves on Linea (default)
zerolend --chain 59144 reserves
# Filter by symbol
zerolend --chain 59144 reserves --asset USDC
# Filter by address
zerolend --chain 59144 reserves --asset 0x176211869cA2b568f2A7D4EE941E073a821EE1ff
# zkSync markets
zerolend --chain 324 reserves
# Blast markets
zerolend --chain 81457 reserves
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Linea",
  "chainId": 59144,
  "reserveCount": 8,
  "reserves": [
    {
      "underlyingAsset": "0x176211...",
      "supplyApy": "3.2500%",
      "variableBorrowApy": "5.1200%"
    }
  ]
}
```

---

### positions — View current positions

**Trigger phrases:** "my zerolend positions", "zerolend portfolio", "zerolend position"

**Usage:**
```bash
zerolend --chain 59144 positions
zerolend --chain 324 positions --from 0xSomeAddress
```

**Expected output:**
```json
{
  "ok": true,
  "chain": "Linea",
  "healthFactor": "1.85",
  "healthFactorStatus": "safe",
  "totalCollateralUSD": "10000.00",
  "totalDebtUSD": "5400.00",
  "positions": { ... }
}
```

---

### set-collateral — Enable or disable collateral

**Trigger phrases:** "disable collateral on zerolend", "use asset as collateral on zerolend"

**IMPORTANT:** Always check health factor first. Disabling collateral with outstanding debt may trigger liquidation.

**Usage:**
```bash
# Dry-run first
zerolend --chain 59144 --dry-run set-collateral --asset 0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e --enable false
# Execute after confirmation
zerolend --chain 59144 set-collateral --asset 0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e --enable false
```

---

### set-emode — Set efficiency mode

**Trigger phrases:** "enable emode on zerolend", "zerolend efficiency mode", "zerolend emode"

**E-Mode notes:**
- E-Mode category IDs are configured per ZeroLend deployment and may differ per chain
- Check ZeroLend UI (app.zerolend.xyz) for active categories on each chain
- Common categories: `0` = No E-Mode, `1` = Stablecoins, `2` = ETH-correlated

**Usage:**
```bash
zerolend --chain 59144 --dry-run set-emode --category 1
zerolend --chain 59144 set-emode --category 1
```

---

### claim-rewards — Claim accrued rewards

**Trigger phrases:** "claim zerolend rewards", "collect zerolend rewards"

**Usage:**
```bash
zerolend --chain 59144 claim-rewards
zerolend --chain 59144 --dry-run claim-rewards
```

---

## Asset Address Reference

For borrow and repay, use ERC-20 contract addresses. Confirmed ZeroLend-supported addresses:

### Linea (59144) — Primary chain
| Symbol | Address |
|--------|---------|
| USDC | 0x176211869cA2b568f2A7D4EE941E073a821EE1ff |
| WETH | 0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e |

### zkSync Era (324)
| Symbol | Address |
|--------|---------|
| USDC | 0x3355df6D4c9C3035724Fd0e3914dE96A5a83aaf4 |
| WETH | 0x5AEa5775959fBC2557Cc8789bC1bf90A239D9a91 |

### Blast (81457)
| Symbol | Address |
|--------|---------|
| USDC | 0x4300000000000000000000000000000000000003 |
| WETH | 0x4300000000000000000000000000000000000004 |

---

## Safety Rules

1. **Dry-run first**: Always simulate with `--dry-run` before any on-chain write
2. **Confirm before broadcast**: Show the user what will happen and wait for explicit confirmation
3. **Never borrow if HF < 1.5 without warning**: Explicitly warn user of liquidation risk
4. **Block at HF < 1.05**: Require explicit override from user before proceeding
5. **Full repay safety**: Use `--all` flag for full repay — avoids underpayment due to accrued interest
6. **Collateral warning**: Before disabling collateral, simulate health factor impact
7. **ERC-20 approval**: repay automatically handles approval; inform user if approval tx is included
8. **Pool address is never hardcoded**: Resolved at runtime from PoolAddressesProvider
9. **zkSync write ops**: Verify `onchainos wallet contract-call --chain 324` support before live write tests on zkSync (confirm with user before each transaction)

---

## Troubleshooting

| Error | Solution |
|-------|----------|
| `Could not resolve active wallet` | Run `onchainos wallet login` |
| `Unsupported chain ID` | Use chain 324, 59144, or 81457 |
| `No borrow capacity available` | Supply collateral first or repay existing debt |
| `eth_call RPC error` | RPC endpoint may be rate-limited; retry (fallback: linea.drpc.org, zksync.drpc.org, blast.drpc.org) |
| `contract-call --chain 324 not supported` | zkSync write ops blocked; read-only ops (health-factor, reserves) still work |
