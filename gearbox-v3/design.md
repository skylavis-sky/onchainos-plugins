# Gearbox V3 — Plugin Design

## §0 Plugin Meta

- **plugin_name**: gearbox-v3
- **dapp_name**: Gearbox V3
- **version**: 0.1.0
- **target_chains**: [arbitrum (chain 42161), ethereum (chain 1)]
- **category**: defi-protocol
- **integration_path**: direct contract calls (no SDK required)

---

## §1 Feasibility

**Overall verdict: FEASIBLE with caveats**

Gearbox V3 uses a Credit Account architecture where every user operation (open account, add collateral, borrow, withdraw) is executed through the `CreditFacadeV3` contract via `multicall`. The `openCreditAccount` and `closeCreditAccount` functions both accept a `MultiCall[]` array — meaning even a "simple" open requires encoding at least one inner multicall call (e.g. `increaseDebt`).

**What is feasible for v0.1:**
- `get-pools` — Off-chain reads of pool state via DataCompressor or direct PoolV3 calls
- `get-account` — Off-chain reads via DataCompressor or `CreditManagerV3.calcDebtAndCollateral()`
- `open-account` — On-chain write via `CreditFacadeV3.openCreditAccount()` with a minimal multicall (just `increaseDebt` + `addCollateral`)
- `add-collateral` — On-chain write via `CreditFacadeV3.multicall()` with inner `addCollateral` call
- `close-account` — On-chain write via `CreditFacadeV3.closeCreditAccount()` with inner `decreaseDebt` + `withdrawCollateral` calls
- `withdraw` — On-chain write via `CreditFacadeV3.multicall()` with inner `withdrawCollateral` call

**Complexity note (critical for developer):**
The `multicall` parameter takes `(address target, bytes callData)[]`. For operations on the facade itself (addCollateral, increaseDebt, etc.), `target` = the CreditFacadeV3 address, and `callData` = the ABI-encoded inner function call. The ABI encoding requires nested struct encoding. See §2 and §6 for details.

**ERC-20 approval required before `addCollateral`:**
The user must approve the `CreditManagerV3` (not the facade) to spend their token before adding collateral.

**Quota system:**
Non-underlying collateral tokens require a "quota" to count toward health factor. For v0.1, we only support depositing the underlying token as collateral (no quota management needed), avoiding this complexity.

---

## §2 Interface Mapping

### Operations Table

| Operation | Type | Contract | Notes |
|-----------|------|----------|-------|
| `get-pools` | off-chain read | DataCompressor / PoolV3 | Enumerate pools, borrow APR, liquidity |
| `get-account` | off-chain read | DataCompressor / CreditManagerV3 | Account debt, collateral, health factor |
| `open-account` | on-chain write | CreditFacadeV3 | multicall: increaseDebt + addCollateral |
| `add-collateral` | on-chain write | CreditFacadeV3 | multicall: addCollateral (+ ERC-20 approve) |
| `close-account` | on-chain write | CreditFacadeV3 | multicall: decreaseDebt + withdrawCollateral |
| `withdraw` | on-chain write | CreditFacadeV3 | multicall: withdrawCollateral |

---

### On-chain Write Operations (EVM)

> All write operations target **CreditFacadeV3** on Arbitrum (preferred chain).
> The user must first ERC-20 `approve` the **CreditManagerV3** address (not the facade) before adding collateral.

| Operation | Contract Address (Arbitrum, source) | Function Signature | Selector (keccak256 verified) | Param Order |
|-----------|-------------------------------------|--------------------|-------------------------------|-------------|
| `open-account` | CreditFacadeV3 — varies by CM (see §5) | `openCreditAccount(address onBehalfOf, (address target, bytes callData)[] calls, uint256 referralCode)` | `0x92beab1d` | onBehalfOf (user addr), calls array, referralCode (0) |
| `close-account` | CreditFacadeV3 — varies by CM | `closeCreditAccount(address creditAccount, (address target, bytes callData)[] calls)` | `0x36b2ced3` | creditAccount addr, calls array |
| `multicall` (add/withdraw) | CreditFacadeV3 — varies by CM | `multicall(address creditAccount, (address target, bytes callData)[] calls)` | `0xebe4107c` | creditAccount addr, calls array |

#### Inner multicall call encodings (callData contents, target = CreditFacadeV3 address)

