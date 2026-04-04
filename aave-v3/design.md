# Aave V3 — Plugin Store Integration PRD

## 0. Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | aave-v3 |
| dapp_name | Aave V3 |
| dapp_repo | https://github.com/aave-dao/aave-v3-origin |
| dapp_alias | aave, aave-v3, aave3 |
| one_liner | Lend and borrow crypto assets on the leading decentralized liquidity protocol |
| category | defi-protocol |
| tags | lending, borrowing, defi, earn, aave, collateral, health-factor |
| target_chains | ethereum:1, polygon:137, arbitrum:42161, base:8453 |
| target_protocols | Aave V3 |
| architecture | Skill + Binary (Rust) |
| github_username | skylavis-sky |

---

## 1. Background

### What is Aave V3

Aave V3 is the leading decentralized, non-custodial liquidity protocol where users supply crypto assets to earn interest and borrow against overcollateralized positions across 14+ EVM-compatible blockchains. As of late 2025, Aave V3 holds over $43 billion in total value locked (TVL), making it the largest DeFi lending market by TVL and commanding roughly one fifth of total DeFi TVL. The protocol issues aTokens (e.g., aUSDC, aWETH) representing earning positions and tracks debt via variable-rate debt tokens, with positions protected by a Health Factor liquidation model. V3 introduces efficiency mode (E-Mode) for correlated assets, isolation mode for volatile assets, and multi-chain deployment using a uniform contract interface anchored by a per-chain `PoolAddressesProvider` registry.

### Feasibility Research

| Check | Result |
|-------|--------|
| Rust SDK? | Community-only: `aave-sdk` crate on crates.io (not official). Official SDK is TypeScript-only (`@aave/client`, `@aave/react`). |
| SDK tech stacks? | Official: TypeScript (`@aave/react`, `@aave/contract-helpers`, `@aave/math-utils`). Community Rust crate available but unmaintained. Recommend using `alloy-rs` or `ethabi` + direct RPC/subgraph for Rust binary. |
| REST API? | No official REST API for V3. Data available via: (1) The Graph subgraphs (GraphQL), (2) on-chain view contracts (`UiPoolDataProvider`, `AaveProtocolDataProvider`), (3) legacy `aave-api-v2.aave.com` (V2-era, limited V3 coverage). |
| Official Skill? | No official onchainos skill for Aave V3. |
| Community Skill? | Two community MCP servers found: `Tairon-ai/aave-mcp` (TypeScript/NestJS, Base-only, 22 tools) and `kukapay/aave-mcp` (Python, multi-chain via The Graph). Neither is a native onchainos skill. |
| Supported chains | ethereum (1), polygon (137), arbitrum (42161), base (8453) — all EVM, same contract interface |
| onchainos broadcast needed? | Yes — borrow, repay, set-collateral, set-emode require signed transactions via `onchainos wallet contract-call` |

### Integration Path

**Hybrid: onchainos defi skill + Rust binary**

- **Supply / Withdraw / Collect rewards**: Delegated to `onchainos defi invest`, `onchainos defi withdraw`, `onchainos defi collect` — the LENDING category in onchainos natively maps to Aave V3 Pool operations.
- **Borrow / Repay / Set collateral / Set E-Mode**: Rust binary constructs ABI-encoded calldata (using `ethabi` crate) and submits via `onchainos wallet contract-call`. Pool address resolved at runtime from `PoolAddressesProvider.getPool()`.
- **Health factor / Reserve data / User positions (read-only)**: Rust binary queries The Graph subgraphs (GraphQL) or calls on-chain view contracts (`getUserAccountData`, `UiPoolDataProvider`) via RPC. No transaction signing needed.

---

## 2. DApp Core Capabilities & Interface Mapping

### Operations to Integrate

