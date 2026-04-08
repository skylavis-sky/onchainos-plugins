---
name: solv-solvbtc
description: "Solv Protocol SolvBTC plugin. Trigger phrases: mint SolvBTC, deposit WBTC into Solv, redeem SolvBTC, cancel redemption, wrap SolvBTC into xSolvBTC, unwrap xSolvBTC, SolvBTC price, xSolvBTC NAV, Solv TVL, my SolvBTC balance, yield on BTC"
version: "0.1.0"
author: "skylavis-sky"
tags:
  - btc
  - yield
  - liquid-btc
  - arbitrum
  - ethereum
---

# solv-solvbtc

Interact with Solv Protocol SolvBTC: mint liquid BTC, redeem back to WBTC, and wrap into yield-bearing xSolvBTC.

## Overview

Solv Protocol issues SolvBTC -- a 1:1 BTC-backed ERC-20 token on Arbitrum and Ethereum.
Users deposit WBTC to receive SolvBTC, and can optionally wrap SolvBTC into xSolvBTC for
yield via Solv's basis trading and staking strategies (Ethereum mainnet only).

Token hierarchy:
  WBTC --[mint]--> SolvBTC --[wrap]--> xSolvBTC
  xSolvBTC --[unwrap]--> SolvBTC --[redeem (non-instant)]--> WBTC

**Always confirm with the user before executing any transaction that calls wallet contract-call.**
Show parameters and wait for explicit approval.

## Commands

### get-nav
Fetch current SolvBTC and xSolvBTC price from DeFiLlama, and Solv Protocol TVL.

Usage:
  solv-solvbtc get-nav

Example trigger: "What is the SolvBTC price?" / "Show Solv Protocol TVL" / "xSolvBTC NAV"

### get-balance
Query your SolvBTC and xSolvBTC token balances on a given chain.

Usage:
  solv-solvbtc get-balance [--chain <chain_id>]

Options:
  --chain   Chain ID: 42161 (Arbitrum, default) or 1 (Ethereum)

Example trigger: "How much SolvBTC do I have on Arbitrum?" / "my SolvBTC balance"

### mint
Deposit WBTC to receive SolvBTC via SolvBTCRouterV2. Two transactions: approve then deposit.
Default chain: Arbitrum (42161).

Usage:
  solv-solvbtc mint --amount <wbtc_amount> [--chain <chain_id>] [--dry-run]

Options:
  --amount  WBTC amount (human-readable, e.g. 0.001)
  --chain   Chain ID: 42161 (default) or 1
  --dry-run Simulate without broadcasting

Example trigger: "Mint 0.001 WBTC worth of SolvBTC" / "Deposit 0.005 WBTC into Solv"

### redeem
Submit a SolvBTC withdrawal request to get back WBTC.

IMPORTANT: Redemption is NOT instant. It creates an ERC-3525 SFT claim ticket.
WBTC is released only after the OpenFundMarket queue processes the request.
Use cancel-redeem to cancel a pending request.

Usage:
  solv-solvbtc redeem --amount <solvbtc_amount> [--chain <chain_id>] [--dry-run]

Options:
  --amount  SolvBTC amount to redeem
  --chain   Chain ID: 42161 (default) or 1
  --dry-run Simulate without broadcasting

Example trigger: "Redeem 0.001 SolvBTC back to WBTC"

### cancel-redeem
Cancel a pending SolvBTC redemption request and recover SolvBTC.

Usage:
  solv-solvbtc cancel-redeem --redemption-addr <address> --redemption-id <id> [--chain <chain_id>] [--dry-run]

Options:
  --redemption-addr  Redemption contract address (from the SFT ticket)
  --redemption-id    Redemption token ID
  --chain            Chain ID: 42161 (default) or 1
  --dry-run          Simulate without broadcasting

Example trigger: "Cancel my SolvBTC redemption request"

### wrap
Wrap SolvBTC into yield-bearing xSolvBTC via XSolvBTCPool.deposit().
Ethereum mainnet only. Instant, no fee.

Usage:
  solv-solvbtc wrap --amount <solvbtc_amount> [--dry-run]

Options:
  --amount  SolvBTC amount to wrap
  --dry-run Simulate without broadcasting

Example trigger: "Wrap 0.05 SolvBTC into xSolvBTC for yield" / "Get xSolvBTC yield"

### unwrap
Unwrap xSolvBTC back to SolvBTC via XSolvBTCPool.withdraw().
Ethereum mainnet only. Instant, 0.05% withdraw fee.

Usage:
  solv-solvbtc unwrap --amount <xsolvbtc_amount> [--dry-run]

Options:
  --amount  xSolvBTC amount to unwrap
  --dry-run Simulate without broadcasting

Example trigger: "Unwrap 0.05 xSolvBTC back to SolvBTC"

## Do NOT use for

Do NOT use for: direct BTC transactions, non-SolvBTC wrapped BTC, Ethereum staking, non-Bitcoin yield protocols

## Key Facts

- SolvBTC is 1:1 BTC-backed; minting is instant, redemption to WBTC is queued (non-instant)
- xSolvBTC NAV accrues over time via yield strategies; however, market price of xSolvBTC may trade below NAV on secondary markets — always verify the on-chain NAV via `get-nav` before trading
- xSolvBTC withdraw fee: 0.05% (5/10000)
- Arbitrum (42161): mint and redeem SolvBTC
- Ethereum (1): mint/redeem SolvBTC + wrap/unwrap xSolvBTC
- All transactions require --force (handled automatically by the binary)
- Both approve + deposit steps are executed; 3-second delay between them for nonce safety

## Supported Chains

| Chain     | Chain ID | Supported Operations                    |
|-----------|----------|-----------------------------------------|
| Arbitrum  | 42161    | mint, redeem, cancel-redeem, get-balance|
| Ethereum  | 1        | mint, redeem, wrap, unwrap, get-balance |

## Contract Addresses

### Arbitrum (42161)
- SolvBTC:    0x3647c54c4c2C65bC7a2D63c0Da2809B399DBBDC0
- WBTC:       0x2f2a2543B76A4166549F7aaB2e75Bef0aefC5B0f
- RouterV2:   0x92E8A4407FD1ae7a53a32f1f832184edF071080A

### Ethereum (1)
- SolvBTC:    0x7a56e1c57c7475ccf742a1832b028f0456652f97
- xSolvBTC:   0xd9d920aa40f578ab794426f5c90f6c731d159def
- WBTC:       0x2260FAC5E5542a773Aa44fBCfeDf7C193bc2C599
- RouterV2:   0x3d93B9e8F0886358570646dAd9421564C5fE6334
- XSolvBTCPool: 0xf394Aa7CFB25644e2A713EbbBE259B81F7c67c86

## Function Selectors

| Function | Selector |
|----------|----------|
| ERC-20 approve | 0x095ea7b3 |
| RouterV2 deposit | 0x672262e5 |
| RouterV2 withdrawRequest | 0xd2cfd97d |
| RouterV2 cancelWithdrawRequest | 0x42c7774b |
| XSolvBTCPool deposit | 0xb6b55f25 |
| XSolvBTCPool withdraw | 0x2e1a7d4d |
| ERC-20 balanceOf | 0x70a08231 |
