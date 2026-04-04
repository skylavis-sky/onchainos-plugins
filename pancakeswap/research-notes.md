# PancakeSwap Plugin — Research Notes

## Integration Path Chosen: Direct On-Chain (ABI calldata)

### Why not SDK?
PancakeSwap has no Rust SDK. The official SDKs (`@pancakeswap/sdk`, `@pancakeswap/smart-router`, `@pancakeswap/v3-sdk`) are TypeScript-only and cannot be used in a Rust plugin without a subprocess wrapper, which would be fragile and slow.

### Why not the official pancakeswap-ai skill?
`github.com/pancakeswap/pancakeswap-ai` provides `pancakeswap-driver` and `pancakeswap-farming` plugins, but they work by generating **deep links for UI confirmation** — they do not perform autonomous on-chain execution. They are planning tools, not execution tools. Not suitable for onchainos.

### Why not okx/onchainos-skills?
`okx/onchainos-skills` includes `okx-defi-invest` which covers PancakeSwap farming/staking via the OKX infrastructure. However:
- It routes through OKX's aggregation layer, adding a third-party dependency
- It does not expose raw V3 contract interactions for arbitrary swaps and custom LP positions
- It requires OKX API keys and is tightly coupled to their backend

### Final decision: Direct on-chain via ABI calldata
PancakeSwap V3 is a close fork of Uniswap V3 with well-documented, stable ABIs. All write operations are straightforward contract calls with known function signatures. The approach:
- Off-chain reads: `eth_call` to QuoterV2 for swap quotes, PancakeV3Factory for pool addresses, pool contracts for slot0/liquidity, TheGraph subgraph for LP positions/history
- On-chain writes: ABI-encode calldata in Rust, submit via `onchainos wallet contract-call`

---

## Key Findings

### Contract Address Verification
Contract addresses were confirmed from two authoritative sources:
1. `pancakeswap/exchange-v3-subgraphs` GitHub repo — `config/bsc.js` and `config/base.js`
2. BscScan / BaseScan — verified contract labels ("PancakeSwap V3: Smart Router", "PancakeSwap: Quoter v2", etc.)

The PancakeV3Factory (`0x0BFbCF9...`) and NonfungiblePositionManager (`0x46A15B0b...`) share the **same address on both BSC and Base** (deterministic CREATE2 deployment). The SmartRouter differs: BSC uses `0x13f4EA83...`, Base uses `0x678Aa4bF...`.

Notably, in the BSC deployment JSON, `MixedRouteQuoterV1` is deployed at `0x678Aa4bF...` — the same address as Base's SmartRouter. This is NOT an error; each chain has its own deployment and these happen to share addresses on different chains for different contracts. Always use chain ID to select the correct address.

### SmartRouter vs SwapRouter
There are two swap routers:
- **SmartRouter** (`0x13f4EA83...` BSC, `0x678Aa4bF...` Base): The recommended entry point. Routes across V2, V3, and stable pools. `exactInputSingle` has 7 struct fields (NO deadline).
- **SwapRouter** (`0x1b81D678...` on both chains): V3-only legacy router. `exactInputSingle` has 8 struct fields (includes deadline). Still functional but SmartRouter is preferred.

The Developer Agent should default to SmartRouter for swaps.

### QuoterV2 Note
QuoterV2 (`0xB048Bbc1...`) is deployed at the same address on both BSC and Base. It uses internal simulation (not actual state changes) via eth_call. Some RPC nodes impose strict eth_call gas limits — the Rust client should set a generous gas limit (~5M) in the eth_call request body to avoid false "out of gas" errors from the quoter.

### Subgraph Status
- BSC: The hosted TheGraph service (`api.thegraph.com/subgraphs/name/pancakeswap/exchange-v3-bsc`) is active but TheGraph is deprecating hosted services. The decentralized subgraph ID is `78EUqzJmEVJsAKvWghn7qotf9LVGqcTQxJhT5z84ZmgJ`. An API key is required for the decentralized network.
- Base: `https://api.studio.thegraph.com/query/45376/exchange-v3-base/version/latest` — Studio endpoint, no API key needed, suitable for development and moderate production use.

### LP Position Retrieval
PancakeSwap does not have a REST API for LP positions. Options:
1. TheGraph subgraph (preferred for full history): query `positions` entity filtered by `owner`
2. On-chain enumeration: `NonfungiblePositionManager.balanceOf(owner)` → `tokenOfOwnerByIndex(owner, i)` → `positions(tokenId)`. Slower but no subgraph dependency.
The design recommends subgraph-first with on-chain fallback.

### Gotchas (Summary)
1. SmartRouter `exactInputSingle` has NO `deadline` field — 7 params, not 8
2. NonfungiblePositionManager requires `token0 < token1` numerically
3. `decreaseLiquidity` + `collect` are always two separate txns
4. Approvals target different contracts: SmartRouter for swaps, NPM for LP
5. QuoterV2 eth_call needs explicit gas limit to work reliably
6. Tick values must be multiples of tickSpacing for the fee tier
7. Base has two USDC tokens (USDC.b vs USDC native) — treat as distinct
