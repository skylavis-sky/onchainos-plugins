# Exactly Protocol — Plugin Design

## §0 Plugin Meta

- **plugin_name**: exactly-protocol
- **dapp_name**: Exactly Protocol
- **version**: 0.1.0
- **target_chains**: optimism (chain 10), ethereum (chain 1)
- **category**: defi-protocol
- **integration_path**: direct contract calls (no SDK required)
- **primary_chain**: optimism (chain 10) — lower gas, higher TVL activity
- **github**: https://github.com/exactly-finance/protocol
- **docs**: https://docs.exact.ly

---

## §1 Feasibility

| # | Item | Finding |
|---|------|---------|
| 1 | Protocol type | Fixed-rate, fixed-term lending via maturity-based pools (ERC-4626 FixedRatePool). Variable-rate (floating) pools also available as secondary feature. |
| 2 | Contract architecture | Market contracts (one per asset) handle all lending/borrowing. Auditor manages collateral and health checks. Previewer provides read-only market/position data. MarketETHRouter wraps native ETH for WETH market. |
| 3 | SDK requirement | None. All operations are direct EVM contract calls. No off-chain SDK, no oracle pre-call required for user-facing operations. |
| 4 | Contract verification | All contracts verified on Optimism and Ethereum Etherscan. Transparent upgradeable proxies (EIP-1967). |
| 5 | Key contracts on Optimism | Auditor `0xaEb62e6F27BC103702E7BC879AE98bceA56f027E`, MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF`, MarketUSDC `0x6926B434CCe9b5b7966aE1BfEef6D0A7DCF3A8bb`, Previewer `0x328834775A18A4c942F30bfd091259ade4355C2a` |
| 6 | Fixed-rate maturity model | Maturities are weekly/monthly timestamps (Unix epoch). Users deposit or borrow at a specific maturity; position is locked until that date. Early exit available with market-discount penalty. |
| 7 | Collateral flow | User must call `Auditor.enterMarket(marketAddr)` to enable a deposited asset as collateral before borrowing. Collateral is tracked per Market. |

**Integration path decision**: Direct EVM contract calls to Market contracts via `onchainos wallet contract-call`. The Previewer contract provides all read-only data (rates, positions, market state) via a single `exactly(address)` call. No subgraph or REST API is needed for core operations.

---

## §2 Interface Mapping

### Operations Table

| Operation | Type | Contract | Notes |
|-----------|------|----------|-------|
| `get-markets` | read | Previewer | Returns all markets with rates per maturity via `exactly(address)` |
| `get-position` | read | Previewer | Returns user's fixed and floating positions across all markets via `exactly(address)` |
| `deposit` | write | Market | ERC-4626 `deposit(assets, receiver)` — floating-rate pool |
| `borrow` | write | Market | `borrow(assets, receiver, borrower)` — floating-rate pool |
| `repay` | write | Market | `refund(borrowShares, borrower)` — repay floating-rate borrow |
| `withdraw` | write | Market | ERC-4626 `withdraw(assets, receiver, owner)` — floating-rate pool |
| `deposit-fixed` | write | Market | `depositAtMaturity(maturity, assets, minAssetsRequired, receiver)` — fixed-rate pool |
| `borrow-fixed` | write | Market | `borrowAtMaturity(maturity, assets, maxAssets, receiver, borrower)` — fixed-rate pool |
| `repay-fixed` | write | Market | `repayAtMaturity(maturity, positionAssets, maxAssets, borrower)` — fixed-rate pool |
| `withdraw-fixed` | write | Market | `withdrawAtMaturity(maturity, positionAssets, minAssetsRequired, receiver, owner)` — fixed-rate pool |
| `enter-market` | write | Auditor | `enterMarket(market)` — enable asset as collateral |
| `exit-market` | write | Auditor | `exitMarket(market)` — remove asset as collateral (requires zero debt) |

---

### On-chain Write Operations (EVM)

All write operations require ERC-20 `approve(market, amount)` before deposit/repay calls (except `borrow`, `withdraw`, `enter-market`, `exit-market`).

| Operation | Contract Address (Optimism) | Function Signature | Selector (verified via eth_utils keccak256) | ABI Param Order |
|-----------|-----------------------------|--------------------|----------------------------------------------|-----------------|
| `deposit` (floating) | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `deposit(uint256,address)` | `0x6e553f65` | assets, receiver |
| `withdraw` (floating) | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `withdraw(uint256,address,address)` | `0xb460af94` | assets, receiver, owner |
| `borrow` (floating) | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `borrow(uint256,address,address)` | `0xd5164184` | assets, receiver, borrower |
| `refund` (floating repay) | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `refund(uint256,address)` | `0x7ad226dc` | borrowShares, borrower |
| `deposit-fixed` | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `depositAtMaturity(uint256,uint256,uint256,address)` | `0x34f7d1f2` | maturity, assets, minAssetsRequired, receiver |
| `borrow-fixed` | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `borrowAtMaturity(uint256,uint256,uint256,address,address)` | `0x1a5b9e62` | maturity, assets, maxAssets, receiver, borrower |
| `repay-fixed` | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `repayAtMaturity(uint256,uint256,uint256,address)` | `0x3c6f317f` | maturity, positionAssets, maxAssets, borrower |
| `withdraw-fixed` | MarketWETH `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | `withdrawAtMaturity(uint256,uint256,uint256,address,address)` | `0xa05a091a` | maturity, positionAssets, minAssetsRequired, receiver, owner |
| `enter-market` | Auditor `0xaEb62e6F27BC103702E7BC879AE98bceA56f027E` | `enterMarket(address)` | `0x3fe5d425` | market |
| `exit-market` | Auditor `0xaEb62e6F27BC103702E7BC879AE98bceA56f027E` | `exitMarket(address)` | `0xede4edd0` | market |
| ERC-20 approve (pre-step) | underlying token (e.g. WETH, USDC) | `approve(address,uint256)` | `0x095ea7b3` | spender (market addr), amount |