| # | Operation | Description | On-chain/Off-chain |
|---|-----------|-------------|-------------------|
| 1 | supply | Deposit asset to earn interest, receive aTokens | On-chain (onchainos defi invest) |
| 2 | withdraw | Redeem aTokens and withdraw underlying asset | On-chain (onchainos defi withdraw) |
| 3 | borrow | Borrow asset against posted collateral | On-chain (wallet contract-call) |
| 4 | repay | Repay borrowed debt (partial or full) | On-chain (wallet contract-call) |
| 5 | positions | View current supply and borrow positions | Off-chain (onchainos defi positions + subgraph) |
| 6 | health-factor | Check account health factor and liquidation risk | Off-chain (Pool.getUserAccountData view call) |
| 7 | reserves | List market rates, APYs, liquidity for all assets | Off-chain (The Graph subgraph or UiPoolDataProvider) |
| 8 | claim-rewards | Claim accrued AAVE/GHO/token rewards | On-chain (onchainos defi collect) |
| 9 | set-collateral | Enable or disable an asset as collateral | On-chain (wallet contract-call) |
| 10 | set-emode | Enable efficiency mode for a correlated asset category | On-chain (wallet contract-call) |

---

### Off-chain Queries (Rust binary direct API / view contract calls)

| Operation | Method | Endpoint / Contract | Key Params | Returns |
|-----------|--------|---------------------|------------|---------|
| health-factor | RPC eth_call | `Pool.getUserAccountData(address user)` (resolved from PoolAddressesProvider) | user address | totalCollateralBase, totalDebtBase, availableBorrowsBase, currentLiquidationThreshold, ltv, healthFactor |
| reserves | GraphQL (The Graph) | See subgraph endpoints in §4 | chain ID, optional asset filter | reserve list with liquidityRate, variableBorrowRate, utilizationRate, totalLiquidity, supplyCap, borrowCap |
| user-positions | GraphQL (The Graph) | See subgraph endpoints in §4 | user address, chain ID | supplied assets + aToken balance, borrowed assets + debt balance, health factor |
| reserve-detail | RPC eth_call | `AaveProtocolDataProvider.getReserveData(address asset)` | asset address | liquidity rate, variable borrow rate, stable borrow rate, liquidity index, aToken address |
| user-reserve-detail | RPC eth_call | `AaveProtocolDataProvider.getUserReserveData(address asset, address user)` | asset address, user address | currentATokenBalance, currentVariableDebt, scaledVariableDebt, usageAsCollateralEnabled |
| all-reserves | RPC eth_call | `UiPoolDataProvider.getReservesData(IPoolAddressesProvider provider)` | PoolAddressesProvider address | AggregatedReserveData[] with APY, caps, configuration per asset |

---

### On-chain Write Operations (via onchainos CLI)

| Operation | onchainos Command | Contract Method | Calldata Construction | Notes |
|-----------|------------------|-----------------|----------------------|-------|
| supply | `onchainos defi invest` | `Pool.supply(asset, amount, onBehalfOf, referralCode)` | via defi skill | Requires investment-id lookup; skill handles ERC-20 approval + supply |
| withdraw | `onchainos defi withdraw` | `Pool.withdraw(asset, amount, to)` | via defi skill | Pass `type(uint256).max` for full withdrawal |
| claim-rewards | `onchainos defi collect` | `RewardsController.claimAllRewards(assets[], to)` | via defi skill | Assets = array of aToken addresses for user's positions |
| borrow | `onchainos wallet contract-call` | `Pool.borrow(asset, amount, interestRateMode, referralCode, onBehalfOf)` | ABI encode in Rust (`ethabi`) | interestRateMode: 2=variable (1=stable deprecated); referralCode=0 |
| repay | `onchainos wallet contract-call` | `Pool.repay(asset, amount, interestRateMode, onBehalfOf)` | ABI encode in Rust | Use `uint256.max` for full repay; requires ERC-20 approval for debt token |
| set-collateral | `onchainos wallet contract-call` | `Pool.setUserUseReserveAsCollateral(asset, useAsCollateral)` | ABI encode in Rust | Boolean toggle; asset = ERC-20 token address |
| set-emode | `onchainos wallet contract-call` | `Pool.setUserEMode(categoryId)` | ABI encode in Rust | categoryId: 0=no emode, 1=stablecoins, 2=ETH correlated (chain-specific) |

