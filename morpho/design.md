# Morpho — Plugin Store Design Document

> Complete design for the Developer Agent. Covers Morpho Blue + MetaMorpho on Ethereum (1) and Base (8453).

---

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `morpho` |
| dapp_name | Morpho |
| dapp_repo | https://github.com/morpho-org/morpho-blue |
| dapp_alias | morpho blue, morpho protocol, morpho v1, metamorpho |
| one_liner | Supply, borrow and earn yield on Morpho — a permissionless lending protocol with $5B+ TVL |
| category | defi-protocol |
| tags | lending, borrowing, defi, earn, morpho, collateral, erc4626, metamorpho |
| target_chains | ethereum (1), base (8453) |
| target_protocols | Morpho Blue, MetaMorpho |
| version | 1.0.0 |
| author | plugin-dev-pipeline |

---

## 1. Feasibility Research

### 1a. Feasibility Table

| Check | Result |
|-------|--------|
| Has Rust SDK? | **No.** Morpho has no Rust SDK. All official SDKs are TypeScript: `@morpho-org/blue-sdk`, `@morpho-org/blue-sdk-viem`, `@morpho-org/bundler-sdk-viem` (96.5% TypeScript). Repo: https://github.com/morpho-org/sdks |
| SDK supported languages? | TypeScript/JavaScript only (Viem + Wagmi ecosystem). No Rust, Python, or Go SDKs exist. |
| Has REST/GraphQL API? | **Yes.** GraphQL API at `https://api.morpho.org/graphql`. Supports markets, vaults, positions, rewards queries. Rate limit: 5,000 requests per 5 minutes. No API key required. Docs: https://docs.morpho.org/tools/offchain/api/get-started/ |
| Has official Skill? | **No.** No official onchainos / plugin-store Skill found. Morpho has a Ledger hardware wallet plugin (`morpho-ledger-plugin`) but nothing for this platform. |
| Community Skill (similar)? | **No.** Searched "morpho onchainos", "morpho plugin-store", "morpho skill github" — no results found. No reference implementation to copy. |
| Supported chains? | Ethereum (1), Base (8453), Arbitrum (42161), OP Mainnet (10), Polygon (137), Unichain (130), HyperEVM, Katana, Monad. This plugin targets Ethereum and Base only. |
| Needs onchainos broadcast? | **Yes.** All write operations (supply, borrow, repay, collateral, approve) are EVM contract calls and must go through `onchainos wallet contract-call`. |

### 1b. Integration Path Decision

**Path: API** (GraphQL for off-chain reads + manual ABI encoding for on-chain writes)

Rationale:
- No Rust SDK exists
- TypeScript SDKs cannot be used in a Rust plugin binary
- The GraphQL API covers all read operations cleanly
- On-chain writes encode directly to calldata using the Morpho Blue and ERC-4626 ABIs — no SDK needed
- No community Skill to reference

---

## 2. Interface Mapping

### 2a. Operations Overview

| # | Operation | Type | Priority | Contract |
|---|-----------|------|----------|---------|
| 1 | Approve ERC-20 for Morpho Blue | On-chain | P0 (prerequisite) | Token contract |
| 2 | Supply collateral to Morpho Blue market | On-chain | P0 | Morpho Blue |
| 3 | Borrow from Morpho Blue market | On-chain | P0 | Morpho Blue |
| 4 | Repay debt on Morpho Blue market | On-chain | P0 | Morpho Blue |
| 5 | Withdraw collateral from Morpho Blue market | On-chain | P0 | Morpho Blue |
| 6 | Approve ERC-20 for MetaMorpho vault | On-chain | P0 (prerequisite) | Token contract |
| 7 | Deposit (supply) to MetaMorpho vault | On-chain | P0 | MetaMorpho vault (ERC-4626) |
| 8 | Withdraw from MetaMorpho vault | On-chain | P0 | MetaMorpho vault (ERC-4626) |
| 9 | View user positions and health factor | Off-chain read | P0 | Morpho GraphQL API |
| 10 | List markets with APYs | Off-chain read | P0 | Morpho GraphQL API |
| 11 | Claim rewards (Merkl) | On-chain | P1 | Merkl Distributor |

---

### 2b. Contract Addresses

#### Morpho Blue (Core Protocol)
Morpho Blue is deployed at the **same address on both chains** (deterministic deployment):

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb` |
| Base (8453) | `0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb` |

Source: https://docs.morpho.org/get-started/resources/addresses/ and verified on Etherscan + BaseScan.

#### MetaMorpho Factory

| Chain | Version | Address |
|-------|---------|---------|
| Ethereum (1) | v1.1 (current) | `0x1897A8997241C1cD4bD0698647e4EB7213535c24` |
| Ethereum (1) | v1.0 (old) | `0xA9c3D3a366466Fa809d1Ae982Fb2c46E5fC41101` |
| Base (8453) | v1.1 (current) | `0xFf62A7c278C62eD665133147129245053Bbf5918` |

Source: `@morpho-org/blue-sdk` addresses.ts — authoritative hardcoded addresses in SDK source.

#### MetaMorpho Vault V2 Factory (newer vaults)

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0xA1D94F746dEfa1928926b84fB2596c06926C0405` |
| Base (8453) | `0x4501125508079A99ebBebCE205DeC9593C2b5857` |

