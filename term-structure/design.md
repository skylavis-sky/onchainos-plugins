# Term Structure (TermMax) — Plugin Design

## §0 Plugin Meta

- **plugin_name**: term-structure
- **dapp_name**: Term Structure (TermMax)
- **version**: 0.1.0
- **target_chains**: [arbitrum (chain 42161), ethereum (chain 1), bnb (chain 56)]
- **category**: defi-protocol
- **integration_path**: direct contract calls (Router + Market + Order + Viewer)

### Protocol Rebrand Note

Term Structure sunset its original auction-based protocol on **2025-02-19** and relaunched as **TermMax**, a customized Uniswap V3 AMM for fixed-rate lending/borrowing. The original TermAuctionOfferLocker / TermAuctionBidLocker architecture is retired. This plugin targets **TermMax V2** (the live protocol). GitHub: `term-structure/termmax-contract-v2`.

---

## §1 Feasibility

| Metric | Value |
|--------|-------|
| TVL (DeFiLlama, 2025-04) | ~$3.6M on Ethereum; multi-chain |
| Chains live | Arbitrum, Ethereum, BNB, Base, Berachain, Hyperliquid, X Layer, BSquared |
| On-chain reads | ✅ via MarketViewer / TermMaxViewer (eth_call) |
| On-chain writes | ✅ Router V1/V2 |
| Off-chain API | None documented; no official subgraph found |
| Orderbook type | **AMM-based** (customized Uniswap V3 curves), NOT auction-based periodic clearing |

### Feasibility Concerns

1. **Thin orderbook risk**: TVL is ~$3.6M, distributed across many markets and chains. Individual markets may have insufficient liquidity to fill large orders at the quoted APR.
2. **Market discovery**: No factory `getMarkets()` enumerator exists on-chain. Markets must be tracked by listening to factory events (`MarketCreated`) or from the deployment `.env` files in the GitHub repo. The plugin's `get-markets` command should maintain a curated list of known market addresses, refreshed periodically.
3. **Per-market contracts**: Each maturity×token-pair is its own `TermMaxMarket` with one or more `TermMaxOrder` AMM pools. The Router routes across these.
4. **AMM pricing, not sealed-bid**: Unlike the legacy auction system, rates are continuous and change with each trade (curve-based). There is no periodic auction window — orders execute immediately.
5. **FT/XT token model**: Lenders receive FT (Fixed-rate Token / bond token). Borrowers get GT (Gearing Token, an NFT). XT tokens are intermediary tokens used in the AMM. Positions are expressed in these internal tokens, not raw underlying.

---

## §2 Interface Mapping

### Token Architecture (critical context)

Each `TermMaxMarket` has:
- **Underlying**: the debt token (e.g. USDC, WETH)
- **Collateral**: collateral token (e.g. wstETH, ARB)
- **FT** (Fixed-rate Token): ERC-20 lender receipt, redeemable 1:1 for underlying at `maturity`
- **XT**: intermediary AMM token
- **GT** (Gearing Token): ERC-721 NFT, represents borrower's collateralized debt position
- **Maturity**: `uint64` Unix timestamp stored in `MarketConfig.maturity`

### On-chain Write Operations (EVM — Router)

All write operations go through the **Router** contract (V1 or V2). Markets are per-maturity deployments.

