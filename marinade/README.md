# Marinade Finance Plugin

Marinade Finance liquid staking integration for onchainos.

## Features

- **rates** — Query mSOL/SOL exchange rate and staking APY
- **positions** — Query mSOL holdings and SOL-equivalent value
- **stake** — Stake SOL to receive mSOL (via Jupiter routing)
- **unstake** — Unstake mSOL back to SOL (via Jupiter routing)

## Chain Support

Solana mainnet (chain ID: 501)

## Usage

```bash
# Check staking rates
marinade rates

# Check mSOL balance
marinade positions

# Stake 0.001 SOL (dry run preview)
marinade stake --amount 0.001 --dry-run

# Stake 0.001 SOL (real transaction)
marinade stake --amount 0.001

# Unstake 0.001 mSOL
marinade unstake --amount 0.001
```

## Key Addresses

| Name | Address |
|------|---------|
| mSOL Mint | `mSoLzYCxHdYgdzU16g5QSh3i5K3z3KZK7ytfqcJm7So` |
| Marinade Program | `MarBmsSgKXdrN1egZf5sqe1TMai9K1rChYNDJgjq7aD` |
