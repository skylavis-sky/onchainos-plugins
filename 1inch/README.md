# 1inch

1inch aggregation protocol plugin for the OKX onchainos plugin store. Swap tokens at the best rates across 200+ DEXs on Ethereum, Arbitrum, Base, BSC, and Polygon.

## Commands

- `get-quote` -- Get best swap quote (read-only, no gas)
- `swap` -- Swap tokens via 1inch AggregationRouterV6
- `get-allowance` -- Check current ERC-20 allowance for the 1inch router
- `approve` -- Approve an ERC-20 token for use by the 1inch router

## Supported Chains

| Chain | Chain ID |
|-------|----------|
| Ethereum | 1 |
| Arbitrum | 42161 |
| Base | 8453 |
| BSC | 56 |
| Polygon | 137 |

## Setup

Set your 1inch API key (obtain at https://portal.1inch.dev):

```bash
export ONEINCH_API_KEY=your_api_key_here
```

If unset, the plugin defaults to the `demo` key (rate-limited).

## License

MIT