> **Note**: The same Market addresses work for both WETH and USDC operations — just replace the contract address with the relevant market. Apply the function signatures identically.

---

### Market Contract Addresses (Optimism — chain 10)

| Market | Underlying Asset | Contract Address |
|--------|-----------------|-----------------|
| MarketWETH | WETH (`0x4200000000000000000000000000000000000006`) | `0xc4d4500326981eacD020e20A81b1c479c161c7EF` |
| MarketUSDC | USDC (`0x0b2c639c533813f4aa9d7837caf62653d097ff85`) | `0x6926B434CCe9b5b7966aE1BfEef6D0A7DCF3A8bb` |
| MarketOP | OP (`0x4200000000000000000000000000000000000042`) | `0xa430A427bd00210506589906a71B54d6C256CEdb` |
| MarketwstETH | wstETH (`0x1F32b1c2345538c0c6f582fCB022739c4A194Ebb`) | `0x22ab31Cd55130435b5efBf9224b6a9d5EC36533F` |
| MarketWBTC | WBTC (`0x68f180fcCe6836688e9084f035309E29Bf0A2095`) | `0x6f748FD65d7c71949BA6641B3248C4C191F3b322` |
| MarketETHRouter | native ETH wrapper | `0x29bAbFF3eBA7B517a75109EA8fd6D1eAb4A10258` |
| Auditor | — | `0xaEb62e6F27BC103702E7BC879AE98bceA56f027E` |
| Previewer | — | `0x328834775A18A4c942F30bfd091259ade4355C2a` |

### Market Contract Addresses (Ethereum Mainnet — chain 1)

| Market | Contract Address |
|--------|-----------------|
| MarketWETH | `0xc4d4500326981eacD020e20A81b1c479c161c7EF` (same as Optimism — deterministic CREATE2) |
| MarketUSDC | `0x660e2fC185a9fFE722aF253329CEaAD4C9F6F928` |
| MarketwstETH | `0x3843c41DA1d7909C86faD51c47B9A97Cf62a29e1` |
| MarketWBTC | `0x8644c0FDED361D1920e068bA4B09996e26729435` |
| MarketETHRouter | `0x29bAbFF3eBA7B517a75109EA8fd6D1eAb4A10258` (same as Optimism) |
| Auditor | `0x310A2694521f75C7B2b64b5937C16CE65C3EFE01` |
| Previewer | `0x5fE09baAa75fd107a8dF8565813f66b3603a13D3` |

---

### Off-chain Read Operations

All read operations use `eth_call` against the Previewer contract. No REST API or subgraph is required.