#### Adaptive Curve IRM (default interest rate model)

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x870aC11D48B15DB9a138Cf899d20F13F79Ba00BC` |
| Base (8453) | `0x46415998764C29aB2a25CbeA6254146D50D22687` |

#### Bundler3 (for batched operations — informational only)

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x6566194141eefa99Af43Bb5Aa71460Ca2Dc90245` |
| Base (8453) | `0x6BFd8137e702540E7A42B74178A4a49Ba43920C4` |

#### MORPHO Token

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x58D97B57BB95320F9a05dC918Aef65434969c2B2` |
| Base (8453) | `0xBAa5CC21fd487B8Fcc2F632f3F4E8D37262a0842` |

#### Universal Rewards Distributor Factory

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x9baA51245CDD28D8D74Afe8B3959b616E9ee7c8D` |
| Base (8453) | `0x7276454fc1cf9C408deeed722fd6b5E7A4CA25D8` |

#### Merkl Rewards Distributor (current reward system)

| Chain | Address |
|-------|---------|
| Ethereum (1) | `0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae` |
| Base (8453) | Use Merkl API to resolve per-chain distributor address |

Source: https://docs.morpho.org/build/rewards/tutorials/claim-rewards

#### Well-Known MetaMorpho Vault Addresses

**Ethereum Mainnet:**

| Vault | Asset | Address |
|-------|-------|---------|
| Steakhouse USDC | USDC | `0xBEEF01735c132Ada46AA9aA4c54623cAA92A64CB` |
| Gauntlet USDC Core | USDC | `0x8eB67A509616cd6A7c1B3c8C21D48FF57df3d458` |
| MEV Capital USDC | USDC | `0xd63070114470f685b75B74D60EEc7c1113d33a3D` |
| Steakhouse ETH | WETH | `0xBEEf050ecd6a16c4e7bfFbB52Ebba7846C4b8cD4` |
| Gauntlet WETH Prime | WETH | `0x2371e134e3455e0593363cBF89d3b6cf53740618` |
| Gauntlet WETH Core | WETH | `0x4881Ef0BF6d2365D3dd6499ccd7532bcdBCE0658` |

**Base:**

| Vault | Asset | Address |
|-------|-------|---------|
| Moonwell Flagship USDC | USDC | `0xc1256Ae5FF1cf2719D4937adb3bbCCab2E00A2Ca` |
| Steakhouse USDC | USDC | `0xbeeF010f9cb27031ad51e3333f9aF9C6B1228183` |
| Spark USDC | USDC | `0x7BfA7C4f149E7415b73bdeDfe609237e29CBF34A` |
| Base wETH | WETH | `0x3aC2bBD41D7A92326dA602f072D40255Dd8D23a2` |
| Seamless WETH | WETH | `0x27D8c7273fd3fcC6956a0B370cE5Fd4A7fc65c18` |

> Note: Vault addresses should always be resolved dynamically via the Morpho GraphQL API (`vaultV2s` query) — the above list is a starting set for reference and testing. New vaults are deployed regularly.

#### Common Token Addresses

**Ethereum Mainnet:**
- WETH: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`
- USDC: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48`
- USDT: `0xdAC17F958D2ee523a2206206994597C13D831ec7`
- DAI: `0x6B175474E89094C44Da98b954EedeAC495271d0F`
- wstETH: `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0`

**Base:**
- WETH: `0x4200000000000000000000000000000000000006`
- USDC: `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913`
- cbETH: `0x2Ae3F1Ec7F1F5012CFEab0185bfc7aa3cf0DEc22`
- cbBTC: `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf`

---

### 2c. Core Data Structures

#### MarketParams Struct (used in all Morpho Blue calls)

```
struct MarketParams {
    address loanToken;        // The loan/borrow token
    address collateralToken;  // The collateral token
    address oracle;           // Price oracle contract
    address irm;              // Interest rate model contract
    uint256 lltv;             // Liquidation LTV in WAD (1e18 = 100%)
}
```

ABI-encoding order: `(address, address, address, address, uint256)` — packed as 5 × 32-byte slots.

The **marketId** (bytes32 `Id`) is `keccak256(abi.encode(marketParams))` — the hash of the ABI-encoded struct. This is what the GraphQL API returns as `uniqueKey`.

#### Position Struct (returned by `position()`)

```
struct Position {
    uint256 supplyShares;
    uint128 borrowShares;
    uint128 collateral;
}
```

#### Market Struct (returned by `market()`)

```
struct Market {
    uint128 totalSupplyAssets;
    uint128 totalSupplyShares;
    uint128 totalBorrowAssets;
    uint128 totalBorrowShares;
    uint128 lastUpdate;
    uint128 fee;
}
```

---

### 2d. Morpho Blue Function Signatures

All functions are on `Morpho Blue` at `0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb`.