---

### Pool Contract Function Signatures (for ABI encoding in Rust)

```solidity
// WRITE — borrow
function borrow(
    address asset,
    uint256 amount,
    uint256 interestRateMode,   // 2 = variable (1 = stable, deprecated)
    uint16 referralCode,        // 0
    address onBehalfOf
) external;

// WRITE — repay
function repay(
    address asset,
    uint256 amount,             // type(uint256).max for full repay
    uint256 interestRateMode,   // 2 = variable
    address onBehalfOf
) external returns (uint256);

// WRITE — set collateral
function setUserUseReserveAsCollateral(
    address asset,
    bool useAsCollateral
) external;

// WRITE — set E-Mode
function setUserEMode(uint8 categoryId) external;

// READ — health factor and account summary
function getUserAccountData(address user)
    external view returns (
        uint256 totalCollateralBase,
        uint256 totalDebtBase,
        uint256 availableBorrowsBase,
        uint256 currentLiquidationThreshold,
        uint256 ltv,
        uint256 healthFactor       // scaled 1e18; < 1e18 = liquidatable
    );
```

---

### PoolAddressesProvider Interface (for runtime Pool address resolution)

```solidity
function getPool() external view returns (address);
function getPoolDataProvider() external view returns (address);
function getPriceOracle() external view returns (address);
```

**Critical runtime flow in Rust binary:**
1. Read `POOL_ADDRESSES_PROVIDER` from config (per chain, static).
2. Call `PoolAddressesProvider.getPool()` → receive dynamic Pool address.
3. Use the Pool address for all subsequent `eth_call` or calldata submissions.

---

### Contract Address Resolution (CRITICAL — no hardcoding of Pool)

`PoolAddressesProvider` addresses per chain — these are the **immutable registry entry points** and are safe to store in config:

| Chain | Chain ID | PoolAddressesProvider Address |
|-------|----------|-------------------------------|
| Ethereum Mainnet | 1 | `0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e` |
| Polygon | 137 | `0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb` |
| Arbitrum One | 42161 | `0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb` |
| Base | 8453 | `0xe20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D` |

> Note: Polygon and Arbitrum share the same PoolAddressesProvider address (`0xa97684...`) — this is correct per Aave's cross-chain deployment pattern. Verify against the BGD Labs aave-address-book (`@aave-dao/aave-address-book`) before finalizing.

**At runtime**: Always call `PoolAddressesProvider.getPool()` to obtain the current Pool proxy address. Never cache or hardcode the Pool address itself.

---

## 3. User Scenarios

### Scenario 1 — Supply USDC to earn yield (Base)
- User says: *"Supply 1000 USDC to Aave on Base"*
- Agent action sequence:
  1. [off-chain query] Rust binary queries The Graph (Base subgraph) → confirm USDC reserve exists, fetch current supply APY
  2. [onchainos command] `onchainos defi invest --chain 8453 --protocol aave-v3 --asset USDC --amount 1000` → skill approves ERC-20 + calls `Pool.supply()`
  3. [confirmation] Return tx hash, new aUSDC balance, current APY

### Scenario 2 — Borrow ETH against USDC collateral (Arbitrum)
- User says: *"Borrow 0.5 ETH on Aave on Arbitrum using my USDC as collateral"*
- Agent action sequence:
  1. [off-chain query] Rust binary calls `Pool.getUserAccountData(userAddr)` on Arbitrum → check health factor and available borrow capacity
  2. [off-chain query] Rust binary queries subgraph → fetch WETH reserve, current variable borrow rate
  3. [onchainos command] Rust binary ABI-encodes `Pool.borrow(WETH, 0.5e18, 2, 0, userAddr)`, submits via `onchainos wallet contract-call --chain 42161 --to <Pool> --calldata <hex>`
  4. [confirmation] Return tx hash, new health factor, borrow APY