| Inner Operation | Function Signature | Selector | Param Order |
|-----------------|--------------------|----------|-------------|
| Add collateral | `addCollateral(address token, uint256 amount)` | `0x6d75b9ee` | token addr, amount (in token decimals) |
| Increase debt (borrow) | `increaseDebt(uint256 amount)` | `0x2b7c7b11` | amount in underlying decimals |
| Decrease debt (repay) | `decreaseDebt(uint256 amount)` | `0x2a7ba1f7` | amount in underlying decimals; use `type(uint256).max` to repay all |
| Withdraw collateral | `withdrawCollateral(address token, uint256 amount, address to)` | `0x1f1088a0` | token addr, amount (`type(uint256).max` = all), recipient addr |
| Update quota | `updateQuota(address token, int96 quotaChange, uint96 minQuota)` | `0x712c10ad` | token addr, quota delta, min quota (use 0) |

**Encoding example for `open-account` (deposit 1000 USDC, borrow 4000 USDC = 5x leverage):**
```
openCreditAccount(
  onBehalfOf = <user_wallet>,
  calls = [
    { target: <CreditFacadeV3_addr>,
      callData: abi_encode(increaseDebt(4000_000000))   // borrow 4000 USDC
    },
    { target: <CreditFacadeV3_addr>,
      callData: abi_encode(addCollateral(USDC_addr, 1000_000000))  // deposit 1000 USDC
    }
  ],
  referralCode = 0
)
```
Note: `increaseDebt` must come before `addCollateral` in the open flow per Gearbox docs.

---

### Off-chain Read Operations

| Operation | Contract | Address (Arbitrum) | Function Signature | Selector | Returns |
|-----------|----------|--------------------|--------------------|----------|---------|
| List pools | DataCompressor | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` | `getPoolsV3List()` | `0xa0f068df` | array of PoolData structs |
| Pool detail | DataCompressor | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` | `getPoolData(address pool)` | `0x13d21cdf` | PoolData struct |
| List credit managers | DataCompressor | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` | `getCreditManagersV3List()` | `0xc7fd2b45` | array of CreditManagerData structs |
| Credit manager detail | DataCompressor | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` | `getCreditManagerData(address cm)` | `0xae093f3f` | CreditManagerData struct |
| Account data | DataCompressor | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` | `getCreditAccountData(address creditAccount, (address,uint256,bytes)[] priceUpdates)` | `0x7b7f70d6` | CreditAccountData (debt, collateral, HF, tokens) |
| Accounts by borrower | DataCompressor | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` | `getCreditAccountsByBorrower(address borrower, (address,uint256,bytes)[] priceUpdates)` | `0x16e5b9f1` | array of CreditAccountData |
| Debt + collateral (on CM) | CreditManagerV3 | varies by CM | `calcDebtAndCollateral(address creditAccount, uint8 task)` | `0x0d334ca6` | CollateralDebtData struct |
| Is liquidatable | CreditManagerV3 | varies by CM | `isLiquidatable(address creditAccount, uint16 minHealthFactor)` | `0x8340e24d` | bool |
| Pool borrow rate | PoolV3 | varies by pool | `baseInterestRate()` | `0xafd92762` | uint256 (ray, 1e27 = 100%) |
| Pool supply rate | PoolV3 | varies by pool | `supplyRate()` | `0xad2961a3` | uint256 (ray) |
| Pool available liq | PoolV3 | varies by pool | `availableLiquidity()` | `0x74375359` | uint256 |
| Debt limits | CreditFacadeV3 | varies by CM | `debtLimits()` | `0x166bf9d9` | (uint128 minDebt, uint128 maxDebt) |

**DataCompressor note:** Pass an empty array `[]` for `priceUpdates` on `getCreditAccountData` and `getCreditAccountsByBorrower` when using standard tokens that don't require on-demand oracle updates. This is the common case for USDC/WETH accounts.

---

## §3 User Scenarios

### Scenario A — Check available pools
```
User: "Show me Gearbox pools on Arbitrum"
Plugin: get-pools --chain 42161
→ Calls DataCompressor.getPoolsV3List() on Arbitrum
→ For each pool: name, underlying token, borrow APR (baseInterestRate_ray / 1e27 * 100%), 
  available liquidity, total borrowed
→ Also shows linked Credit Managers and their min/max debt limits
```

### Scenario B — Check existing Credit Account health
```
User: "What's my Gearbox position?"
Plugin: get-account --chain 42161
→ resolve_wallet() → wallet address
→ Calls DataCompressor.getCreditAccountsByBorrower(wallet, [])
→ Returns: Credit Account address, debt amount, collateral value (USD), 
  health factor (twvUSD / totalDebtUSD), list of held tokens
→ If no accounts: "No open Credit Accounts found"
```

