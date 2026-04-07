---
name: spectra
description: "Spectra Finance yield tokenization plugin. Deposit ERC-4626 assets to receive PT (fixed yield) and YT (variable yield). Redeem PT for underlying at maturity. Claim accrued yield from YT. Swap PT for IBT via Curve StableSwap. Trigger phrases: Spectra deposit, Spectra redeem, claim yield Spectra, Spectra PT, Spectra YT, fixed yield Base, yield tokenization, buy PT Spectra, sell PT Spectra, Spectra pools, Spectra position."
trigger_phrases:
  - "Spectra"
  - "spectra finance"
  - "spectra pt"
  - "spectra yt"
  - "deposit spectra"
  - "redeem spectra"
  - "claim yield spectra"
  - "swap pt spectra"
  - "fixed yield base"
  - "yield tokenization"
  - "principal token"
license: MIT
metadata:
  author: GeoGu360
  version: "0.1.0"
---

## Architecture

Spectra Finance has NO hosted SDK or API for calldata generation (unlike Pendle). All operations use direct ABI-encoded calls to PrincipalToken contracts or the Router execute dispatcher.

- Read ops (`get-pools`, `get-position`) — `eth_call` against Base RPC; `get-pools` tries the Spectra app data API first, falls back to on-chain Registry enumeration
- Write ops (`deposit`, `redeem`, `claim-yield`, `swap`) — ABI-encoded calldata submitted via `onchainos wallet contract-call --force`
- Approve before write ops — ERC-20 `approve(spender, max_uint256)` submitted automatically when required
- `--dry-run` is handled in the plugin wrapper; never passed to the onchainos CLI

## Supported Chains

| Chain | Chain ID | Status |
|-------|---------|--------|
| Base (default) | 8453 | Primary — active pools, low gas |
| Arbitrum | 42161 | Secondary |
| Ethereum | 1 | Available |

## Command Routing

| User intent | Command |
|-------------|---------|
| List Spectra pools / what pools exist / APY | `get-pools` |
| My Spectra positions / PT balance / YT balance / pending yield | `get-position` |
| Deposit / lock in fixed yield / buy PT+YT | `deposit` |
| Redeem PT at maturity / exit fixed yield | `redeem` |
| Claim accrued yield from YT | `claim-yield` |
| Swap PT for IBT / sell PT early / buy PT | `swap` |

## Do NOT use for

- Pendle Finance operations (use `pendle` plugin)
- Adding or removing Curve liquidity (not exposed in this skill — use Curve plugin)
- Yield strategies on Aave/Compound directly (use those plugins)
- Chains other than Base, Arbitrum, or Ethereum

## Execution Flow for Write Operations

1. Run with `--dry-run` first to preview calldata and estimated output
2. Show the user: amount in, expected PT shares (or underlying out), maturity date, implied APY
3. **Ask user to confirm** before executing on-chain
4. Execute only after explicit user approval
5. Report approve tx hash (if any), main tx hash, and outcome

---

## Commands

### get-pools — List Spectra PT Pools

**Trigger phrases:** "list Spectra pools", "show Spectra pools", "Spectra APY", "what pools does Spectra have", "Spectra markets Base"

```bash
spectra [--chain 8453] get-pools [--active-only] [--limit <N>]
```

**Parameters:**
- `--chain` — chain ID (default 8453 = Base)
- `--active-only` — filter expired pools
- `--limit` — max results (default 20)

**Example:**
```bash
spectra --chain 8453 get-pools --active-only --limit 10
```

**Output:** JSON with `pools` array. Each pool: `name`, `pt`, `yt`, `ibt`, `underlying`, `curve_pool`, `maturity_ts`, `days_to_maturity`, `active`, `apy`, `tvl_usd`.

---

### get-position — View Wallet Positions

**Trigger phrases:** "my Spectra positions", "what PT do I hold Spectra", "Spectra portfolio", "pending yield Spectra", "YT balance Spectra"

```bash
spectra [--chain 8453] get-position [--user <ADDRESS>]
```

**Parameters:**
- `--user` — wallet address (defaults to logged-in wallet)

**Example:**
```bash
spectra --chain 8453 get-position --user 0xYourWallet
```

**Output:** For each held PT/YT: balances, pending yield in IBT, redemption value, maturity status.

---

### deposit — Deposit to Get PT + YT

**Trigger phrases:** "deposit Spectra", "buy PT Spectra", "lock fixed yield Spectra", "tokenize yield Spectra", "Spectra deposit WETH"

```bash
spectra [--chain 8453] [--dry-run] deposit \
  --pt <PT_ADDRESS> \
  --amount <AMOUNT_WEI> \
  [--use-ibt] \
  [--receiver <ADDRESS>] \
  [--from <ADDRESS>] \
  [--slippage 0.005]
```

