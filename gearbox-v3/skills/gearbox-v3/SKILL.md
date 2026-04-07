---
name: gearbox-v3
description: "Open and manage leveraged Credit Accounts on Gearbox V3 (Arbitrum, Ethereum). Trigger phrases: gearbox open account, gearbox leverage, gearbox credit account, gearbox borrow, gearbox collateral, gearbox close account, leveraged yield gearbox."
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

# Gearbox V3 Skill

## Overview

Gearbox V3 is a leverage protocol that lets users open Credit Accounts and borrow funds to amplify their DeFi positions. This skill manages Credit Account lifecycle: open, add collateral, withdraw collateral, and close.

**Supported chains:**

| Chain | Chain ID |
|-------|----------|
| Arbitrum One | 42161 (default) |
| Ethereum Mainnet | 1 |

**Default Credit Manager for testing:** Trade USDC Tier 2 (Arbitrum)
- CreditFacade: `0x3974888520a637ce73bdcb2ee28a396f4b303876`
- CreditManager: `0xb780dd9cec259a0bbf7b32587802f33730353e86`
- Minimum debt: 1,000 USDC | Maximum debt: 20,000 USDC

---

## Minimum Debt Requirement

**CRITICAL:** Every Gearbox Credit Manager enforces a minimum borrow amount (`minDebt`).
Opening an account requires borrowing at least this amount. The contract will revert if the borrow is below minimum.

| Credit Manager | Underlying | Min Debt | Max Debt |
|---------------|-----------|---------|---------|
| Trade USDC Tier 2 (recommended) | USDC | 1,000 USDC | 20,000 USDC |
| Trade USDC Tier 1 | USDC | 20,000 USDC | 400,000 USDC |
| Trade USDC.e Tier 2 | USDC.e | 5,000 USDC.e | 25,000 USDC.e |
| Trade USDC.e Tier 1 | USDC.e | 5,000 USDC.e | 100,000 USDC.e |
| Trade WETH Tier 2 | WETH | 0.35 WETH | 7 WETH |
| Trade WETH Tier 1 | WETH | 7 WETH | 150 WETH |

---

## Pre-flight Checks

Before executing any command:

1. **Binary installed**: `gearbox-v3 --version`
2. **Wallet connected**: `onchainos wallet status`
3. **Chain supported**: 42161 (Arbitrum) recommended; 1 (Ethereum) for mainnet pools

---

## Command Routing Table

| User Intent | Command |
|-------------|---------|
| List all Gearbox Credit Managers and limits | `gearbox-v3 get-pools --chain 42161` |
| Check my Credit Accounts | `gearbox-v3 get-account --chain 42161` |
| Open leveraged account | `gearbox-v3 open-account --facade <CF> --manager <CM> --token USDC --token-addr <USDC> --collateral 1000 --borrow 2000` |
| Add collateral | `gearbox-v3 add-collateral --facade <CF> --manager <CM> --account <CA> --token USDC --token-addr <USDC> --amount 500` |
| Withdraw partial collateral | `gearbox-v3 withdraw --facade <CF> --account <CA> --token USDC --token-addr <USDC> --amount 200` |
| Withdraw all collateral | `gearbox-v3 withdraw --facade <CF> --account <CA> --token USDC --token-addr <USDC>` |
| Close account (repay + withdraw all) | `gearbox-v3 close-account --facade <CF> --account <CA> --underlying <USDC_ADDR>` |

**Address shortcuts for Arbitrum USDC Tier 2:**
- `<CF>` = `0x3974888520a637ce73bdcb2ee28a396f4b303876` (CreditFacade)
- `<CM>` = `0xb780dd9cec259a0bbf7b32587802f33730353e86` (CreditManager)
- `<USDC>` = `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` (native USDC on Arbitrum)

---

## Example: Open a 3x Leveraged USDC Position

```
# Deposit 1000 USDC, borrow 2000 USDC = 3x leverage, total position 3000 USDC
gearbox-v3 open-account \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --token USDC \
  --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --collateral 1000 \
  --borrow 2000
```

This executes two transactions:
1. `USDC.approve(CreditManagerV3, 1000e6)` — approve to manager (NOT facade)
2. `openCreditAccount(wallet, [increaseDebt(2000e6), addCollateral(USDC, 1000e6)], 0)`

---

## Example: Dry Run (Preview Calldata)

```
gearbox-v3 open-account --dry-run \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  ...
```

Dry run prints the ABI-encoded calldata without broadcasting.

---

## Health Factor Warning

- Health Factor (HF) = weighted collateral value / total debt
- **HF < 1.0** = position is liquidatable by third parties
- **HF < 1.1** = warning zone — add collateral or repay debt
- Withdrawals that would push HF below 1.0 are rejected by the contract

---

## Close Account Limitations

The `close-account` command uses `decreaseDebt(MAX) + withdrawCollateral(MAX)`.

**Requirement:** You must have enough underlying token (e.g. USDC) in your **external wallet** to repay the outstanding debt (principal + accrued borrow interest).

If you do not have enough underlying:
1. Add more collateral first
2. Perform an internal swap within the multicall to convert collateral to underlying
   (this requires adapter-based swaps — out of scope for v0.1)

---

## Known Limitations (v0.1)

- **Underlying token collateral only.** Non-underlying collateral (e.g. WBTC in a USDC account) requires `updateQuota()` calls to count toward health factor. Multi-token collateral management is not supported.
- **No internal swaps.** The `close-account` flow does not support liquidating collateral positions internally. External funds are required for debt repayment.
- **No quota management.** Adding non-underlying tokens to a Credit Account provides zero health factor value without a quota update.
- **Arbitrum only for practical use.** Ethereum mainnet Credit Manager addresses are not included in this version.
- **Address freshness.** Contract addresses are from block 239832594 (Aug 2024). Run `get-pools` to see current debt limits; core addresses (DataCompressor, known facades) are stable.

---

## Do NOT Use For

- Executing internal swaps within Credit Accounts (requires adapter-specific multicall steps)
- Managing multiple collateral token quotas
- Liquidating other users' positions
- Gearbox V1/V2 contracts (different interface)
- Any protocol on chains other than Arbitrum (chain 42161) and Ethereum (chain 1)