| Operation | Method | Contract | Selector | Input | Output |
|-----------|--------|----------|----------|-------|--------|
| `get-markets` | `eth_call` | Previewer | `0x157c9e0e` (`exactly(address)`) | account address (use `address(0)` for market-only query) | `MarketAccount[]` — all markets with rates, maturity pools, TVL |
| `get-position` | `eth_call` | Previewer | `0x157c9e0e` (`exactly(address)`) | user wallet address | `MarketAccount[]` — with user's fixed/floating deposit and borrow positions |
| `preview-deposit-fixed` | `eth_call` | Previewer | `0x8eab0a2a` (`previewDepositAtMaturity(address,uint256,uint256)`) | market, maturity, assets | `FixedPreview` — projected yield at maturity |
| `preview-borrow-fixed` | `eth_call` | Previewer | `0x109c46ce` (`previewBorrowAtMaturity(address,uint256,uint256)`) | market, maturity, assets | `FixedPreview` — total owed at maturity |
| `preview-all-deposit-maturities` | `eth_call` | Previewer | `0xb15a3f61` (`previewDepositAtAllMaturities(address,uint256)`) | market, assets | `FixedPreview[]` — rates across all active maturities |
| `preview-all-borrow-maturities` | `eth_call` | Previewer | `0xb18ba945` (`previewBorrowAtAllMaturities(address,uint256)`) | market, assets | `FixedPreview[]` — cost across all active maturities |
| `account-liquidity` | `eth_call` | Auditor | `0x9e9d7967` (`accountLiquidity(address,address,uint256)`) | account, market, withdrawAssets | (sumCollateral, sumDebt) — USD-denominated health check |
| `asset-price` | `eth_call` | Auditor | `0xb883b058` (`assetPrice(address)`) | priceFeed address | uint256 price with 18 decimals |

**RPC endpoints**:
- Optimism: `https://mainnet.optimism.io` (primary), `https://optimism.publicnode.com` (fallback)
- Ethereum: `https://ethereum.publicnode.com` (avoid `llamarpc.com` — rate-limits under multi-call load)

---

## §3 User Scenarios

### Scenario 1: Fixed-Rate Deposit (Lend at Fixed Rate)

> "I want to lend 1000 USDC for 4 weeks at the best fixed rate on Optimism."

1. `get-markets` — call `Previewer.exactly(address(0))` on chain 10; parse `FixedPool[]` for MarketUSDC to show rates per maturity.
2. User selects maturity (e.g., Unix timestamp for 28 days from now, rounded to weekly interval).
3. `preview-deposit-fixed` — call `Previewer.previewDepositAtMaturity(MarketUSDC, maturity, 1000e6)` to show projected return at maturity.
4. ERC-20 `approve(MarketUSDC, 1000e6)` on USDC contract. Wait ~3 seconds (nonce collision guard).
5. `deposit-fixed` — call `Market.depositAtMaturity(maturity, 1000e6, minAssetsRequired, receiver)`. `minAssetsRequired` = 99% of preview output (1% slippage tolerance).
6. Confirm txHash. Position is locked until maturity; user receives principal + fixed yield at expiry.

**Approval note**: `deposit-fixed` and `deposit` (floating) both require ERC-20 approval for the Market contract. `borrow` and `withdraw` do NOT require approval.

---

### Scenario 2: Fixed-Rate Borrow

> "I have deposited WETH as collateral. I want to borrow 500 USDC for 2 weeks at a fixed rate."

1. `get-position` — call `Previewer.exactly(wallet)` on chain 10; verify user has WETH floating deposit and has entered the WETH market as collateral.
2. If `isCollateral == false` for WETH market: `enter-market` — call `Auditor.enterMarket(MarketWETH)`.
3. `preview-borrow-fixed` — call `Previewer.previewBorrowAtMaturity(MarketUSDC, maturity, 500e6)` to show total owed at maturity (principal + interest).
4. `borrow-fixed` — call `Market.borrowAtMaturity(maturity, 500e6, maxAssets, receiver, borrower)`. `maxAssets` = 101% of preview output (1% slippage tolerance). **No approval needed**.
5. USDC is transferred to `receiver`; debt is recorded at `maturity`.

---

### Scenario 3: Repay Fixed-Rate Loan Before/At Maturity

> "My USDC borrow is due in 3 days. I want to repay it now."

1. `get-position` — call `Previewer.exactly(wallet)`; locate the fixed borrow position with its `maturity` timestamp and `positionAssets` (principal + fee).
2. Call `Previewer.previewRepayAtMaturity(MarketUSDC, maturity, positionAssets, wallet)` to get the actual repay cost (may be discounted if repaying early).
3. **Pitfall — "repay-all" overflow**: Do NOT pass `uint256.max` as `positionAssets` — the contract will attempt to pull the full debt amount, which may exceed wallet balance if interest has accrued since last preview. Instead, pass the `positionAssets` from `get-position` directly, or fetch wallet's actual USDC balance and pass `min(positionAssets, walletBalance)`.
4. ERC-20 `approve(MarketUSDC, repayAmount)` on USDC. Wait ~3 seconds.
5. `repay-fixed` — call `Market.repayAtMaturity(maturity, positionAssets, maxAssets, borrower)`. `maxAssets` = repayAmount with 1% buffer.
6. After maturity with no repayment, penalty fees accrue daily (`penaltyRate`). Repay as soon as possible to stop fee accrual.

