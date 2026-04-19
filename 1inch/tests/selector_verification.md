# Selector Verification -- 1inch

1inch integration uses the 1inch Aggregation API v6 which returns pre-built calldata.
No manual ABI encoding is performed -- the plugin broadcasts the calldata exactly as returned by the API.

## ERC-20 Selectors (for reference)

| Selector | Canonical Signature | Usage |
|----------|--------------------|----|
| `0x095ea7b3` | `approve(address,uint256)` | Encoded by 1inch API in /approve/transaction response |
| `0xdd62ed3e` | `allowance(address,address)` | Queried indirectly via 1inch /approve/allowance endpoint |

Verification method: `cast sig "approve(address,uint256)"` = `0x095ea7b3`

## 1inch Router V6

**Address:** `0x111111125421cA6dc452d289314280a0f8842A65`

Deployed at the same address on:
- Ethereum (1)
- Arbitrum (42161)
- Base (8453)
- BSC (56)
- Polygon (137)

## Integration Notes

- The plugin does NOT manually encode any calldata.
- `/swap` endpoint returns `tx.data` (complete calldata), `tx.to` (router address), and `tx.value` (ETH in wei).
- `/approve/transaction` returns `data` (approve calldata) and `to` (token contract address).
- All calldata is broadcast via `onchainos wallet contract-call --input-data <data> --to <to> --force`.

## API Key

Set `ONEINCH_API_KEY` environment variable with a key from https://portal.1inch.dev.
Defaults to `demo` (rate-limited) if unset.
