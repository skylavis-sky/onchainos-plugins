# Solv SolvBTC Plugin

Interact with Solv Protocol: mint liquid BTC (SolvBTC) by depositing WBTC, earn yield via xSolvBTC, and manage redemptions.

## Commands

- `get-nav` — Fetch current SolvBTC and xSolvBTC prices from DeFiLlama; Solv Protocol TVL
- `get-balance` — Query SolvBTC and xSolvBTC token balances
- `mint` — Deposit WBTC to receive SolvBTC (approve + RouterV2.deposit)
- `redeem` — Request SolvBTC → WBTC redemption (non-instant, creates ERC-3525 SFT claim ticket)
- `cancel-redeem` — Cancel a pending redemption request
- `wrap` — Wrap SolvBTC into yield-bearing xSolvBTC (Ethereum only)
- `unwrap` — Unwrap xSolvBTC back to SolvBTC with 0.05% fee (Ethereum only)

## Chains

- Arbitrum (42161) — SolvBTC mint/redeem
- Ethereum (1) — SolvBTC mint/redeem + xSolvBTC wrap/unwrap

## Usage

```
solv-solvbtc get-nav
solv-solvbtc get-balance --chain 42161
solv-solvbtc mint --amount 0.001 --chain 42161
solv-solvbtc wrap --amount 0.001
solv-solvbtc unwrap --amount 0.001
```