---

### Scenario 4: Withdraw Fixed Deposit at or Before Maturity

> "My 4-week USDC deposit matures tomorrow. I want to withdraw early."

1. `get-position` — parse user's `fixedDeposits` in MarketUSDC; get `maturity` and `position.principal + position.fee`.
2. Call `Previewer.previewWithdrawAtMaturity(MarketUSDC, maturity, positionAssets, wallet)` to see the early-withdrawal discount amount (market deducts a fee for breaking the fixed term).
3. **Pitfall — early withdrawal discount**: `withdrawAtMaturity` before maturity returns fewer assets than deposited principal + yield. The discount is dynamic (depends on current market utilization). Inform the user of the penalty before proceeding.
4. `withdraw-fixed` — call `Market.withdrawAtMaturity(maturity, positionAssets, minAssetsRequired, receiver, owner)`. Set `minAssetsRequired` to 99% of preview output.
5. After the maturity timestamp, no discount is applied — full principal + yield is returned.

---

### Scenario 5: Floating Rate Deposit + Borrow (Variable Rate)

> "I want to deposit WETH to earn variable yield, then borrow USDC at variable rate."

1. `get-markets` — show floating rate APY for WETH (deposit) and USDC (borrow) via `exactly(address(0))`.
2. ERC-20 `approve(MarketWETH, wethAmount)`. Wait 3 seconds.
3. `deposit` — call `Market.deposit(wethAmount, receiver)`. Receive exaWETH voucher shares.
4. `enter-market` — call `Auditor.enterMarket(MarketWETH)` to enable WETH as collateral.
5. `borrow` — call `MarketUSDC.borrow(usdcAmount, receiver, borrower)`. No approval needed.
6. To repay: call `MarketUSDC.refund(borrowShares, borrower)` (not `repay` — Exactly uses `refund` for floating repay). Need USDC approval for `refund`.
7. To withdraw: call `MarketWETH.withdraw(assets, receiver, owner)`. Requires clearing all debt first (health factor check); same pitfall as Aave `withdraw(uint256.max)`.

---

## §4 External API Dependencies

| Dependency | URL | Purpose | Required? |
|------------|-----|---------|-----------|
| Optimism RPC | `https://mainnet.optimism.io` | All on-chain reads and writes (chain 10) | Yes |
| Ethereum RPC | `https://ethereum.publicnode.com` | Ethereum mainnet ops (chain 1) | For chain 1 ops |
| Previewer contract | on-chain (chain 10 or 1) | Market data, rates, user positions | Yes (reads) |
| Market contracts | on-chain (chain 10 or 1) | All deposit/borrow/repay/withdraw | Yes (writes) |
| Auditor contract | on-chain (chain 10 or 1) | Collateral management, health checks | Yes (enter/exit market) |

No off-chain REST APIs, no subgraph, no SDK endpoint required.

---

## §5 Config Parameters

| Parameter | Default (Optimism) | Notes |
|-----------|-------------------|-------|
| `chain_id` | `10` | Optimism mainnet; use `1` for Ethereum |
| `rpc_url` | `https://mainnet.optimism.io` | Swap for `https://ethereum.publicnode.com` on chain 1 |
| `auditor_address` | `0xaEb62e6F27BC103702E7BC879AE98bceA56f027E` | Chain-dependent |
| `previewer_address` | `0x328834775A18A4c942F30bfd091259ade4355C2a` | Chain-dependent |
| `market_weth` | `0xc4d4500326981eacD020e20A81b1c479c161c7EF` | Same address both chains (deterministic) |
| `market_usdc` | `0x6926B434CCe9b5b7966aE1BfEef6D0A7DCF3A8bb` | Optimism; `0x660e2fC185a9fFE722aF253329CEaAD4C9F6F928` on chain 1 |
| `market_eth_router` | `0x29bAbFF3eBA7B517a75109EA8fd6D1eAb4A10258` | Same address both chains |
| `slippage_bps` | `100` | 1% — used for `minAssetsRequired` / `maxAssets` slippage guards |
| `approve_delay_secs` | `3` | Delay between ERC-20 approve and deposit/repay call |
| `max_maturity_count` | `8` | Max maturities to display in `get-markets` output |

