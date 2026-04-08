---
name: lido
description: "Lido liquid staking plugin — stake ETH to get stETH, wrap/unwrap wstETH, request and claim ETH withdrawals. Trigger phrases: stake ETH lido, lido staking, stake to lido, lido liquid staking, lido stETH, convert stETH to wstETH, wrap stETH, unwrap wstETH, request lido withdrawal, claim lido ETH, lido withdraw."
version: "0.1.0"
author: "skylavis-sky"
tags:
  - staking
  - liquid-staking
  - ethereum
  - steth
  - wsteth
---

## Do NOT use for...

- Staking on protocols other than Lido (e.g. Rocket Pool, Frax, EigenLayer)
- General ERC-20 token swaps or DEX trading
- Lending, borrowing, or yield farming on Morpho/Aave/Compound
- Chains other than Ethereum (1), Arbitrum (42161), Base (8453), or Optimism (10)
- Claiming Merkl or protocol rewards unrelated to Lido staking

## Architecture

- Read ops (get-position, get-apr, get-withdrawal-status) → direct `eth_call` via public RPC and Lido REST API; no confirmation needed
- Write ops (stake, wrap, unwrap, request-withdrawal, claim-withdrawal) → after user confirmation, submits via `onchainos wallet contract-call`

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview the operation and calldata
2. **Ask user to confirm** the transaction details before executing on-chain
3. Execute only after explicit user approval
4. Report the transaction hash and outcome

---

## Commands

### get-apr — Query current stETH staking APR

Fetches the 7-day SMA APR from Lido's official REST API.

```
lido get-apr
```

No confirmation needed (read-only).

**Example output:**
```json
{
  "ok": true,
  "data": {
    "smaApr": 3.8,
    "description": "7-day moving average APR for stETH liquid staking on Lido",
    "note": "Lido charges a 10% protocol fee on staking rewards"
  }
}
```

---

### get-position — Query stETH/wstETH positions

Queries stETH balance on Ethereum plus wstETH balances on Ethereum, Arbitrum, Base, and Optimism. Also shows the current exchange rate and APR.

```
lido get-position [--from <wallet>] [--chain <chain_id>]
```

No confirmation needed (read-only).

**Parameters:**
- `--from` — wallet address (auto-resolved from onchainos if omitted)
- `--chain` — chain ID to filter wstETH query (0 = all chains, default: 1)

---

### get-withdrawal-status — Query withdrawal request status

Checks whether withdrawal request(s) are finalized and ready to claim.

```
lido get-withdrawal-status --request-ids <id1>,<id2>
```

No confirmation needed (read-only).

**Parameters:**
- `--request-ids` — comma-separated withdrawal NFT request IDs

**Status values:** `pending`, `ready_to_claim`, `claimed`

---

### stake — Stake ETH to get stETH

Stakes ETH via Lido's `submit(address)` function on Ethereum mainnet. Returns stETH at approximately 1:1 ratio. Lido charges a 10% fee on staking rewards.

```
lido stake --amount <wei> [--from <wallet>] [--dry-run]
```

**Parameters:**
- `--amount` — ETH amount in wei (e.g. `1000000000000000000` = 1 ETH)
- `--from` — wallet address
- `--dry-run` — preview calldata without broadcasting

**Pre-checks:**
1. Verify staking is not paused (`getCurrentStakeLimit() > 0`)
2. Display current APR and expected stETH output
3. **Ask user to confirm** before submitting the stake transaction
4. Execute: `onchainos wallet contract-call` → Lido.submit(address(0))

**Example:**
```
lido stake --amount 1000000000000000000 --from 0xYourWallet
```

---

### wrap — Convert stETH to wstETH

Wraps stETH into the non-rebasing wstETH token via the wstETH contract on Ethereum. Useful for DeFi integrations.

```
lido wrap --amount <stETH_wei> [--from <wallet>] [--dry-run]
```

**Parameters:**
- `--amount` — stETH amount in wei to wrap
- `--from` — wallet address
- `--dry-run` — preview without broadcasting

