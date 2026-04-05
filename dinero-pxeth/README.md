# dinero-pxeth

Dinero pxETH liquid staking plugin for onchainos. Deposit ETH to receive pxETH, then stake pxETH to earn yield as apxETH (ERC-4626 auto-compounding vault).

## Commands

- `deposit` — Deposit ETH to receive pxETH (via PirexEth)
- `stake` — Stake pxETH to receive yield-bearing apxETH (ERC-4626)
- `redeem` — Redeem apxETH back to pxETH
- `rates` — Query apxETH APR and exchange rate
- `positions` — Query pxETH and apxETH holdings

## Supported Chains

- Ethereum mainnet (chain ID: 1)

## Protocol Status

⚠️ The PirexEth main contract (ETH → pxETH deposits) is currently paused. The apxETH vault (pxETH → apxETH stake/unstake) remains operational.