### Scenario 3 — Check health factor and liquidation risk (Ethereum)
- User says: *"What's my health factor on Aave Ethereum? Am I at risk of liquidation?"*
- Agent action sequence:
  1. [off-chain query] Rust binary resolves Pool address via `PoolAddressesProvider(0x2f39d...).getPool()`
  2. [off-chain query] Rust binary calls `Pool.getUserAccountData(userAddr)` → decode 6 return values
  3. [off-chain display] Return health factor (formatted as `healthFactor / 1e18`), total collateral in USD, total debt in USD, liquidation threshold, available borrowing capacity; flag if HF < 1.1 as "liquidation risk"

### Scenario 4 — Repay USDC debt fully (Polygon)
- User says: *"Repay all my USDC debt on Aave Polygon"*
- Agent action sequence:
  1. [off-chain query] Rust binary calls `AaveProtocolDataProvider.getUserReserveData(USDC, userAddr)` on Polygon → fetch `currentVariableDebt`
  2. [off-chain query] Check user USDC wallet balance via `onchainos wallet balance` → confirm sufficient funds
  3. [onchainos command] Rust binary ABI-encodes `Pool.repay(USDC, type(uint256).max, 2, userAddr)`, submits via `onchainos wallet contract-call --chain 137 --to <Pool> --calldata <hex>` (uses max uint256 to repay full balance regardless of accrued interest)
  4. [confirmation] Return tx hash, confirmed zero debt balance

### Scenario 5 — Full position overview across all chains
- User says: *"Show me all my Aave positions"*
- Agent action sequence:
  1. [onchainos command] `onchainos defi positions --protocol aave-v3` → returns summary from onchainos defi integration
  2. [off-chain query] Rust binary queries The Graph subgraphs for all 4 chains in parallel (Ethereum, Polygon, Arbitrum, Base), fetching user reserve data: supplied assets, borrowed assets, aToken balances, health factors per chain
  3. [off-chain display] Aggregate and display: per-chain breakdown of (a) supplied assets with APY and aToken balance, (b) borrowed assets with variable APR and debt balance, (c) health factor per chain, (d) total net worth in USD

### Scenario 6 — Enable E-Mode for stablecoin borrowing (Base)
- User says: *"Enable E-Mode on Aave Base so I can borrow more stablecoins"*
- Agent action sequence:
  1. [off-chain query] Rust binary queries UiPoolDataProvider or subgraph → list available E-Mode categories on Base (e.g., categoryId=1 for stablecoins)
  2. [off-chain display] Show user: current LTV vs E-Mode LTV (e.g., 80% → 97% for stablecoins), explain health factor impact
  3. [onchainos command] Rust binary ABI-encodes `Pool.setUserEMode(1)`, submits via `onchainos wallet contract-call --chain 8453 --to <Pool> --calldata <hex>`
  4. [confirmation] Return tx hash, new E-Mode category, updated borrow capacity

### Scenario 7 — Disable collateral to free an asset (Ethereum)
- User says: *"Disable my LINK as collateral on Aave Ethereum"*
- Agent action sequence:
  1. [off-chain query] Rust binary calls `Pool.getUserAccountData(userAddr)` → check if disabling LINK would drop health factor below 1.1 threshold; warn user if so
  2. [onchainos command] Rust binary ABI-encodes `Pool.setUserUseReserveAsCollateral(LINK_address, false)`, submits via `onchainos wallet contract-call --chain 1 --to <Pool> --calldata <hex>`
  3. [confirmation] Return tx hash, updated health factor

---

## 4. External API Dependencies