---

## §6 Known Risks / Gotchas

### 1. Fixed-Rate Repay Overflow (repay-all pitfall)
Passing the exact `positionAssets` from `get-position` to `repayAtMaturity` can revert if even 1 wei of penalty has accrued since the last read. Always preview via `previewRepayAtMaturity` immediately before the repay call and pass the preview's `assets` value as `positionAssets`, with a 0.1% buffer added to `maxAssets`. This mirrors the Aave V3 `repay(uint256.max)` pitfall documented in `kb/protocols/lending.md`.

### 2. Early Withdrawal Discount
`withdrawAtMaturity` before the maturity timestamp applies a market-determined discount — the user receives LESS than their deposited principal + yield. The discount magnitude depends on current pool utilization. Always call `previewWithdrawAtMaturity` first and display the penalty to the user explicitly. Never call `withdrawAtMaturity` without user confirmation of the discount.

### 3. Maturity Timestamps Are Weekly Intervals
Maturity dates are fixed Unix timestamps set by the protocol, NOT arbitrary user-chosen dates. They are aligned to weekly boundaries (e.g., every Thursday UTC). Always read available maturities from `Previewer.exactly()` (the `FixedPool[].maturity` array) and present them to the user for selection. Passing an invalid maturity timestamp will revert.

### 4. `enterMarket` Required Before Borrowing
Unlike some protocols (e.g., Aave V3 which auto-marks supply as collateral), Exactly requires an explicit `Auditor.enterMarket(market)` call before a deposit in that market counts as collateral. Skipping this causes the borrow health check to fail silently (the deposited asset is ignored in collateral calculations). Always check `isCollateral` flag from `Previewer.exactly()` and call `enterMarket` if `false`.

### 5. Floating Repay Uses `refund`, Not `repay`
The floating-rate repay function is `refund(borrowShares, borrower)`, NOT a `repay(assets)` function. It takes `borrowShares` (the share token amount) rather than underlying asset amount. Use `Previewer.exactly(wallet)` to read the user's `floatingBorrowShares` and pass that value. ERC-20 approval IS required for `refund`.

### 6. `withdraw` (Floating) Requires Zero Debt
Withdrawing all floating-rate collateral while outstanding debt exists will revert due to health factor check (same pattern as Aave V3 `withdraw(uint256.max)` pitfall). Clear all floating and fixed borrows before withdrawing collateral. Document this explicitly in the skill.

### 7. Penalty Fees Accrue After Maturity
If a fixed-rate borrower does not repay by the maturity timestamp, a daily penalty rate (`penaltyRate` storage variable) is applied. The total owed increases each day. Warn users when `block.timestamp > maturity` during `get-position` and surface the accrued penalty in the output.

### 8. `deposit` vs `depositAtMaturity` — Distinct Pools
Floating deposits (`deposit`) go into the floating pool and earn variable yield. Fixed deposits (`depositAtMaturity`) go into a specific maturity pool and earn a locked fixed yield. These are completely separate positions. `withdraw` and `withdrawAtMaturity` operate on different pools and cannot be swapped.

### 9. ERC-20 Approval Race Condition
Apply the 3-second delay between `approve` and any Market call that pulls tokens (`deposit`, `depositAtMaturity`, `repayAtMaturity`, `refund`). Submitting both in the same second causes nonce collision or "replacement transaction underpriced" errors (documented pattern in `kb/protocols/lending.md`).

### 10. Same Contract Addresses on Both Chains (Deterministic Deployment)
MarketWETH (`0xc4d45...`) and MarketETHRouter (`0x29bAb...`) share the same address on Optimism and Ethereum mainnet. This is by design (CREATE2 deterministic deployment). Always pair addresses with the correct `chain_id` to avoid cross-chain confusion in config.

### 11. Native ETH Deposits Use MarketETHRouter
To deposit native ETH (not WETH) into the WETH market, users must call `MarketETHRouter.deposit(assets, receiver)` with `msg.value = assets`. Direct calls to `MarketWETH.deposit` require WETH ERC-20, not native ETH. For simplicity, the plugin should accept native ETH and route through `MarketETHRouter`.

### 12. Python `hashlib.sha3_256` Produces Wrong Selectors
Python's `hashlib.sha3_256` is NIST SHA3, NOT Ethereum Keccak-256. All selectors in this document were computed using `eth_utils.keccak(text=sig)` which produces correct Ethereum keccak256. Never use `hashlib.sha3_256` for selector computation (see `KNOWLEDGE_HUB.md` row 76).