**Parameters:**
- `--pt` — PrincipalToken contract address (required; resolve from `get-pools`)
- `--amount` — amount in wei (underlying asset by default; IBT if `--use-ibt`)
- `--use-ibt` — deposit IBT directly (skip underlying-to-IBT wrapping)
- `--receiver` — PT and YT recipient (default: sender)
- `--from` — sender wallet (default: logged-in wallet)
- `--slippage` — slippage tolerance, default 0.005 (0.5%)

**Example (deposit 0.01 WETH into weETH pool):**
```bash
spectra --chain 8453 --dry-run deposit \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --amount 10000000000000000 \
  --from 0xYourWallet
```

**Steps executed:**
1. Calls `previewDeposit(amount)` to estimate PT shares
2. Approves underlying/IBT for PT contract (max uint256)
3. Calls `deposit(assets, ptReceiver, ytReceiver, minShares)` selector `0xe4cca4b0` on PT

**Note:** Deposits are blocked post-maturity. Will error if PT has expired.

---

### redeem — Redeem PT for Underlying

**Trigger phrases:** "redeem Spectra PT", "exit fixed yield Spectra", "Spectra matured", "claim PT Spectra", "redeem after maturity Spectra"

```bash
spectra [--chain 8453] [--dry-run] redeem \
  --pt <PT_ADDRESS> \
  --shares <SHARES_WEI> \
  [--receiver <ADDRESS>] \
  [--owner <ADDRESS>] \
  [--from <ADDRESS>] \
  [--slippage 0.005]
```

**Parameters:**
- `--pt` — PrincipalToken contract address
- `--shares` — PT amount to redeem in wei
- `--receiver` — underlying recipient (default: sender)
- `--owner` — owner of PT shares (default: sender)

**Post-expiry:** calls `redeem(shares, receiver, owner, minAssets)` selector `0x9f40a7b3`

**Pre-expiry:** calls `withdraw(assets, receiver, owner)` selector `0xb460af94` — requires equal YT balance

**Example (redeem 0.01 PT post-maturity):**
```bash
spectra --chain 8453 --dry-run redeem \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --shares 9999999999999999 \
  --from 0xYourWallet
```

---

### claim-yield — Claim Accrued Yield from YT

**Trigger phrases:** "claim yield Spectra", "collect Spectra yield", "Spectra YT yield", "how much yield Spectra", "claim Spectra accrued yield"

```bash
spectra [--chain 8453] [--dry-run] claim-yield \
  --pt <PT_ADDRESS> \
  [--in-ibt] \
  [--receiver <ADDRESS>] \
  [--from <ADDRESS>]
```

**Parameters:**
- `--pt` — PrincipalToken contract address (yield is claimed via PT, not YT)
- `--in-ibt` — receive yield as IBT instead of underlying
- `--receiver` — yield recipient (default: sender)

**Example:**
```bash
spectra --chain 8453 --dry-run claim-yield \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --from 0xYourWallet
```

**Steps:**
1. Calls `getCurrentYieldOfUserInIBT(user)` selector `0x0e1b6d89` to preview pending yield
2. If yield > 0: calls `claimYield(receiver)` selector `0x999927df` (or `claimYieldInIBT` `0x0fba731e` if `--in-ibt`)

---

### swap — Swap PT via Curve (Router)

**Trigger phrases:** "sell PT Spectra", "buy PT Spectra Curve", "exit PT early Spectra", "swap Spectra PT", "sell Spectra PT before maturity"

```bash
spectra [--chain 8453] [--dry-run] swap \
  --pt <PT_ADDRESS> \
  --amount-in <AMOUNT_WEI> \
  [--sell-pt] \
  [--min-out <MIN_WEI>] \
  [--curve-pool <POOL_ADDRESS>] \
  [--from <ADDRESS>] \
  [--slippage 0.01]
```

**Parameters:**
- `--pt` — PrincipalToken address
- `--amount-in` — amount to sell (PT wei if `--sell-pt`; IBT wei otherwise)
- `--sell-pt` — sell PT for IBT (omit to buy PT with IBT)
- `--min-out` — minimum output in wei (0 = auto from slippage)
- `--curve-pool` — Curve pool address (auto-resolved for known pools)
- `--slippage` — default 0.01 (1%)

**Router execute pattern (TRANSFER_FROM + CURVE_SWAP_SNG):**
- Command bytes: `[0x00, 0x1E]` (TRANSFER_FROM=0x00, CURVE_SWAP_SNG=0x1E)
- weETH Curve pool layout: coins(0)=IBT, coins(1)=PT
- Sell PT: i=1, j=0; Buy PT: i=0, j=1

**Example (sell 0.01 PT for IBT):**
```bash
spectra --chain 8453 --dry-run swap \
  --pt 0x07f58450a39d07f9583c188a2a4a441fac358100 \
  --amount-in 10000000000000000 \
  --sell-pt \
  --from 0xYourWallet
```

**Steps:**
1. Approves token_in for Router contract
2. Calls Router `execute(bytes,bytes[])` selector `0x24856bc3` with encoded TRANSFER_FROM + CURVE_SWAP_SNG commands