### Scenario C — Open a leveraged Credit Account
```
User: "Open a Gearbox credit account with 1000 USDC at 3x leverage on Arbitrum"
Plugin: open-account --pool usdc --collateral 1000 --leverage 3 --chain 42161

Flow:
1. Prompt user to confirm (E106 compliance)
2. resolve_wallet() → user address
3. Determine CM: "Trade USDC Tier 2 Arbitrum" (lower minimum $1, good for testing)
4. borrow_amount = collateral * (leverage - 1) = 2000 USDC
5. Check debtLimits() — ensure borrow_amount within [minDebt, maxDebt]
6. ERC-20 approve: USDC.approve(creditManagerV3, collateral_amount)
7. openCreditAccount(userAddr, [increaseDebt(2000e6), addCollateral(USDC, 1000e6)], 0)
8. Return: Credit Account address, total position size (3000 USDC), health factor estimate
```

### Scenario D — Add collateral to existing account
```
User: "Add 500 USDC collateral to my Gearbox account 0xABC..."
Plugin: add-collateral --account 0xABC... --token usdc --amount 500 --chain 42161

Flow:
1. Prompt user to confirm
2. ERC-20 approve: USDC.approve(creditManagerV3, 500e6)
3. multicall(0xABC..., [addCollateral(USDC, 500e6)])
```

### Scenario E — Close Credit Account
```
User: "Close my Gearbox account 0xABC..."
Plugin: close-account --account 0xABC... --chain 42161

Flow:
1. Prompt user to confirm
2. get-account to fetch current debt
3. User must have sufficient underlying to repay — check wallet balance
4. closeCreditAccount(0xABC..., [
     decreaseDebt(type(uint256).max),   // repay all debt
     withdrawCollateral(underlying, type(uint256).max, userAddr)  // withdraw all
   ])
Note: User needs enough underlying token in their wallet to cover debt repayment.
      If no underlying in wallet, they must add collateral first and do a swap via multicall 
      (complex — out of scope for v0.1, document this limitation).
```

### Scenario F — Withdraw collateral
```
User: "Withdraw 200 USDC from my Gearbox account 0xABC..."
Plugin: withdraw --account 0xABC... --token usdc --amount 200 --chain 42161

Flow:
1. Prompt user to confirm
2. multicall(0xABC..., [withdrawCollateral(USDC, 200e6, userAddr)])
3. Health factor is checked post-call — if HF < 1, tx reverts
```

---

## §4 External API Dependencies

| Dependency | Purpose | URL | Notes |
|------------|---------|-----|-------|
| DataCompressor (on-chain) | Batch read of pool/CM/account state | Arbitrum RPC eth_call | Preferred for reads — single call returns all data |
| CreditFacadeV3 (on-chain) | Write operations | Arbitrum RPC eth_sendTransaction | One facade per Credit Manager |
| CreditManagerV3 (on-chain) | On-chain reads as fallback | Arbitrum RPC eth_call | Can use calcDebtAndCollateral directly |
| PoolV3 (on-chain) | Pool rates and liquidity | Arbitrum RPC eth_call | ERC-4626 compatible |
| Gearbox Subgraph | Historical data, indexing | https://api.thegraph.com/subgraphs/name/gearbox-protocol/gearbox-v3-arbitrum | Optional — not required for v0.1 |

**Recommended RPC for Arbitrum:** `https://arb1.arbitrum.io/rpc` or `https://arbitrum.publicnode.com`

No external REST API required for core operations — all reads use `eth_call` to DataCompressor or individual contracts.

---

## §5 Config Parameters

### Arbitrum (chain 42161) — Deployed Addresses (from dev-docs stateArbitrum.json, block 239832594 / Aug 2024)

#### Core Infrastructure
| Contract | Address |
|----------|---------|
| AddressProviderV3 | `0x7d04ecdb892ae074f03b5d0aba03796f90f3f2af` |
| ContractsRegister | `0xc3e00cda97d5779bfc8f17588d55b4544c8a6c47` |
| DataCompressor v3 | `0x88aa4FbF86392cBF6f6517790E288314DE03E181` |
| AccountFactory | `0x03cd6b5c36c15b9feed278c417274902609e5df9` |
| PriceOracle v3 | `0xF6C709a419e18819dea30248f59c95cA20fd83d5` |