| API | Base URL | Purpose | Auth needed? |
|-----|----------|---------|-------------|
| Aave `UiPoolDataProvider` (all chains) | On-chain `eth_call` via onchainos RPC | `getReservesData(PoolAddressesProvider)` — returns all reserves with APY, caps, config in one call | No — uses onchainos managed RPC |
| Aave `AaveProtocolDataProvider` (all chains) | On-chain `eth_call` via onchainos RPC | `getUserReserveData(asset, user)` — per-asset user balance, debt, collateral flag | No — uses onchainos managed RPC |
| Aave `Pool` (all chains) | On-chain `eth_call` via onchainos RPC | `getUserAccountData(user)` — health factor, total collateral/debt, available borrows | No — uses onchainos managed RPC |
| Aave `PoolAddressesProvider` (all chains) | On-chain `eth_call` via onchainos RPC | `getPool()`, `getPoolDataProvider()` — runtime contract address resolution | No — uses onchainos managed RPC |
| onchainos defi registry | `onchainos defi search --platform aave --chain <id>` | Resolve investment-id per asset per chain for supply/withdraw/collect | No — onchainos CLI |

> **Decision: Option B (on-chain RPC, zero-config).** Plugin store users are not expected to manage API keys. The Graph subgraph is available as an optional enhancement — users who set `THEGRAPH_API_KEY` env var will get richer historical data, but it is not required. All default data flows use on-chain `eth_call` via the RPC already managed by onchainos.

---

## 5. Configuration Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| default_chain | 8453 | Default chain ID (Base) |
| interest_rate_mode | 2 | Interest rate mode: 1=stable (deprecated in V3.1+), 2=variable |
| referral_code | 0 | Aave referral code (0 = no referral) |
| dry_run | true | Simulate transaction without broadcasting |
| health_factor_warn_threshold | 1.1 | Warn user when health factor drops below this value |
| health_factor_danger_threshold | 1.05 | Block action and require explicit override when HF below this |
| thegraph_api_key | — | Optional: The Graph API key for richer historical queries (env `THEGRAPH_API_KEY`). Not required — all default flows use on-chain RPC. |
| pool_addresses_provider.1 | 0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e | PoolAddressesProvider registry — Ethereum |
| pool_addresses_provider.137 | 0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb | PoolAddressesProvider registry — Polygon |
| pool_addresses_provider.42161 | 0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb | PoolAddressesProvider registry — Arbitrum |
| pool_addresses_provider.8453 | 0xe20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D | PoolAddressesProvider registry — Base |

---

## 6. Agent Execution Guide

The plugin follows the standard 4-phase execution pipeline:

**Phase 1 — Intent Parsing**
- Identify operation: supply / withdraw / borrow / repay / positions / health-factor / reserves / claim-rewards / set-collateral / set-emode
- Extract parameters: asset symbol, amount, chain ID (default: Base), address (from wallet context)
- Validate: check chain is in supported list; normalize asset symbol to checksummed ERC-20 address via subgraph lookup

**Phase 2 — Pre-flight Checks (off-chain)**
- For write operations: call `Pool.getUserAccountData()` to check health factor
- For borrow: verify `availableBorrowsBase` covers requested amount
- For repay: verify wallet balance covers repay amount (or flag to get more)
- For set-collateral(false): simulate health factor impact before proceeding
- For supply/borrow: fetch current APY from subgraph to show user before confirming
- All pre-flight via Rust binary → RPC `eth_call` or The Graph GraphQL

**Phase 3 — Execution**
- Supply/Withdraw/Collect: delegate to `onchainos defi` skill with appropriate parameters
- Borrow/Repay/SetCollateral/SetEMode: Rust binary constructs calldata, submits via `onchainos wallet contract-call`
- All executions respect `dry_run=true` until user explicitly confirms

**Phase 4 — Confirmation + Display**
- Parse tx receipt, confirm success
- For supply: show new aToken balance, current supply APY
- For borrow: show new debt balance, variable borrow APY, updated health factor
- For repay: show remaining debt (should be 0 for full repay), updated health factor
- For health-factor check: display formatted summary card with all 6 return values from `getUserAccountData`

---

## 7. Open Questions