#### `supplyCollateral`
```
function supplyCollateral(
    MarketParams memory marketParams,  // (address,address,address,address,uint256)
    uint256 assets,                    // collateral amount in token units
    address onBehalf,                  // position owner (usually msg.sender)
    bytes calldata data                // empty bytes (0x) for simple calls
) external
```
Selector: `keccak256("supplyCollateral((address,address,address,address,uint256),uint256,address,bytes)")` → `0x238d6579`

#### `withdrawCollateral`
```
function withdrawCollateral(
    MarketParams memory marketParams,
    uint256 assets,     // collateral amount to withdraw
    address onBehalf,   // position owner
    address receiver    // recipient of tokens
) external
```
Selector: `0x8720316d`

#### `borrow`
```
function borrow(
    MarketParams memory marketParams,
    uint256 assets,     // amount to borrow in loan token (set shares=0)
    uint256 shares,     // borrow shares (set assets=0 to use shares)
    address onBehalf,   // position owner
    address receiver    // recipient of borrowed tokens
) external returns (uint256 assetsBorrowed, uint256 sharesBorrowed)
```
Selector: `0x50d8cd4b`

#### `repay`
```
function repay(
    MarketParams memory marketParams,
    uint256 assets,     // repay amount in assets (set shares=0)
    uint256 shares,     // repay in shares (preferred for full repay; set assets=0)
    address onBehalf,   // position owner
    bytes calldata data // empty bytes (0x) for simple calls
) external returns (uint256 assetsRepaid, uint256 sharesRepaid)
```
Selector: `0x20b76e81`

#### `supply` (supply lending liquidity — distinct from MetaMorpho vault deposit)
```
function supply(
    MarketParams memory marketParams,
    uint256 assets,     // amount to lend (set shares=0)
    uint256 shares,     // lend in shares (set assets=0)
    address onBehalf,
    bytes calldata data // empty bytes (0x)
) external returns (uint256 assetsSupplied, uint256 sharesSupplied)
```
Selector: `0xa99aad89`

#### `withdraw` (withdraw lending position from Morpho Blue — NOT vault)
```
function withdraw(
    MarketParams memory marketParams,
    uint256 assets,
    uint256 shares,
    address onBehalf,
    address receiver
) external returns (uint256 assetsWithdrawn, uint256 sharesWithdrawn)
```
Selector: `0x5c2bea49`

#### `position` (view — read current position)
```
function position(bytes32 id, address user)
    external view returns (uint256 supplyShares, uint128 borrowShares, uint128 collateral)
```

#### `market` (view — read market state)
```
function market(bytes32 id)
    external view returns (
        uint128 totalSupplyAssets, uint128 totalSupplyShares,
        uint128 totalBorrowAssets, uint128 totalBorrowShares,
        uint128 lastUpdate, uint128 fee
    )
```

---

### 2e. MetaMorpho (ERC-4626) Function Signatures

All MetaMorpho vaults implement ERC-4626. The vault address is vault-specific (see §2b).

#### `deposit` (supply assets to vault)
```
function deposit(
    uint256 assets,      // underlying token amount
    address receiver     // recipient of vault shares
) external returns (uint256 shares)
```
Selector: `0x6e553f65`

#### `withdraw` (withdraw exact asset amount)
```
function withdraw(
    uint256 assets,      // underlying token amount to withdraw
    address receiver,    // recipient of tokens
    address owner        // share owner (usually msg.sender)
) external returns (uint256 shares)
```
Selector: `0xb460af94`

#### `redeem` (redeem exact shares — preferred for full withdrawal)
```
function redeem(
    uint256 shares,      // vault shares to burn
    address receiver,    // recipient of underlying tokens
    address owner        // share owner (usually msg.sender)
) external returns (uint256 assets)
```
Selector: `0xba087652`

#### `balanceOf` (view — get share balance)
```
function balanceOf(address account) external view returns (uint256)
```

#### `convertToAssets` (view — preview share redemption)
```
function convertToAssets(uint256 shares) external view returns (uint256)
```

#### `maxWithdraw` (view — max withdrawable — NOTE: returns 0 for V2 vaults)
```
function maxWithdraw(address owner) external view returns (uint256)
```

---

### 2f. ERC-20 Approve Function Signature

Required before any Morpho Blue or MetaMorpho vault call that pulls tokens.

```
function approve(address spender, uint256 amount) external returns (bool)
```
Selector: `0x095ea7b3`

Calldata construction:
```
0x095ea7b3
<spender: 32-byte left-padded address>
<amount: 32-byte uint256>
```

For "approve max" use `uint256::MAX = 0xffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff`.

---

### 2g. On-Chain Operation → onchainos Command Mapping

All on-chain operations use:
```
onchainos wallet contract-call --chain <CHAIN_ID> --to <CONTRACT_ADDRESS> --input-data <HEX_CALLDATA>
```

#### Operation 1: ERC-20 Approve (prerequisite for deposits/repays/collateral)

Calldata: `approve(spender, amount)`
```
0x095ea7b3
000000000000000000000000<SPENDER_ADDRESS_NO_0x>
<AMOUNT_AS_32_BYTE_HEX>
```