#### Pools (Arbitrum)
| Pool Name | Pool Address | Underlying | Underlying Address |
|-----------|-------------|------------|-------------------|
| Main USDC.e v3 | `0xa76c604145D7394DEc36C49Af494C144Ff327861` | USDC.e | `0xFF970A61A04b1cA14834A43f5dE4533eBDDB5CC8` |
| Main WETH v3 | `0x04419d3509f13054f60d253E0c79491d9E683399` | WETH | `0x82aF49447D8a07e3bd95BD0d56f35241523fBab1` |
| Main USDC v3 | `0x890A69EF363C9c7BdD5E36eb95Ceb569F63ACbF6` | USDC (native) | `0xaf88d065e77c8cC2239327C5EDb3A432268e5831` |

#### Credit Managers (Arbitrum)
| Name | CreditManager | CreditFacade | Underlying | Pool | minDebt | maxDebt |
|------|--------------|--------------|-----------|------|---------|---------|
| Trade USDC.e Tier 1 | `0x75bc0fef1c93723be3d73b2000b5ba139a0c680c` | `0x026329e9b559ece6eaab765e6d3aa6aaa7d01e11` | USDC.e | Main USDC.e v3 | 5,000 USDC.e | 100,000 USDC.e |
| Trade USDC.e Tier 2 | `0xb4bc02c0859b372c61abccfa5df91b1ccaa4dd1f` | `0x8d5d92d4595fdb190d41e1a20f96a0363f17f72c` | USDC.e | Main USDC.e v3 | 5,000 USDC.e | 25,000 USDC.e |
| Trade WETH Tier 1 | `0xcedaa4b4a42c0a771f6c24a3745c3ca3ed73f17a` | `0x7d4a58b2f09f97537310a31e77ecd41e7d0dcbfa` | WETH | Main WETH v3 | 7 WETH | 150 WETH |
| Trade WETH Tier 2 | `0x3ab1d35500d2da4216f5863229a7b81e2f6ff976` | `0xf1fada023dd48b9bb5f52c10b0f833e35d1c4c56` | WETH | Main WETH v3 | 0.35 WETH | 7 WETH |
| Trade USDC Tier 1 | `0xe5e2d4bb15d26a6036805fce666c5488367623e2` | `0xbe0715eceadb3b238be599bbdb30bea28a3ebef6` | USDC | Main USDC v3 | 20,000 USDC | 400,000 USDC |
| Trade USDC Tier 2 | `0xb780dd9cec259a0bbf7b32587802f33730353e86` | `0x3974888520a637ce73bdcb2ee28a396f4b303876` | USDC | Main USDC v3 | 1,000 USDC | 20,000 USDC |

**For v0.1 testing:** Use **Trade USDC.e Tier 2** (minDebt 5,000 USDC.e, maxDebt 25,000) or **Trade USDC Tier 2** (minDebt 1,000 USDC, maxDebt 20,000). The USDC Tier 2 CM has the lowest minimum debt ($1,000), making it the best choice for integration testing.

**Recommended default CM for user-facing operations:** Trade USDC Tier 2 (USDC, `creditFacade=0x3974888520a637ce73bdcb2ee28a396f4b303876`)

---

## §6 Known Risks / Gotchas

### G1 — Multicall nested ABI encoding (HIGH complexity)
All CreditFacadeV3 write functions require a `MultiCall[]` parameter:
```solidity
struct MultiCall { address target; bytes callData; }
```
The `callData` is itself an ABI-encoded function call (e.g. `addCollateral(token, amount)`). Developer must encode:
1. The outer call to `openCreditAccount` / `closeCreditAccount` / `multicall`
2. Each inner `MultiCall` element — with `target = creditFacadeAddr` and `callData = abi_encode(innerFn(args))`

In Rust with `alloy-sol-types`, this requires two levels of `sol!{}` macro usage. See Gearbox SDK and alloy docs for reference.

### G2 — Approve CreditManager, NOT CreditFacade
ERC-20 approvals for `addCollateral` must target the **CreditManagerV3** address, not the CreditFacadeV3 address. The facade calls `transferFrom` via the manager.

### G3 — Minimum debt enforcement
Opening an account requires borrowing at least `minDebt` (from `debtLimits()`). For Trade USDC.e Tier 2, this is 5,000 USDC.e. The contract reverts if debt is below minimum. Always check `debtLimits()` before submitting.