| Operation | Contract | Function Signature | Selector | Param Order |
|-----------|----------|--------------------|----------|-------------|
| Lend (buy FT) | RouterV1 | `swapExactTokenToToken(address tokenIn, address tokenOut, address recipient, address[] orders, uint128[] tradingAmts, uint128 minTokenOut, uint256 deadline)` | `0x1ac100a4` ✅ | tokenIn=underlying, tokenOut=FT address, recipient=wallet, orders=[orderAddr], tradingAmts=[amount], minTokenOut=slippage, deadline |
| Borrow (simple) | RouterV1 | `borrowTokenFromCollateral(address recipient, address market, uint256 collInAmt, uint256 borrowAmt)` | `0x95320fd0` ✅ | recipient=wallet, market=marketAddr, collInAmt=collateral amount (18-dec), borrowAmt=underlying to borrow |
| Borrow (via orders) | RouterV1 | `borrowTokenFromCollateral(address recipient, address market, uint256 collInAmt, address[] orders, uint128[] tokenAmtsWantBuy, uint128 maxDebtAmt, uint256 deadline)` | `0xfc1c1b21` ✅ | recipient, market, collInAmt, orders[], tokenAmtsWantBuy[], maxDebtAmt, deadline |
| Repay (flash from collateral) | RouterV1 | `flashRepayFromColl(address recipient, address market, uint256 gtId, address[] orders, uint128[] amtsToBuyFt, bool byDebtToken, (address,address,uint256,bytes)[] units, uint256 deadline)` | `0x867ba445` ✅ | recipient, market, gtId (NFT ID), orders[], amtsToBuyFt[], byDebtToken=false for normal, units=[], deadline |
| Repay via FT buy | RouterV1 | `repayByTokenThroughFt(address recipient, address market, uint256 gtId, address[] orders, uint128[] ftAmtsWantBuy, uint128 maxTokenIn, uint256 deadline)` | `0x84e09091` ✅ | recipient, market, gtId, orders[], ftAmtsWantBuy[], maxTokenIn, deadline |
| Sell FT early (exit lend) | RouterV1 | `sellTokens(address recipient, address market, uint128 ftInAmt, uint128 xtInAmt, address[] orders, uint128[] amtsToSell, uint128 minTokenOut, uint256 deadline)` | `0xc71c700c` ✅ | recipient, market, ftInAmt, xtInAmt=0 (lender exit), orders[], amtsToSell[], minTokenOut, deadline |
| Redeem FT at maturity | Market | `redeem(uint256 ftAmount, address recipient)` | `0x7bde82f2` ✅ | ftAmount, recipient=wallet; callable only after `maturity` timestamp |
| Redeem + swap | RouterV1 | `redeemAndSwap(address recipient, address market, uint256 ftAmount, (address,address,uint256,bytes)[] units, uint256 minTokenOut)` | `0xed303d94` ✅ | recipient, market, ftAmount, units=[] for no swap, minTokenOut |
| Create maker order | RouterV1 | `createOrderAndDeposit(address market, address maker, uint256 maxXtReserve, address swapTrigger, uint256 debtTokenToDeposit, uint128 ftToDeposit, uint128 xtToDeposit, CurveCuts curveCuts)` | `0xd84ec034` ✅ | Advanced: for market-making bots only |
| ERC-20 approve (pre-flight) | ERC-20 | `approve(address spender, uint256 amount)` | `0x095ea7b3` | spender=RouterV1 address; required before lend/borrow |

### On-chain Read Operations (eth_call)

| Operation | Contract | Function | Selector | Returns |
|-----------|----------|----------|----------|---------|
| Get market config (maturity, fees) | TermMaxMarket | `config()` | `0x79502c55` ✅ | `(address treasurer, uint64 maturity, FeeConfig feeConfig)` |
| Get market tokens | TermMaxMarket | `tokens()` | `0x9d63848a` ✅ | `(ft, xt, gt, collateral, underlying)` all ERC-20/ERC-721 addresses |
| Get order live APR | TermMaxOrder | `apr()` | `0x57ded9c9` ✅ | `(uint256 lendApr, uint256 borrowApr)` — scaled 1e18, annualized |
| Get order reserves | TermMaxOrder | `tokenReserves()` | `0x4bad9510` ✅ | `(uint256 ftReserve, uint256 xtReserve)` |
| Get user position in market | TermMaxViewer | `getPositionDetail(address market, address owner)` | `0x34c2cb2e` ✅ | `Position{underlyingBalance, collateralBalance, ftBalance, xtBalance, gtInfo[]}` |
| Get positions multi-market | TermMaxViewer | `getPositionDetails(address[] markets, address owner)` | `0x90711dc2` ✅ | `Position[]` |
| Get order state | TermMaxViewer | `getOrderState(address order)` | `0xd8d26d96` ✅ | `OrderState{ftReserve, xtReserve, maxXtReserve, curveCuts, feeConfig, ...}` |
| Get all borrow positions | TermMaxViewer | `getAllLoanPosition(address market, address owner)` | `0x255f1872` ✅ | `LoanPosition[]{loanId, collateralAmt, debtAmt}` |
| Get FT balance (lend position) | TermMaxMarket FT token | ERC-20 `balanceOf(address)` | `0x70a08231` | raw FT units held |
| Get maturity | TermMaxMarket | via `config()` `.maturity` field | `0x79502c55` | Unix timestamp |

### Off-chain Read Operations

No official REST API or subgraph has been documented for TermMax. Market discovery must be done by:
1. Querying the on-chain factory for `MarketCreated` events (event log scan).
2. Using the curated deployment `.env` files from `term-structure/termmax-contract-v2` repo as a seed list.
3. Calling `config()` + `tokens()` on each known market address to populate metadata.

---

## §3 User Scenarios

### Scenario A — Get Active Markets (AI rate survey)

```
get-markets --chain 42161
```

1. For each known market address in the curated list (seeded from deployment files):
   - `eth_call config()` → get maturity timestamp, filter out expired markets (maturity < now)
   - `eth_call tokens()` → get underlying token symbol and collateral token symbol
   - For each order within that market, `eth_call apr()` → get current lend/borrow APR