1. **Polygon vs Arbitrum PoolAddressesProvider collision**: Both chains currently show `0xa97684ead0e402dC232d5A977953DF7ECBaB3CDb`. This needs verification against BGD Labs `aave-address-book` (`AaveV3Arbitrum.POOL_ADDRESSES_PROVIDER` vs `AaveV3Polygon.POOL_ADDRESSES_PROVIDER`). If different, update config accordingly.

2. **onchainos defi `investment-id` mapping**: The `onchainos defi invest/withdraw` commands require an `investment-id` parameter. The Developer Agent needs to confirm how Aave V3 markets are registered in the onchainos defi registry — is there a per-chain, per-asset investment ID, or a single protocol-level ID? Check `onchainos defi list --protocol aave-v3`.

3. **Stable rate deprecation**: Aave V3.1 deprecated stable rate mode (interestRateMode=1). The binary should hard-block any attempt to borrow with mode=1 and always default to variable (mode=2). Confirm the exact V3 version deployed on each target chain.

4. **The Graph API key management**: The Graph subgraph queries require a paid API key. Confirm whether the onchainos plugin store provides a shared API key via environment injection, or whether the user must supply their own `THEGRAPH_API_KEY`. If no API key is available, fall back to direct on-chain `UiPoolDataProvider.getReservesData()` calls via RPC.

5. **ERC-20 approval for borrow repay**: `Pool.repay()` requires the caller to have approved the Pool contract to spend the repayment token. The Rust binary must check current allowance and, if insufficient, instruct the user to approve via `onchainos wallet approve` before repay. Clarify whether the binary can chain these into a single user-facing action.

6. **Rewards controller address**: The `claim-rewards` path uses `onchainos defi collect`, which should handle rewards internally. However, verify that the onchainos defi skill correctly targets the Aave `RewardsController` (not the deprecated `IncentivesController`). The RewardsController address is resolvable via `PoolAddressesProvider.getAddress(keccak256("INCENTIVES_CONTROLLER"))`.

7. **Community Rust crate viability**: The `aave-sdk` crate on crates.io is community-maintained and potentially unmaintained. The recommended approach is to build calldata encoding from scratch using `alloy-rs` (preferred) or `ethabi` crate with the function signatures documented in §2. Do not take a dependency on the community crate.

8. **Base as default chain justification**: Base is set as default due to lower gas costs and Aave's active incentive programs on Base. Confirm with product whether Ethereum should be default for institutional users or Base for retail users.

---

## 8. Reference Links

- Aave V3 Protocol Docs: https://aave.com/docs
- Aave V3 Core Contracts (origin): https://github.com/aave-dao/aave-v3-origin
- Aave Address Book (BGD Labs): https://github.com/bgd-labs/aave-address-book
- Aave Protocol Subgraphs: https://github.com/aave/protocol-subgraphs
- Aave SDK (official TypeScript): https://github.com/aave/aave-sdk
- Community MCP Server (TypeScript, Base): https://github.com/Tairon-ai/aave-mcp
- Community MCP Server (Python, multi-chain): https://github.com/kukapay/aave-mcp
- The Graph Explorer — Aave V3 Ethereum: https://thegraph.com/explorer/subgraphs/JCNWRypm7FYwV8fx5HhzZPSFaMxgkPuw4TnR3Gpi81zk
- The Graph Explorer — Aave V3 Base: https://thegraph.com/explorer/subgraphs/GQFbb95cE6d8mV989mL5figjaGaKCQB3xqYrr1bRyXqF
- Ethereum PoolAddressesProvider: https://etherscan.io/address/0x2f39d218133AFaB8F2B819B1066c7E434Ad94E9e
- Base PoolAddressesProvider: https://basescan.org/address/0xe20fCBdBfFC4Dd138cE8b2E6FBb6CB49777ad64D
- Pool ABI functions reference: https://aave.com/docs/aave-v3/smart-contracts/pool
- alloy-rs (recommended Rust Ethereum library): https://github.com/alloy-rs/alloy