Example — approve Morpho Blue to spend 1000 USDC (6 decimals = 1_000_000_000):
```
onchainos wallet contract-call \
  --chain 1 \
  --to 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
  --input-data 0x095ea7b3000000000000000000000000BBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb000000000000000000000000000000000000000000000000000000003b9aca00
```

#### Operation 2: Supply Collateral to Morpho Blue

Calldata encodes: `supplyCollateral(marketParams, assets, onBehalf, data)`

The `marketParams` tuple `(address,address,address,address,uint256)` is ABI-encoded as 5 × 32-byte slots. The `data` field (bytes) is an empty dynamic bytes value.

Full calldata layout (in order):
1. 4-byte selector: `0x238d6579`
2. 5 × 32 bytes for `marketParams` (loanToken, collateralToken, oracle, irm, lltv)
3. 32 bytes for `assets`
4. 32 bytes for `onBehalf`
5. 32 bytes for offset of `data` bytes field (= `0x00...c0` = 192, pointing past the fixed params)
6. 32 bytes for `data` length (= 0)

```
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to 0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb \
  --input-data <CALLDATA>
```

#### Operation 3: Borrow from Morpho Blue

Calldata: `borrow(marketParams, assets, 0, onBehalf, receiver)`
- Set `assets` = desired borrow amount (in loan token units)
- Set `shares` = 0 (asset-first mode)
- `onBehalf` = user address
- `receiver` = address to receive borrowed tokens (usually same as user)

Selector: `0x50d8cd4b`

Layout: 4 selector + 5×32 for marketParams + 32 assets + 32 shares + 32 onBehalf + 32 receiver

#### Operation 4: Repay Morpho Blue Debt

Calldata: `repay(marketParams, 0, shares, onBehalf, data)`
- For full repayment: use `shares` = borrowShares from `position()` view, set `assets` = 0
- For partial repayment: use `assets` = repay amount, set `shares` = 0

Selector: `0x20b76e81`

Layout: 4 + 5×32 marketParams + 32 assets + 32 shares + 32 onBehalf + 32 data_offset + 32 data_length

**Pre-requisite:** Approve Morpho Blue to spend loan token (`approve(morphoBlue, repayAmount)`).

#### Operation 5: Withdraw Collateral from Morpho Blue

Calldata: `withdrawCollateral(marketParams, assets, onBehalf, receiver)`

Selector: `0x8720316d`

Layout: 4 + 5×32 marketParams + 32 assets + 32 onBehalf + 32 receiver

**Pre-condition check:** Health factor must remain > 1.0 after withdrawal. Calculate before calling:
```
health_factor = (collateral_remaining × oracle_price / 1e36 × lltv / 1e18) / borrow_assets
```

#### Operation 6: Deposit to MetaMorpho Vault

Calldata: `deposit(assets, receiver)`

Selector: `0x6e553f65`

Layout:
```
0x6e553f65
<assets: 32 bytes>
<receiver: 32 bytes (left-padded address)>
```

**Pre-requisite:** Approve vault contract to spend the underlying token.

```
onchainos wallet contract-call \
  --chain <CHAIN_ID> \
  --to <VAULT_ADDRESS> \
  --input-data 0x6e553f65<ASSETS_32_BYTES><RECEIVER_32_BYTES>
```

#### Operation 7: Withdraw/Redeem from MetaMorpho Vault

For partial withdrawal by asset amount: `withdraw(assets, receiver, owner)` — selector `0xb460af94`
```
0xb460af94
<assets: 32 bytes>
<receiver: 32 bytes>
<owner: 32 bytes>
```

For full withdrawal by shares: `redeem(shares, receiver, owner)` — selector `0xba087652`
```
0xba087652
<shares: 32 bytes>
<receiver: 32 bytes>
<owner: 32 bytes>
```

**Best practice:** Use `redeem(balanceOf(user), receiver, user)` for full exit to avoid dust.

#### Operation 8: Claim Rewards (Merkl)

**Step 1 (off-chain):** Fetch claim data from Merkl API:
```
GET https://api.merkl.xyz/v4/claim?user=<USER_ADDRESS>&chainId=<CHAIN_ID>
```
Response contains: `user`, `tokens[]`, `amounts[]`, `proofs[][]`

**Step 2 (on-chain):** Call `claim(user, tokens, amounts, proofs)` on Merkl Distributor.

Merkl Distributor Ethereum: `0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae`

Selector: `0x2e7ba6ef` (Merkl `claim(address,address[],uint256[],bytes32[][])`)

