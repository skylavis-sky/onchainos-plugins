# dydx-v4

dYdX V4 plugin for onchainos — decentralised perpetuals exchange on a Cosmos appchain.

## What this plugin does

- Query all perpetual markets (price, volume, open interest)
- View L2 orderbook for any market
- Check open positions for a dYdX address
- Check account equity and free collateral
- Bridge DYDX tokens from Ethereum mainnet to the dYdX chain

## What it does NOT do

- Order placement and cancellation require Cosmos gRPC (`MsgPlaceOrder`, `MsgCancelOrder`),
  which is not supported by onchainos. Use the dYdX web app (https://dydx.trade) or
  the TypeScript SDK (@dydxprotocol/v4-client-js) for order operations.
- USDC deposits via Noble/IBC are Cosmos-native and not supported here.

## Commands

| Command | Description |
|---------|-------------|
| `dydx-v4 get-markets` | List all perpetual markets |
| `dydx-v4 get-orderbook --market BTC-USD` | L2 orderbook |
| `dydx-v4 get-positions --address dydx1...` | Open positions |
| `dydx-v4 get-balance --address dydx1...` | Account equity |
| `dydx-v4 deposit --amount 100 --dydx-address dydx1... --dry-run` | Bridge DYDX tokens |
| `dydx-v4 place-order --market BTC-USD --side buy --size 0.1 --price 70000` | Informational only |

## Installation

```bash
plugin-store install dydx-v4
```

## License

MIT
