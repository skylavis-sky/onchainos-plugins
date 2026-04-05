# Rocket Pool Plugin Design

## Overview

Rocket Pool is a decentralised Ethereum liquid staking protocol. Users deposit ETH and receive rETH
(Rocket Pool ETH), a liquid staking token that increases in value relative to ETH as staking rewards
accumulate. Unlike rebasing tokens (stETH), rETH's balance stays constant while its exchange rate
increases. Node operators run Ethereum validators in "minipools" using a mix of user-deposited ETH
and their own bonded ETH.

## Contract Architecture

### RocketStorage (Registry)
- **Address**: `0x1d8f8f00cfa6758d7bE78336684788Fb0ee0Fa46`
- Central registry for all Rocket Pool contract addresses
- `getAddress(bytes32 key)` → resolves current addresses dynamically
- Keys are `keccak256("contract.address<contractName>")` (no space between "address" and name)

### RocketDepositPool
- **Resolved Address**: `0xce15294273cfb9d9b628f4d61636623decdf4fdc`
- Main deposit contract: users send ETH here to receive rETH
- Key functions:
  - `deposit()` payable — `0xd0e30db0` — deposit ETH, receive rETH
  - `getBalance()` — `0x12065fe0` — current ETH balance in deposit pool

### RocketTokenRETH (rETH ERC20)
- **Resolved Address**: `0xae78736cd615f374d3085123a210448e74fc6393`
- rETH ERC20 token contract
- Key functions:
  - `getExchangeRate()` → `uint256` — `0xe6aa216c` — rETH/ETH rate (1e18 = 1 ETH per 1 rETH)
  - `burn(uint256 rethAmount)` — `0x42966c68` — burn rETH to receive ETH
  - `balanceOf(address)` — `0x70a08231` — rETH balance
  - `totalSupply()` — `0x18160ddd` — total rETH in circulation

### RocketNetworkBalances
- **Resolved Address**: `0x1d9f14c6bfd8358b589964bad8665add248e9473`
- Protocol-wide balance tracking
- Key functions:
  - `getTotalETHBalance()` — `0x964d042c` — total ETH staked in protocol
  - `getTotalRETHSupply()` — `0xc4c8d0ad` — total rETH supply tracked by network

### RocketNodeManager
- **Resolved Address**: `0xcf2d76a7499d3acb5a22ce83c027651e8d76e250`
- `getNodeCount()` — `0x39bf397e` — number of registered node operators

### RocketMinipoolManager
- **Resolved Address**: `0xe54b8c641fd96de5d6747f47c19964c6b824d62c`
- `getMinipoolCount()` — `0xae4d0bed` — total minipool count

## Storage Key Hashes

| Contract Name | keccak256 key |
|---|---|
| rocketDepositPool | `0x65dd923ddfc8d8ae6088f80077201d2403cbd565f0ba25e09841e2799ec90bb2` |
| rocketTokenRETH | `0xe3744443225bff7cc22028be036b80de58057d65a3fdca0a3df329f525e31ccc` |
| rocketNetworkBalances | `0x7630e125f1c009e5fc974f6dae77c6d5b1802979b36e6d7145463c21782af01e` |
| rocketNodeManager | `0xaf00be55c9fb8f543c04e0aa0d70351b880c1bfafffd15b60065a4a50c85ec94` |
| rocketMinipoolManager | (computed as needed) |

## Function Selectors (verified via `cast sig`)

| Function | Selector |
|---|---|
| `deposit()` | `0xd0e30db0` |
| `burn(uint256)` | `0x42966c68` |
| `getExchangeRate()` | `0xe6aa216c` |
| `getAddress(bytes32)` | `0x21f8a721` |
| `getBalance()` | `0x12065fe0` |
| `getTotalETHBalance()` | `0x964d042c` |
| `getTotalRETHSupply()` | `0xc4c8d0ad` |
| `getNodeCount()` | `0x39bf397e` |
| `getMinipoolCount()` | `0xae4d0bed` |
| `balanceOf(address)` | `0x70a08231` |
| `totalSupply()` | `0x18160ddd` |

## Operations

### `rate` (read)
Query the current ETH/rETH exchange rate from RocketTokenRETH.getExchangeRate().
Returns: how many ETH wei 1 rETH is worth (e.g. 1.16 ETH/rETH currently).

### `apy` (read)
Compute estimated APY from the exchange rate growth. Since the exchange rate is deterministic,
we use the Rocket Pool API (`https://api.rocketpool.net/api/apr`) for the current APY, or
fall back to computing from the on-chain rate vs. a known baseline.

### `stats` (read)
Protocol-level statistics:
- Total ETH staked (TVL) from RocketNetworkBalances.getTotalETHBalance()
- Total rETH supply from RocketTokenRETH.totalSupply()
- Node count from RocketNodeManager.getNodeCount()
- Minipool count from RocketMinipoolManager.getMinipoolCount()
- Current exchange rate

### `positions` (read)
User's rETH balance from RocketTokenRETH.balanceOf(address), shown in rETH and ETH equivalent.

### `stake` (write, payable)
Deposit ETH into RocketDepositPool.deposit() to receive rETH.
- Minimum deposit: 0.01 ETH (enforced by protocol)
- The protocol computes how much rETH to mint based on current exchange rate
- `--amt <wei>` passed to onchainos for ETH value

### `unstake` (write)
Burn rETH via RocketTokenRETH.burn(uint256) to receive ETH.
- Protocol must have sufficient ETH in the deposit pool or staking pool
- The amount of ETH received is: `rethAmount * exchangeRate / 1e18`

## Architecture Notes

- Contract addresses are resolved **dynamically** via RocketStorage at runtime
- This ensures the plugin works even if Rocket Pool upgrades contract addresses
- All read operations use direct JSON-RPC eth_call to `https://ethereum.publicnode.com`
- All write operations use `onchainos wallet contract-call --chain 1 --to <ADDR> --input-data <HEX> --force`

## Chain Support

- Ethereum Mainnet (chain ID: 1) only
- rETH is not natively available on L2 via this protocol (bridges exist but out of scope)

## Live Data (verified 2026-04-05)
- Exchange rate: ~1.1608 ETH/rETH
- Deposit pool balance: ~12.93 ETH
- Total ETH staked: ~628,000 ETH
- Node count: 4,114
- Minipool count: 42,317