The calldata is complex due to dynamic arrays — encode using standard ABI encoding rules:
- 4-byte selector
- Offset to `user` (= 0x20 since it's static address... but actually user is a param): encode as `(address, address[], uint256[], bytes32[][])`

Pseudocode for calldata construction:
```rust
// tokens, amounts, proofs are dynamic arrays from API response
// Use abi_encode_packed for: selector + abi.encode(user, tokens, amounts, proofs)
```

---

### 2h. Off-Chain Query Operations

#### GraphQL Endpoint

```
POST https://api.morpho.org/graphql
Content-Type: application/json
```

No authentication required. Rate limit: 5,000 requests / 5 minutes.

#### Query 1: List Markets with APYs

```graphql
query ListMarkets($chainId: Int!, $first: Int) {
  markets(
    first: $first
    orderBy: SupplyAssetsUsd
    orderDirection: Desc
    where: { chainId_in: [$chainId] }
  ) {
    items {
      uniqueKey
      lltv
      loanAsset {
        address
        symbol
        decimals
      }
      collateralAsset {
        address
        symbol
        decimals
      }
      state {
        supplyApy
        borrowApy
        avgNetSupplyApy
        avgNetBorrowApy
        supplyAssets
        borrowAssets
        supplyAssetsUsd
        borrowAssetsUsd
        utilization
        fee
        rewards {
          asset { address symbol }
          supplyApr
          borrowApr
        }
      }
    }
  }
}
```

Variables: `{ "chainId": 1, "first": 50 }`

#### Query 2: List MetaMorpho Vaults

```graphql
query ListVaults($chainId: Int!) {
  vaultV2s(
    first: 100
    orderBy: TotalAssetsUsd
    orderDirection: Desc
    where: { chainId_in: [$chainId] }
  ) {
    items {
      address
      symbol
      name
      listed
      asset {
        address
        symbol
        decimals
      }
      chain { id network }
      totalAssets
      totalAssetsUsd
      liquidity
      avgApy
      avgNetApy
      performanceFee
    }
  }
}
```

#### Query 3: Get User Positions (Morpho Blue markets)

```graphql
query UserMarketPositions($userAddress: String!, $chainId: Int!) {
  marketPositions(
    first: 50
    where: {
      userAddress_in: [$userAddress]
      chainId_in: [$chainId]
    }
  ) {
    items {
      market {
        uniqueKey
        lltv
        loanAsset { address symbol decimals }
        collateralAsset { address symbol decimals }
        state {
          borrowApy
          supplyApy
        }
      }
      user { address }
      state {
        supplyShares
        supplyAssets
        supplyAssetsUsd
        borrowShares
        borrowAssets
        borrowAssetsUsd
        collateral
        collateralUsd
        healthFactor
      }
    }
  }
}
```

Variables: `{ "userAddress": "0x...", "chainId": 1 }`

Key field: `state.healthFactor` — a float > 1.0 means safe. Below 1.0 means liquidatable.

#### Query 4: Get User Vault Positions

```graphql
query UserVaultPositions($userAddress: String!, $chainId: Int!) {
  userByAddress(address: $userAddress, chainId: $chainId) {
    vaultV2Positions {
      shares
      assets
      assetsUsd
      vault {
        address
        symbol
        name
        asset { address symbol decimals }
        avgApy
        avgNetApy
      }
    }
  }
}
```

#### Query 5: Get Single Vault Details

```graphql
query VaultDetails($vaultAddress: String!, $chainId: Int!) {
  vaultV2ByAddress(address: $vaultAddress, chainId: $chainId) {
    address
    symbol
    name
    totalAssets
    totalAssetsUsd
    totalSupply
    liquidity
    liquidityUsd
    sharePrice
    avgApy
    avgNetApy
    performanceFee
    managementFee
    asset { address symbol decimals }
  }
}
```

#### Health Factor Calculation

The GraphQL API returns `state.healthFactor` directly in `marketPositions`. However, if you need to compute manually:

```
collateral_value_in_loan_token = (collateral_amount × oracle_price) / ORACLE_PRICE_SCALE
ORACLE_PRICE_SCALE = 10^36  (Morpho uses 1e36 scaled oracle prices)

health_factor = (collateral_value_in_loan_token × lltv / WAD) / borrow_assets
WAD = 10^18  (lltv is expressed in WAD)
```

A position is safe when `health_factor > 1.0`. Liquidation risk threshold: `health_factor < 1.1` (aggressive), `< 1.25` (moderate).

---

## 3. User Scenarios

### Scenario 1: "Earn yield on my USDC — supply to best vault on Base"

**User says:** "I want to earn yield on 5000 USDC on Base. What's the best option?"

**Agent action sequence:**

1. **[Off-chain — GraphQL]** Query MetaMorpho vaults on Base sorted by APY:
   ```graphql
   vaultV2s(where: { chainId_in: [8453] }, orderBy: AvgNetApy, orderDirection: Desc, first: 20)
   ```
   Filter results to USDC vaults (`asset.symbol == "USDC"`). Present top 3 options to user with APY and TVL.

2. **User selects** Moonwell Flagship USDC vault (`0xc1256Ae5FF1cf2719D4937adb3bbCCab2E00A2Ca`).

3. **[Off-chain — GraphQL]** Query vault details to confirm current APY and liquidity.

4. **[On-chain — onchainos]** Approve USDC for the vault:
   ```
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 \
     --input-data 0x095ea7b3
                  000000000000000000000000c1256Ae5FF1cf2719D4937adb3bbCCab2E00A2Ca
                  00000000000000000000000000000000000000000000000000000001DCD65000
   ```
   (5000 USDC = 5_000_000_000 units with 6 decimals = `0x12A05F200` ... encode as `0x1DCD65000` for 8000000000... use exact user input)

5. **[On-chain — onchainos]** Deposit 5000 USDC to vault:
   ```
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0xc1256Ae5FF1cf2719D4937adb3bbCCab2E00A2Ca \
     --input-data 0x6e553f65
                  00000000000000000000000000000000000000000000000000000001DCD65000
                  000000000000000000000000<USER_ADDRESS>
   ```

6. **Agent confirms** transaction hash and estimated shares received. Reports expected annual yield in USD.

---

### Scenario 2: "Borrow USDC against my ETH collateral on Ethereum mainnet"

**User says:** "I want to borrow 2000 USDC using my ETH as collateral on Ethereum."

**Agent action sequence:**

1. **[Off-chain — GraphQL]** List Morpho Blue markets on Ethereum where loanToken=USDC and collateralToken=WETH:
   ```graphql
   markets(where: { chainId_in: [1] }) { items { uniqueKey loanAsset { symbol } collateralAsset { symbol } lltv state { borrowApy borrowAssetsUsd } } }
   ```
   Filter to WETH/USDC markets. Present best market (highest LLTV or lowest borrow APY) to user.

2. **[Off-chain — GraphQL]** Query user's current position to confirm no existing debt conflict.

3. **Agent calculates** required collateral: to borrow 2000 USDC safely at 77% LLTV, need at least `2000 / 0.77 / ETH_PRICE_IN_USD` ETH. Warn user about required collateral amount.

4. **[On-chain — onchainos]** Approve WETH for Morpho Blue (if not already approved):
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2 \
     --input-data 0x095ea7b3
                  000000000000000000000000BBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb
                  <COLLATERAL_AMOUNT_AS_32_BYTES>
   ```

5. **[On-chain — onchainos]** Supply WETH collateral to the selected market:
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb \
     --input-data 0x238d6579<ABI_ENCODED_MARKET_PARAMS><COLLATERAL_AMOUNT><USER_ADDRESS><BYTES_OFFSET_AND_LENGTH>
   ```

6. **[On-chain — onchainos]** Borrow 2000 USDC (2_000_000_000 units, 6 decimals):
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb \
     --input-data 0x50d8cd4b<ABI_ENCODED_MARKET_PARAMS>
                  00000000000000000000000000000000000000000000000000000000773593000  [2000 USDC]
                  0000000000000000000000000000000000000000000000000000000000000000  [shares=0]
                  000000000000000000000000<USER_ADDRESS>
                  000000000000000000000000<USER_ADDRESS>
   ```

7. **Agent reports** transaction hash, confirms USDC received, shows current health factor (should be > 1.5 for safety).

---

### Scenario 3: "Check my positions and health factor"

**User says:** "Show me all my Morpho positions and whether I'm at risk of liquidation."

**Agent action sequence:**

1. **[Off-chain — GraphQL]** Query user's Morpho Blue market positions:
   ```graphql
   marketPositions(where: { userAddress_in: ["<USER>"], chainId_in: [1, 8453] }) {
     items {
       market { uniqueKey lltv loanAsset { symbol } collateralAsset { symbol } state { borrowApy } }
       state { collateral collateralUsd borrowAssets borrowAssetsUsd supplyAssets supplyAssetsUsd healthFactor }
     }
   }
   ```

2. **[Off-chain — GraphQL]** Query user's MetaMorpho vault positions:
   ```graphql
   userByAddress(address: "<USER>", chainId: 1) { vaultV2Positions { shares assets assetsUsd vault { symbol avgNetApy } } }
   ```
   Repeat for Base (chainId: 8453).

3. **Agent formats** results:
   - For each borrow position: show collateral, debt, health factor, liquidation price
   - For each vault position: show balance in underlying token + USD, current APY
   - Flag any position with `healthFactor < 1.25` as "At Risk — consider repaying or adding collateral"

4. **If a position is at risk, agent proactively offers:** "Your WETH/USDC position has health factor 1.12. Would you like to add more collateral or repay some debt?"

---

### Scenario 4: "Repay my USDC debt and withdraw collateral"

**User says:** "I want to fully repay my USDC debt on Ethereum and get my ETH back."

**Agent action sequence:**

1. **[Off-chain — GraphQL]** Get user's current position:
   ```graphql
   marketPositions(where: { userAddress_in: ["<USER>"], chainId_in: [1] }) {
     items { market { uniqueKey loanAsset { address } collateralAsset { address } lltv oracle irm }
             state { borrowShares borrowAssets collateral } }
   }
   ```

2. **Agent notes** the `borrowShares` value for full repayment (use shares to avoid dust).

3. **[Off-chain]** Estimate repayment amount: call `market(id)` view function to get current `totalBorrowAssets / totalBorrowShares` ratio. Compute assets to repay ≈ `borrowShares × totalBorrowAssets / totalBorrowShares`. Add 0.1% buffer for accrued interest.

4. **[On-chain — onchainos]** Approve USDC for Morpho Blue (repay amount + buffer):
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48 \
     --input-data 0x095ea7b3
                  000000000000000000000000BBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb
                  <REPAY_AMOUNT_WITH_BUFFER_32_BYTES>
   ```

5. **[On-chain — onchainos]** Repay using shares (full repayment):
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb \
     --input-data 0x20b76e81<MARKET_PARAMS>
                  0000000000000000000000000000000000000000000000000000000000000000  [assets=0]
                  <BORROW_SHARES_32_BYTES>  [use shares for full repay]
                  000000000000000000000000<USER_ADDRESS>
                  <BYTES_OFFSET><BYTES_LENGTH=0>
   ```

6. **[On-chain — onchainos]** Withdraw all collateral:
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0xBBBBBbbBBb9cC5e90e3b3Af64bdAF62C37EEFFCb \
     --input-data 0x8720316d<MARKET_PARAMS>
                  <COLLATERAL_AMOUNT_32_BYTES>
                  000000000000000000000000<USER_ADDRESS>
                  000000000000000000000000<USER_ADDRESS>
   ```

7. **Agent confirms** both transactions, shows USDC spent and WETH received.

---

### Scenario 5: "Claim my Morpho rewards"

**User says:** "Claim all my pending MORPHO rewards."

**Agent action sequence:**

1. **[Off-chain — Merkl API]** Fetch claimable rewards:
   ```
   GET https://api.merkl.xyz/v4/claim?user=<USER_ADDRESS>&chainId=<CHAIN_ID>
   ```
   Parse `tokens[]`, `amounts[]`, `proofs[][]` from response.

2. **[Agent checks]** if `amounts` has any non-zero entries. If empty, report "No rewards to claim."

3. **[On-chain — onchainos]** Call `claim()` on Merkl Distributor:
   ```
   onchainos wallet contract-call \
     --chain 1 \
     --to 0x3Ef3D8bA38EBe18DB133cEc108f4D14CE00Dd9Ae \
     --input-data <ABI_ENCODED_CLAIM_CALLDATA>
   ```
   Calldata: `claim(address user, address[] tokens, uint256[] amounts, bytes32[][] proofs)`

4. **Agent reports** claimed tokens and amounts.

---

## 4. External API Dependencies

| API | URL | Purpose | Auth | Rate Limit |
|-----|-----|---------|------|------------|
| Morpho GraphQL API | `https://api.morpho.org/graphql` | Markets, vaults, positions, APYs, user data | None | 5,000 req/5min |
| Merkl API | `https://api.merkl.xyz/v4/claim` | Reward claim data (tokens, amounts, proofs) | None | Unknown |
| Ethereum RPC | Configurable (e.g. `https://eth.llamarpc.com`) | On-chain view calls (position, market, balanceOf) | None / API key | Provider-dependent |
| Base RPC | Configurable (e.g. `https://base.llamarpc.com`) | On-chain view calls on Base | None / API key | Provider-dependent |

### Morpho GraphQL API Details

- **Endpoint:** `https://api.morpho.org/graphql`
- **Method:** POST with JSON body `{ "query": "...", "variables": {...} }`
- **Supported chains in queries:** Filter by `chainId_in: [1, 8453]`
- **Pagination:** Use `first` (max 1000) and `skip` parameters
- **Max query complexity:** 1,000,000 points per request
- **Caching recommended:** Cache market/vault data for 60 seconds minimum

### Merkl API Details

- **Endpoint:** `GET https://api.merkl.xyz/v4/claim?user={address}&chainId={chainId}`
- **Response fields:**
  - `user`: address
  - `tokens`: `address[]`
  - `amounts`: `string[]` (as decimal strings)
  - `proofs`: `string[][]` (hex strings, bytes32 values)
- **Important:** Rewards system migrated from Morpho URD to Merkl — all new rewards (MORPHO and third-party) are distributed via Merkl.

### RPC Calls Required

The plugin needs direct RPC access for on-chain view calls that the GraphQL API does not expose in real-time:
- `position(marketId, userAddress)` — get current borrow shares for repay calculation
- `market(marketId)` — get current borrow/supply totals for asset↔share conversion
- `balanceOf(userAddress)` on vault — get share balance before redeem

Recommend using `eth_call` via the configured RPC endpoint.

---

## 5. Configuration Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `chain` | `u64` | `1` | Target chain ID (1 = Ethereum, 8453 = Base) |
| `dry_run` | `bool` | `false` | Simulate transactions without broadcasting (print calldata only) |
| `rpc_url_ethereum` | `String` | `"https://eth.llamarpc.com"` | Ethereum mainnet RPC endpoint |
| `rpc_url_base` | `String` | `"https://base.llamarpc.com"` | Base RPC endpoint |
| `morpho_api_url` | `String` | `"https://api.morpho.org/graphql"` | Morpho GraphQL API endpoint |
| `merkl_api_url` | `String` | `"https://api.merkl.xyz/v4/claim"` | Merkl rewards API endpoint |
| `default_slippage_bps` | `u32` | `50` | Default slippage tolerance in basis points (0.5%) |
| `health_factor_warn_threshold` | `f64` | `1.25` | Warn user when health factor falls below this value |
| `health_factor_min_threshold` | `f64` | `1.10` | Refuse to execute borrow/withdraw if health factor would go below this |
| `max_approve_amount` | `String` | `"max"` | Either `"max"` (uint256 max) or `"exact"` for exact approval amounts |
| `log_level` | `String` | `"info"` | Log verbosity: `"debug"`, `"info"`, `"warn"`, `"error"` |

### Configuration File Location

The plugin reads from `~/.config/onchainos/plugins/morpho/config.toml` (or environment variables prefixed with `MORPHO_`).

Example `config.toml`:
```toml
chain = 1
dry_run = false
rpc_url_ethereum = "https://mainnet.infura.io/v3/YOUR_KEY"
rpc_url_base = "https://base-mainnet.g.alchemy.com/v2/YOUR_KEY"
default_slippage_bps = 50
health_factor_warn_threshold = 1.25
health_factor_min_threshold = 1.10
max_approve_amount = "max"
```

---

## Appendix A: ABI Encoding Reference for Morpho Blue Calls

### Encoding `MarketParams` Struct

Given a market with:
- `loanToken`: `0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48` (USDC)
- `collateralToken`: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2` (WETH)
- `oracle`: `0x...` (market-specific, from GraphQL `oracleAddress` field)
- `irm`: `0x870aC11D48B15DB9a138Cf899d20F13F79Ba00BC` (Adaptive Curve IRM on mainnet)
- `lltv`: `770000000000000000` (77% in WAD = 0x0AAE60FA00000000 ≈ 0x0AB0BC7EC43D)

Encoding (each value padded to 32 bytes):
```
000000000000000000000000A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48  [loanToken]
000000000000000000000000C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2  [collateralToken]
000000000000000000000000<ORACLE_ADDRESS>                             [oracle]
000000000000000000000000870aC11D48B15DB9a138Cf899d20F13F79Ba00BC   [irm]
00000000000000000000000000000000000000000000000000000000AAE60FA0   [lltv = 77% WAD ≈ 0xAAE60FA0...]
```

Note: Always fetch the exact `lltv` value from the GraphQL API as it varies per market.

### Encoding Dynamic `bytes` Parameter (empty)

When `data` = empty bytes (`0x`):
- 32 bytes: offset pointing to the bytes field start (value = byte position of the length word, relative to start of tuple)
- 32 bytes: length = 0

For `supplyCollateral(marketParams, assets, onBehalf, data)`:
- Fixed portion: 5×32 (marketParams) + 32 (assets) + 32 (onBehalf) = 224 bytes = 0xE0
- Then: offset = `0x00...E0`, length = `0x00...00`

### Computing MarketId (bytes32)

```rust
// Rust pseudocode
let market_params_encoded = abi_encode_tuple(
    loan_token, collateral_token, oracle, irm, lltv
);
let market_id = keccak256(market_params_encoded);
```

This matches the `uniqueKey` returned by the GraphQL API.

---

## Appendix B: Morpho Blue Function Selectors Reference

| Function | Selector |
|----------|---------|
| `supplyCollateral((address,address,address,address,uint256),uint256,address,bytes)` | `0x238d6579` |
| `withdrawCollateral((address,address,address,address,uint256),uint256,address,address)` | `0x8720316d` |
| `borrow((address,address,address,address,uint256),uint256,uint256,address,address)` | `0x50d8cd4b` |
| `repay((address,address,address,address,uint256),uint256,uint256,address,bytes)` | `0x20b76e81` |
| `supply((address,address,address,address,uint256),uint256,uint256,address,bytes)` | `0xa99aad89` |
| `withdraw((address,address,address,address,uint256),uint256,uint256,address,address)` | `0x5c2bea49` |
| `position(bytes32,address)` | `0x4fe09761` |
| `market(bytes32)` | `0x571adfd3` |
| ERC-4626 `deposit(uint256,address)` | `0x6e553f65` |
| ERC-4626 `withdraw(uint256,address,address)` | `0xb460af94` |
| ERC-4626 `redeem(uint256,address,address)` | `0xba087652` |
| ERC-4626 `balanceOf(address)` | `0x70a08231` |
| ERC-20 `approve(address,uint256)` | `0x095ea7b3` |
| ERC-20 `allowance(address,address)` | `0xdd62ed3e` |
| Merkl `claim(address,address[],uint256[],bytes32[][])` | `0x2e7ba6ef` |

---

## Appendix C: Known Market IDs (uniqueKey examples)

Market IDs to use in GraphQL queries can be resolved dynamically. To get major markets:
```graphql
markets(where: { chainId_in: [1], loanToken_in: ["0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48"] }) {
  items { uniqueKey collateralAsset { symbol } lltv state { borrowApy } }
}
```

The `uniqueKey` is the marketId (bytes32 hash) in hex. Use it in `position()` and `market()` view calls.
