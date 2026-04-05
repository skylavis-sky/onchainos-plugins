# Rocket Pool Selector Verification

All selectors verified using `cast sig` from Foundry.

## Verification Commands & Results

```bash
$ cast sig "deposit()"
0xd0e30db0

$ cast sig "burn(uint256)"
0x42966c68

$ cast sig "getExchangeRate()"
0xe6aa216c

$ cast sig "getAddress(bytes32)"
0x21f8a721

$ cast sig "getBalance()"
0x12065fe0

$ cast sig "getTotalETHBalance()"
0x964d042c

$ cast sig "getTotalRETHSupply()"
0xc4c8d0ad

$ cast sig "getNodeCount()"
0x39bf397e

$ cast sig "getMinipoolCount()"
0xae4d0bed

$ cast sig "balanceOf(address)"
0x70a08231

$ cast sig "totalSupply()"
0x18160ddd
```

## keccak256 Storage Keys (verified via `cast keccak`)

```bash
$ cast keccak "contract.addressrocketDepositPool"
0x65dd923ddfc8d8ae6088f80077201d2403cbd565f0ba25e09841e2799ec90bb2

$ cast keccak "contract.addressrocketTokenRETH"
0xe3744443225bff7cc22028be036b80de58057d65a3fdca0a3df329f525e31ccc

$ cast keccak "contract.addressrocketNetworkBalances"
0x7630e125f1c009e5fc974f6dae77c6d5b1802979b36e6d7145463c21782af01e

$ cast keccak "contract.addressrocketNodeManager"
0xaf00be55c9fb8f543c04e0aa0d70351b880c1bfafffd15b60065a4a50c85ec94

$ cast keccak "contract.addressrocketMinipoolManager"
0xe9dfec9339b94a131861a58f1bb4ac4c1ce55c7ffe8550e0b6ebcfde87bb012f
```

## On-Chain Address Resolution (verified 2026-04-05)

Queried via `eth_call` to RocketStorage (`0x1d8f8f00cfa6758d7bE78336684788Fb0ee0Fa46`):

| Contract | Resolved Address |
|---|---|
| RocketDepositPool | `0xce15294273cfb9d9b628f4d61636623decdf4fdc` |
| RocketTokenRETH | `0xae78736cd615f374d3085123a210448e74fc6393` |
| RocketNetworkBalances | `0x1d9f14c6bfd8358b589964bad8665add248e9473` |
| RocketNodeManager | `0xcf2d76a7499d3acb5a22ce83c027651e8d76e250` |
| RocketMinipoolManager | `0xe54b8c641fd96de5d6747f47c19964c6b824d62c` |

## Live Data Verification (2026-04-05)

```
getExchangeRate() on RocketTokenRETH:
  Result: 0x000000000000000000000000000000000000000000000000101c00ff0c18c7c1
  Decoded: 1160803899374356417 wei = 1.160804 ETH/rETH

getTotalETHBalance() on RocketNetworkBalances:
  Result: 0x00000000000000000000000000000000000000000000537b421a08077b893e29
  Decoded: ~627,000 ETH TVL

getNodeCount():
  Decoded: 4,114 nodes

getMinipoolCount():
  Decoded: 42,317 minipools

RocketDepositPool.getBalance():
  Decoded: 12.93 ETH available
```