**Steps:**
1. Check stETH balance ≥ amount
2. Check and set stETH allowance for wstETH contract if needed
3. **Ask user to confirm** approve transaction (if needed)
4. Execute approve: `onchainos wallet contract-call` → stETH.approve(wstETH, amount)
5. **Ask user to confirm** wrap transaction
6. Execute wrap: `onchainos wallet contract-call` → wstETH.wrap(amount)

---

### unwrap — Convert wstETH back to stETH

Unwraps wstETH back to stETH. Supported on Ethereum (chain 1), Arbitrum (42161), Base (8453), and Optimism (10).

```
lido unwrap --amount <wstETH_wei> [--from <wallet>] [--chain <chain_id>] [--dry-run]
```

**Parameters:**
- `--amount` — wstETH amount in wei to unwrap
- `--from` — wallet address
- `--chain` — chain ID (default: 1)
- `--dry-run` — preview without broadcasting

**Steps:**
1. Check wstETH balance ≥ amount on the target chain
2. **Ask user to confirm** the unwrap transaction
3. Execute: `onchainos wallet contract-call` → wstETH.unwrap(amount)

---

### request-withdrawal — Request ETH withdrawal (stETH → ETH)

Initiates a withdrawal request on Ethereum mainnet. Creates a withdrawal NFT (ERC-721). Typically takes 1-5 days to finalize.

Maximum 1000 stETH per request. Larger amounts are automatically split.

```
lido request-withdrawal --amount <stETH_wei> [--from <wallet>] [--dry-run]
```

**Parameters:**
- `--amount` — stETH amount in wei to withdraw
- `--from` — wallet address
- `--dry-run` — preview without broadcasting

**Steps:**
1. Verify stETH balance ≥ amount
2. Check existing stETH allowance for WithdrawalQueue
3. Display estimated wait time (typically 1-5 days)
4. **Ask user to confirm** approve transaction (if allowance insufficient)
5. Execute approve: `onchainos wallet contract-call` → stETH.approve(WithdrawalQueue, amount)
6. **Ask user to confirm** withdrawal request submission
7. Execute: `onchainos wallet contract-call` → WithdrawalQueue.requestWithdrawals(amounts, owner)
8. Track with `get-withdrawal-status --request-ids <id>` until `isFinalized: true`
9. Claim ETH with `claim-withdrawal --request-ids <id>` once finalized

**Warning:** Withdrawal is a 2-step process. You must call `claim-withdrawal` after the request is finalized.

---

### claim-withdrawal — Claim finalized ETH withdrawal

Claims ETH from finalized withdrawal request(s). Only callable after `isFinalized: true`.

```
lido claim-withdrawal --request-ids <id1>,<id2> [--from <wallet>] [--dry-run]
```

**Parameters:**
- `--request-ids` — comma-separated withdrawal NFT request IDs to claim
- `--from` — wallet address
- `--dry-run` — preview without broadcasting

**Steps:**
1. Query `getLastCheckpointIndex()` from WithdrawalQueue
2. Call `findCheckpointHints(requestIds, 1, lastCheckpoint)` to get required hints
3. **Ask user to confirm** before claiming ETH
4. Execute: `onchainos wallet contract-call` → WithdrawalQueue.claimWithdrawals(requestIds, hints)

---

## Supported Chains

| Chain | ID | Operations |
|-------|-----|-----------|
| Ethereum | 1 | All operations (stake, get-position, wrap, unwrap, request-withdrawal, claim-withdrawal) |
| Arbitrum | 42161 | get-position (wstETH), unwrap |
| Base | 8453 | get-position (wstETH), unwrap |
| Optimism | 10 | get-position (wstETH), unwrap |

## Key Contracts (Ethereum Mainnet)

| Contract | Address |
|----------|---------|
| Lido / stETH (proxy) | `0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84` |
| wstETH | `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0` |
| WithdrawalQueueERC721 | `0x889edC2eDab5f40e902b864aD4d7AdE8E412F9B1` |

Source: https://docs.lido.fi/deployed-contracts/

## Token Notes

- **stETH** is a rebasing token — balance increases daily as staking rewards accrue
- **wstETH** is non-rebasing — share count is fixed, but exchange rate to stETH increases daily
- Both represent the same underlying staked ETH position
