# Selector Verification — dydx-v4

## EVM operations (Ethereum mainnet deposit bridge)

| Selector | Function | Contract | Verification |
|----------|----------|----------|-------------|
| `0x1d45e29c` | `bridge(uint256,bytes,bytes)` | WrappedEthereumDydxToken `0x46b2DeAe6efF3011008EA27EA36b7c27255ddFA9` | `keccak256("bridge(uint256,bytes,bytes)")[:4]` — confirmed via pycryptodome: `0x1d45e29c` |

## Verification method

```python
from Crypto.Hash import keccak as _keccak
k = _keccak.new(digest_bits=256)
k.update(b'bridge(uint256,bytes,bytes)')
print(k.hexdigest()[:8])  # => 1d45e29c
```

Design doc also notes:
```
bridge(uint256,bytes,bytes) -> keccak256[:4] = 0x1d45e29c  (computed via eth-hash, Keccak-256)
```

## Non-EVM operations

All other dYdX V4 operations use the Indexer REST API (read-only, no ABI encoding).
Order placement (place-order) requires Cosmos gRPC — not supported by onchainos.
These commands are implemented as informational stubs.
