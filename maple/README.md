# Maple Finance Plugin

Maple Finance institutional lending protocol integration for onchainos.

Deposit USDC or USDT into Maple's syrup pool vaults to earn yield from institutional lending.

## Supported Chains

- Ethereum mainnet (chain ID: 1)

## Commands

| Command | Description |
|---------|-------------|
| `pools` | List all syrup pools with TVL |
| `positions` | Show your lending positions |
| `rates` | Show pool exchange rates |
| `deposit` | Deposit USDC/USDT into a pool |
| `withdraw` | Request redemption from a pool |

## Usage

```bash
# List pools
maple pools --chain 1

# Check positions
maple positions --chain 1

# Deposit 0.01 USDC (dry-run first)
maple deposit --pool usdc --amount 0.01 --chain 1 --dry-run
maple deposit --pool usdc --amount 0.01 --chain 1

# Request withdrawal
maple withdraw --pool usdc --chain 1 --dry-run
maple withdraw --pool usdc --chain 1
```

## Pool Addresses

| Pool | Address |
|------|---------|
| syrupUSDC | 0x80ac24aA929eaF5013f6436cdA2a7ba190f5Cc0b |
| syrupUSDT | 0x356B8d89c1e1239Cbbb9dE4815c39A1474d5BA7D |
