---
name: dinero-pxeth
description: "Dinero pxETH liquid staking protocol. Deposit ETH to receive pxETH, then stake pxETH to earn auto-compounding yield as apxETH (ERC-4626 vault). Query rates and positions. Trigger phrases: deposit ETH dinero, stake pxETH, apxETH yield, dinero liquid staking, pxETH stake, redeem apxETH, pirex ETH, dinero APR, dinero positions. Chinese: 质押ETH到Dinero, pxETH质押, apxETH收益, Dinero以太坊质押, pirex质押"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Architecture

Dinero pxETH is a two-step liquid staking protocol on Ethereum mainnet by Redacted Cartel:
1. ETH → pxETH via `PirexEth.deposit()` (payable call) — ⚠️ currently paused
2. pxETH → apxETH via ERC-4626 `deposit()` (auto-compounding yield vault) — ✅ active

- **Write ops** (deposit, stake, redeem) → after user confirmation, submits via `onchainos wallet contract-call`
- **Read ops** (rates, positions) → direct `eth_call` via Ethereum public RPC; no confirmation needed

## Protocol Status

⚠️ The PirexEth main contract (`0xD664b74274DfEB538d9baC494F3a4760828B02b0`) is currently **paused**. ETH → pxETH deposits are not available. The apxETH vault (stake/redeem pxETH) remains fully operational.

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata
2. **Ask user to confirm** before executing on-chain
3. Execute only after explicit user approval
4. Report transaction hash and link to etherscan.io

---

## Commands

### `deposit` — Deposit ETH to receive pxETH

Deposit native ETH to receive liquid pxETH token via Dinero's PirexEth contract.

⚠️ **Currently paused** — PirexEth deposits are not accepting new ETH at this time.

**Parameters:**
- `--amount <float>` — Amount of ETH to deposit (e.g. `0.001`)
- `--compound` — Auto-compound directly to apxETH (default: false = receive pxETH)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)
- `--dry-run` — Preview calldata without broadcasting

**Example:**
```
dinero-pxeth deposit --amount 0.001 --chain 1
dinero-pxeth deposit --amount 0.001 --compound --chain 1 --dry-run
```

**Execution:**
1. Run `--dry-run` to preview the transaction
2. **Ask user to confirm** before proceeding on-chain
3. Checks PirexEth.paused() — returns error if paused
4. Calls `PirexEth.deposit(receiver, shouldCompound)` with `--amt <wei>` via `onchainos wallet contract-call`
5. Returns txHash and link to etherscan.io

---

### `stake` — Stake pxETH to receive yield-bearing apxETH

Deposit pxETH into the apxETH ERC-4626 vault to earn auto-compounding staking yield.

**Parameters:**
- `--amount <float>` — Amount of pxETH to stake (e.g. `0.001`)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)
- `--dry-run` — Preview calldata without broadcasting

**Example:**
```
dinero-pxeth stake --amount 0.001 --chain 1
dinero-pxeth stake --amount 0.001 --chain 1 --dry-run
```

**Execution (two-step):**
1. Run `--dry-run` to preview both approve and deposit calldata
2. **Ask user to confirm** before proceeding on-chain
3. Step 1: ERC-20 `approve(apxETH, amount)` on pxETH token via `onchainos wallet contract-call`
4. Step 2: ERC-4626 `deposit(assets, receiver)` on apxETH vault via `onchainos wallet contract-call`
5. Returns txHash for deposit and link to etherscan.io

---

### `redeem` — Redeem apxETH to receive pxETH

Redeem apxETH shares from the ERC-4626 vault to receive pxETH back.

**Parameters:**
- `--amount <float>` — Amount of apxETH to redeem (e.g. `0.001`)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)
- `--dry-run` — Preview calldata without broadcasting

**Example:**
```
dinero-pxeth redeem --amount 0.001 --chain 1
dinero-pxeth redeem --amount 0.001 --chain 1 --dry-run
```

**Execution:**
1. Run `--dry-run` to preview the transaction
2. **Ask user to confirm** before proceeding on-chain
3. Calls ERC-4626 `redeem(shares, receiver, owner)` via `onchainos wallet contract-call`
4. Returns txHash and received pxETH amount

---

### `rates` — Query apxETH exchange rate and vault statistics

Get current apxETH exchange rate, vault TVL, and protocol status. All data fetched on-chain.

**Parameters:** None

**Example:**
```
dinero-pxeth rates
```

**Execution:**
1. Calls `convertToAssets(1e18)` on apxETH for exchange rate
2. Calls `totalAssets()` on apxETH for total pxETH locked
3. Checks `PirexEth.paused()` for protocol status

**Output fields:**
- `apxeth_per_pxeth` — How much pxETH 1 apxETH can be redeemed for
- `total_assets_pxeth` — Total pxETH deposited in apxETH vault
- `total_apxeth_supply` — Total apxETH shares outstanding
- `pirexeth_deposit_paused` — Whether new ETH deposits are paused
- `protocol_status` — Human-readable status message

---

### `positions` — Query pxETH and apxETH holdings

Get pxETH and apxETH balances for a wallet.

**Parameters:**
- `--address <addr>` — Wallet address to query (defaults to logged-in wallet)
- `--chain <id>` — Chain ID (default: `1`, Ethereum mainnet only)

**Example:**
```
dinero-pxeth positions
dinero-pxeth positions --address 0xabc...
```

**Execution:**
1. Calls `balanceOf(address)` on pxETH and apxETH contracts
2. Calls `convertToAssets(apxeth_balance)` to compute underlying pxETH value

---

## Contract Addresses (Ethereum Mainnet)

| Contract | Address | Status |
|----------|---------|--------|
| PirexEth (main) | `0xD664b74274DfEB538d9baC494F3a4760828B02b0` | ⚠️ Paused |
| pxETH token | `0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6` | ✅ Active |
| apxETH vault | `0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6` | ✅ Active |