2. Return table: `market_address | underlying | collateral | maturity_date | lend_apr | borrow_apr | ft_liquidity`
3. Sort by `lend_apr` descending for rate optimization.

### Scenario B — AI Agent Rate Optimization (lend)

Agent wants to lend 1000 USDC at best fixed rate on Arbitrum:

1. `get-markets --chain 42161 --underlying USDC` → find market with highest `lend_apr` and sufficient liquidity
2. Check `tokenReserves()` on the best order — ensure `xtReserve > 0` (liquidity exists)
3. `approve` RouterV1 to spend 1000 USDC (ERC-20 approve)
4. `swapExactTokenToToken(USDC, FT_address, wallet, [orderAddr], [1000e6], minFTOut, deadline)` via RouterV1
5. Wallet receives FT tokens; hold until `maturity` then call `redeem(ftAmount, wallet)` to get USDC + interest

### Scenario C — Get User Position

```
get-position --chain 42161 --wallet 0xABC...
```

1. For each known market:
   - `eth_call getPositionDetail(market, wallet)` on TermMaxViewer
   - If `ftBalance > 0`: active lend position; compute yield = `ftBalance - underlyingEquivalent`
   - If `gtInfo.length > 0`: active borrow position with collateral
2. Return aggregate: `{market, maturity, type: "lend"|"borrow", amount, apy, maturity_date}`

### Scenario D — Borrow at Fixed Rate

```
submit-borrow-offer --market 0xMARKET --collateral 1.0 --borrow 500 --chain 42161
```

1. `approve` RouterV1 to spend collateral (e.g. wstETH)
2. `borrowTokenFromCollateral(wallet, market, collAmount, borrowAmount)` via RouterV1
3. Wallet receives USDC; GT NFT is minted (loanId returned)
4. At maturity, call `flashRepayFromColl(wallet, market, gtId, orders[], ...)` to repay

### Scenario E — Cancel / Exit Lend Early

FT can be sold back before maturity (secondary market exit):

```
cancel-offer --market 0xMARKET --ft-amount 1000 --chain 42161
```

1. `sellTokens(wallet, market, ftAmount, 0, [orderAddr], [ftAmount], minUnderlyingOut, deadline)` via RouterV1
2. Market price determines the effective realized rate (may be higher or lower than original APR depending on market moves)

### Scenario F — Redeem at Maturity

```
redeem --market 0xMARKET --chain 42161
```

1. Check `config().maturity < block.timestamp` (market has matured)
2. `eth_call balanceOf(wallet)` on FT token → get ftBalance
3. `redeem(ftBalance, wallet)` on TermMaxMarket (NOT router) → receive underlying + accrued interest

---

## §4 External API Dependencies

| Dependency | Purpose | Notes |
|------------|---------|-------|
| EVM RPC (Arbitrum) | All eth_call and contract-call ops | Use `https://arb1.arbitrum.io/rpc` or publicnode |
| EVM RPC (Ethereum) | Ethereum mainnet ops | Use `https://ethereum.publicnode.com` (avoid cloudflare-eth.com per kb) |
| EVM RPC (BNB) | BNB chain ops | Use `https://bsc-rpc.publicnode.com` (not bsc-dataseed.binance.org, TLS issue) |
| GitHub (seed data) | Market address list | `term-structure/termmax-contract-v2/deployments/` — read once at startup |
| No subgraph | — | No official subgraph; all market data via on-chain eth_call |

---

## §5 Config Parameters

```toml
[term-structure]
# Arbitrum core V2
arb_factory_v2        = "0x18b8A9433dBefcd15370F10a75e28149bcc2e301"
arb_vault_factory_v2  = "0xa7c93162962D050098f4BB44E88661517484C5EB"
arb_router_v1         = "0x7fa333b184868d88aC78a82eC06d5e87d4B0322E"
arb_router_v2         = "0xCAa5689bfe6E1B9c79D7C44D9E4410f6BFb6c4b5"
arb_termmax_viewer    = "0x012BFcbAC9EdEa04DFf07Cc61269E321f4595DfF"
arb_oracle_aggregator = "0xDF020051fc6f3378459bc9269372AA46fEEa77CA"
# Arbitrum core V1 (legacy markets still active)
arb_factory_v1        = "0x14920Eb11b71873d01c93B589b40585dacfCA096"
arb_market_viewer_v1  = "0x276C0E52508d94ff2D4106b1559c8c4Bc3a75dec"

# Ethereum mainnet core V2
eth_factory_v2        = "0xC53aB74EeB5E818147eb6d06134d81D3AC810987"
eth_vault_factory_v2  = "0x5b8B26a6734B5eABDBe6C5A19580Ab2D0424f027"
eth_router_v1         = "0xC47591F5c023e44931c78D5A993834875b79FB11"
eth_router_v2         = "0x324596C1682a5675008f6e58F9C4E0A894b079c7"
eth_termmax_viewer    = "0xf574c1d7C18E250c341bdFb478cafefcaCbAbF09"
eth_market_viewer     = "0x506a9Dd073D51FcC0BF96d26727928008c4C5Ba3"

# BNB chain core V2
bnb_factory_v2        = "0xdffE6De6de1dB8e1B5Ce77D3222eba401C2573b5"
bnb_router_v1         = "0xb7634dB4f4710bb992118bC37d1F63e00e2704A4"
bnb_router_v2         = "0xFB0c46985d937C755265f697BC10AD3387Ae801a"
bnb_termmax_viewer    = "0x80906014B577AFd760528FA8B32304A49806580C"

# Configurable
default_slippage_bps  = 50        # 0.5%
max_maturity_days     = 365
min_lend_amount_usd   = 10
```