### G4 — Close account requires underlying token balance
To close an account, the user must have enough underlying in their wallet to repay debt (debt = principal + accrued interest). The simple close flow only works if the user has external funds. Closing by liquidating collateral within the account requires additional multicall steps (swaps via adapters) that are out of scope for v0.1. **Document this limitation in SKILL.md.**

### G5 — Quota system for non-underlying collateral
Adding a non-underlying token (e.g., WBTC to a USDC account) requires `updateQuota()` to enable it as counting collateral. Without a quota, the token is on the account but provides zero HF value — creating liquidation risk if debt is outstanding. For v0.1, only support underlying token as collateral. **Explicitly document that multi-token collateral requires quota management.**

### G6 — Health factor monitoring / liquidation risk
Gearbox positions are actively liquidatable by third parties when HF < 1. Positions can be liquidated if collateral price drops or borrow APR accrues. The plugin should always display current HF after writes and warn users when HF drops below 1.1 (close to liquidation).

### G7 — Address staleness
Addresses above are from block 239832594 (Aug 2024). Gearbox governance can add new CMs/pools. Always call `DataCompressor.getCreditManagersV3List()` at runtime to enumerate current CMs rather than hard-coding all addresses. The core infrastructure addresses (DataCompressor, AddressProvider) are stable.

### G8 — multicall gas complexity
The `openCreditAccount` with multicall can use significant gas (~500k–1M gas) due to the solvency check at the end. Always use sufficient gas limits.

### G9 — `decreaseDebt(type(uint256).max)` for full repayment
Per the interface docs, passing an amount greater than the account's total debt triggers full repayment. Use `u256::MAX` for repay-all operations (same pattern as Aave, but Gearbox explicitly documents this behavior).

### G10 — Contracts may be paused
CreditFacadeV3 has `isPaused` flag. Always check `cf.isPaused` before attempting writes and return a clear error message to the user rather than letting the tx revert.

### G11 — DataCompressor `priceUpdates` parameter
`getCreditAccountData` and `getCreditAccountsByBorrower` take a `PriceOnDemand[]` parameter. Pass an empty array `[]` for normal tokens (USDC, WETH). If the RPC reverts with "price feed stale" errors, some tokens in the account may require on-demand price updates (this is rare for major assets).

---

## §7 Selector Verification Summary

All selectors computed via `keccak256(signature)[:4]` using `pycryptodome` (verified against known selectors transfer=0xa9059cbb, balanceOf=0x70a08231).

| Function | Signature | Selector |
|----------|-----------|----------|
| openCreditAccount | `openCreditAccount(address,(address,bytes)[],uint256)` | `0x92beab1d` |
| closeCreditAccount | `closeCreditAccount(address,(address,bytes)[])` | `0x36b2ced3` |
| multicall | `multicall(address,(address,bytes)[])` | `0xebe4107c` |
| addCollateral (inner) | `addCollateral(address,uint256)` | `0x6d75b9ee` |
| withdrawCollateral (inner) | `withdrawCollateral(address,uint256,address)` | `0x1f1088a0` |
| increaseDebt (inner) | `increaseDebt(uint256)` | `0x2b7c7b11` |
| decreaseDebt (inner) | `decreaseDebt(uint256)` | `0x2a7ba1f7` |
| updateQuota (inner) | `updateQuota(address,int96,uint96)` | `0x712c10ad` |
| calcDebtAndCollateral | `calcDebtAndCollateral(address,uint8)` | `0x0d334ca6` |
| isLiquidatable | `isLiquidatable(address,uint16)` | `0x8340e24d` |
| baseInterestRate | `baseInterestRate()` | `0xafd92762` |
| supplyRate | `supplyRate()` | `0xad2961a3` |
| availableLiquidity | `availableLiquidity()` | `0x74375359` |
| debtLimits | `debtLimits()` | `0x166bf9d9` |
| getPoolsV3List | `getPoolsV3List()` | `0xa0f068df` |
| getCreditManagersV3List | `getCreditManagersV3List()` | `0xc7fd2b45` |
| getPoolData | `getPoolData(address)` | `0x13d21cdf` |
| getCreditManagerData | `getCreditManagerData(address)` | `0xae093f3f` |
| getCreditAccountData | `getCreditAccountData(address,(address,uint256,bytes)[])` | `0x7b7f70d6` |
| getCreditAccountsByBorrower | `getCreditAccountsByBorrower(address,(address,uint256,bytes)[])` | `0x16e5b9f1` |