---

## §6 Known Risks / Gotchas

### Protocol Architecture
1. **AMM not auction**: TermMax V2 uses a modified Uniswap V3 AMM with customized fixed-rate curves (`CurveCuts`). Rates are continuous; there is no auction clearing window. Orders execute immediately at the prevailing AMM price.
2. **Per-deployment contracts**: Every market (collateral × underlying × maturity date) is a separate set of contracts (`TermMaxMarket` + `TermMaxOrder`). There is no global registry with a `getMarkets()` call; the plugin must maintain a seeded list from factory events or GitHub deployment files.
3. **FT/XT/GT token model**: Lenders hold FT (ERC-20 bond token). Borrowers hold GT (ERC-721 NFT with loanId). The `redeem()` call requires the FT token balance. GT loanId must be fetched from `getPositionDetail()` before repayment.
4. **Maturity timestamp**: `MarketConfig.maturity` is a `uint64` Unix timestamp. After this timestamp, `redeem()` is callable on the market; before it, lenders can only exit via secondary `sellTokens()` at the prevailing AMM price.
5. **Early exit has price risk**: Selling FT before maturity via `sellTokens()` gives the AMM market price, which may be above or below par. If rates rose since lending, the lender will receive less than par value.

### Integration Gotchas
6. **ERC-20 approve required before every lend/borrow**: RouterV1 is the spender. Must approve the underlying (for lend) or collateral token (for borrow) before calling Router functions.
7. **Selector verification**: Selectors in §2 were computed using Python `eth_hash.auto.keccak` (Keccak-256, not NIST SHA3). Cross-check with `cast keccak` or `alloy-sol-types sol!` macro before finalizing implementation.
8. **V1 vs V2 Router**: Active markets may be under V1 or V2 factory. `arb_router_v1` (`0x7fa...`) is shared between V1 and V2 markets on Arbitrum. `arb_router_v2` handles V2-specific functions (ERC-4626 pool integration, etc.). For basic lend/borrow, RouterV1 functions cover both.
9. **Thin liquidity**: With ~$3.6M total TVL across 8 chains, individual market order depth may be insufficient for amounts > ~$50K. Always check `tokenReserves()` before quoting a fill.
10. **`getOrderState` address argument**: `TermMaxViewer.getOrderState(address order)` takes the `TermMaxOrder` contract address, not the market address. Each market can have multiple orders. Enumerate orders by listening to the `OrderCreated` event on the market contract.
11. **BSC RPC TLS**: `bsc-dataseed.binance.org` has TLS handshake issues in sandbox. Use `bsc-rpc.publicnode.com` (see KNOWLEDGE_HUB.md).
12. **Chain 1 `--output json` not supported**: `wallet balance --chain 1` fails with `--output json`. Use `wallet addresses` and filter by `chainIndex == "1"` for wallet resolution on Ethereum mainnet (see KNOWLEDGE_HUB.md).
13. **Decimal handling**: Underlying tokens (e.g. USDC = 6 decimals) vs FT tokens (inherits underlying decimals). Collateral tokens may be 18 decimals (wstETH, ARB). Always fetch decimals via `decimals()` on token contracts.

### Maturity Date Flow Summary
```
Announcement → Market deployed with maturity=T
         │
         ▼  (continuous, 24/7)
   AMM window: lend/borrow via Router swaps at live APR
         │
         ▼  at block.timestamp >= T
   Maturity: FT holders call market.redeem(ftAmt, wallet)
             GT holders (borrowers) must repay via flashRepayFromColl
             Uncollected positions accrue no additional interest after maturity
```
