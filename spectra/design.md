# Spectra — Plugin Design

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `spectra` |
| `dapp_name` | Spectra Finance |
| `version` | 0.1.0 |
| `category` | defi-protocol |
| `tags` | yield-tokenization, fixed-yield, pt, yt, interest-rate-derivatives |
| `target_chains` | Base (chain 8453), Arbitrum (chain 42161) |
| `primary_test_chain` | Base (chain 8453) — lower gas, active pools |
| `integration_path` | direct contract calls |
| `source_repo` | GeoGu360/onchainos-plugins |
| `source_dir` | spectra |

---

## §1 Feasibility

| Check | Result |
|-------|--------|
| Rust SDK? | **No** — no official Rust SDK. Protocol is EVM smart contracts only. |
| Official SDK? | **No** — no TypeScript SDK either (unlike Pendle). Protocol is designed for direct contract interaction. |
| REST API? | **No public API** — unlike Pendle, Spectra has no hosted SDK API endpoint that generates calldata. All operations require direct contract calls. The app at `app.spectra.finance` has a private Next.js data endpoint (`/_next/data/.../pools.json`) that returns live pool data, usable for read operations. |
| Official Skill? | No |
| Rust SDK needed? | No — direct ABI encoding via `wallet contract-call --input-data` |
| Supported chains? | Ethereum (1), Arbitrum (42161), Base (8453), BSC (56), Optimism (10), Sonic (146), Avalanche (43114), and more. OnchainOS target subset: **Base (8453) primary, Arbitrum (42161) secondary** |
| onchainos broadcast needed? | Yes — all write operations use `wallet contract-call` |
| Pendle competition? | Yes — Spectra is the main Pendle competitor. Key difference: **no hosted SDK**, all operations are direct contract calls using a Router command dispatcher pattern (vs Pendle's REST API calldata generation). |

### Integration Path

**Direct contract calls only** — Spectra requires ABI-encoding the `execute(bytes commands, bytes[] inputs)` dispatcher pattern on the Router, or calling PrincipalToken functions directly for deposit/redeem/claimYield. No external API generates calldata for us.

Two interaction modes:
1. **Via Router** (`execute`): Handles complex multi-step operations (add/remove liquidity, IBT wrapping, combined Curve swaps)
2. **Direct PT calls**: `deposit`, `depositIBT`, `redeem`, `withdraw`, `claimYield` — simpler operations callable directly on the PT contract

---

## §2 Interface Mapping

### Operations Table

| Operation | Type | Description | Complexity |
|-----------|------|-------------|-----------|
| `get-pools` | Off-chain read | List active Spectra pools with APY, maturity, TVL | Low |
| `get-position` | Off-chain read | User's PT/YT holdings (balances + yield) | Low |
| `deposit` | On-chain write | Deposit underlying asset → receive PT + YT | Medium |
| `redeem` | On-chain write | Redeem PT → underlying (post-expiry) or PT+YT → underlying (pre-expiry) | Medium |
| `swap-pt` | On-chain write | Swap IBT ↔ PT via Curve AMM pool | High (Router dispatcher) |
| `claim-yield` | On-chain write | Claim accrued yield from YT holdings | Low |

---

### On-chain Write Operations (EVM)

All selectors verified: (a) computed with `Crypto.Hash.keccak` (NOT Python `hashlib.sha3_256` which is NIST SHA3, not Keccak-256), (b) confirmed via 4byte.directory, (c) confirmed via live `eth_call` on Base mainnet.

| Operation | Contract Address (source) | Function Signature | Selector (cast sig verified) | Param Order |
|-----------|--------------------------|-------------------|------------------------------|-------------|
| `deposit` (underlying → PT+YT) | PT contract (per-pool, see §5) | `deposit(uint256,address,address,uint256)` | `0xe4cca4b0` ✅ | `assets, ptReceiver, ytReceiver, minShares` |
| `depositIBT` (IBT → PT+YT) | PT contract (per-pool) | `depositIBT(uint256,address,address,uint256)` | `0x2a412806` ✅ | `ibts, ptReceiver, ytReceiver, minShares` |
| `redeem` (post-expiry, PT → underlying) | PT contract | `redeem(uint256,address,address,uint256)` | `0x9f40a7b3` ✅ | `shares, receiver, owner, minAssets` |
| `redeem` (post-expiry, no slippage guard) | PT contract | `redeem(uint256,address,address)` | `0xba087652` ✅ | `shares, receiver, owner` |
| `redeemForIBT` (PT → IBT) | PT contract | `redeemForIBT(uint256,address,address)` | `0xb2afd5a3` ✅ | `shares, receiver, owner` |
| `withdraw` (pre-expiry, PT+YT → underlying) | PT contract | `withdraw(uint256,address,address)` | `0xb460af94` ✅ | `assets, receiver, owner` |
| `claimYield` (claim in underlying) | PT contract | `claimYield(address)` | `0x999927df` ✅ | `receiver` |
| `claimYieldInIBT` (claim in IBT) | PT contract | `claimYieldInIBT(address)` | `0x0fba731e` ✅ | `receiver` |
| `swap-pt` (execute Router commands) | Router (see §5) | `execute(bytes,bytes[])` | `0x24856bc3` ✅ | `commands, inputs` |
| `swap-pt` (with deadline) | Router | `execute(bytes,bytes[],uint256)` | `0x3593564c` ✅ | `commands, inputs, deadline` |
| `approve` (ERC-20) | IBT/PT/YT token | `approve(address,uint256)` | `0x095ea7b3` ✅ | `spender, amount` |

#### Router Dispatcher Commands (for swap-pt via execute)

Spectra Router uses a command-dispatcher pattern. `commands` is a `bytes` sequence where each byte is a command enum value. `inputs` is a parallel `bytes[]` of ABI-encoded parameters for each command.

| Command Byte | Command Name | ABI-encoded Parameters |
|---|---|---|
| Requires separate constant from Dispatcher.sol | `CURVE_SWAP` | `(address pool, uint256 i, uint256 j, uint256 amountIn, uint256 minAmountOut, address recipient)` |
| — | `DEPOSIT_ASSET_IN_IBT` | `(address ibt, uint256 assets, address recipient)` |
| — | `DEPOSIT_ASSET_IN_PT` | `(address pt, uint256 assets, address ptRecipient, address ytRecipient, uint256 minShares)` |
| — | `DEPOSIT_IBT_IN_PT` | `(address pt, uint256 ibts, address ptRecipient, address ytRecipient, uint256 minShares)` |
| — | `REDEEM_PT_FOR_ASSET` | `(address pt, uint256 shares, address recipient, uint256 minAssets)` |
| — | `REDEEM_PT_FOR_IBT` | `(address pt, uint256 shares, address recipient, uint256 minIbts)` |
| — | `TRANSFER_FROM` | `(address token, uint256 value)` |
| — | `CURVE_SPLIT_IBT_LIQUIDITY` | `(address pool, uint256 ibts, address recipient, address ytRecipient, uint256 minPTShares)` |
| — | `CURVE_ADD_LIQUIDITY` | `(address pool, uint256[2] amounts, uint256 min_mint_amount, address recipient)` |
| — | `CURVE_REMOVE_LIQUIDITY` | `(address pool, uint256 lps, uint256[2] min_amounts, address recipient)` |
| — | `ASSERT_MIN_BALANCE` | `(address token, address owner, uint256 minValue)` |

> **Note:** The actual command byte values (enum integers) must be read from the Router's Dispatcher.sol source at `github.com/perspectivefi/spectra-core/src/router/Dispatcher.sol`. At implementation time, fetch the enum values before encoding commands. For `swap-pt`, the typical flow is: `TRANSFER_FROM` → `DEPOSIT_ASSET_IN_IBT` (wrap underlying to IBT if needed) → `CURVE_SWAP` (IBT↔PT in Curve pool).

---

### Off-chain Read Operations

| Operation | Method | Details |
|-----------|--------|---------|
| `get-pools` | HTTP GET `app.spectra.finance/_next/data/{buildId}/pools.json` | Returns all pools with PT address, YT address, IBT address, underlying, Curve pool address, maturity timestamp, APY, TVL. Filter by `chainId == 8453` for Base. See §4. |
| `get-position` PT balance | `onchainos wallet balance --chain 8453` | Lists all ERC-20 balances including PT tokens |
| `get-position` YT balance | `onchainos wallet balance --chain 8453` | YT tokens appear in ERC-20 balance list |
| `get-position` pending yield | `eth_call` → `getCurrentYieldOfUserInIBT(address)` | Selector `0x0e1b6d89` on PT contract; returns pending yield in IBT units |
| `get-position` maturity check | `eth_call` → `maturity()` | Selector `0x204f83f9` on PT; returns Unix timestamp |
| `previewDeposit` | `eth_call` → `previewDeposit(uint256)` | Selector `0xef8b30f7`; simulates PT shares minted for `assets` deposited |
| `previewRedeem` | `eth_call` → `previewRedeem(uint256)` | Selector `0x4cdad506`; simulates underlying received for `shares` redeemed |
| `previewRate` | `eth_call` → `previewRate(bytes,bytes[])` on Router | Selector `0x69dfa6c2`; simulates a sequence of commands |
| `list-all-PTs` | `eth_call` → `pTCount()` + `getPTAt(uint256)` on Registry | Enumerate registered PTs on-chain. Selector `0x704bdadc` + `0x6c40a4f0` |
| `isRegisteredPT` | `eth_call` → `isRegisteredPT(address)` on Registry | Selector `0xf5e306f7`; verify a PT is legitimate |

#### Function Selector Reference (Read Operations)

| Function | Selector | Contract |
|----------|----------|---------|
| `maturity()` | `0x204f83f9` | PT |
| `getIBT()` | `0xc644fe94` | PT |
| `getYT()` | `0x04aa50ad` | PT |
| `underlying()` | `0x6f307dc3` | PT |
| `previewDeposit(uint256)` | `0xef8b30f7` | PT |
| `previewRedeem(uint256)` | `0x4cdad506` | PT |
| `getCurrentYieldOfUserInIBT(address)` | `0x0e1b6d89` | PT |
| `previewRate(bytes,bytes[])` | `0x69dfa6c2` | Router |
| `previewSpotRate(bytes,bytes[])` | `0xb748f092` | Router |
| `getRouter()` | `0xb0f479a1` | Registry |
| `getFactory()` | `0x88cc58e4` | Registry |
| `pTCount()` | `0x704bdadc` | Registry |
| `getPTAt(uint256)` | `0x6c40a4f0` | Registry |
| `isRegisteredPT(address)` | `0xf5e306f7` | Registry |

---

## §3 User Scenarios

### Scenario 1: Lock Fixed Yield — Deposit underlying to get PT

**User says**: "I want to lock in fixed yield on my WETH on Base. Deposit 0.01 WETH into the Spectra weETH pool and get PT."

**Agent action sequence**:

1. `[off-chain]` Fetch pools from `app.spectra.finance/_next/data/.../pools.json`, filter for Base (chainId=8453) + weETH pool.
   - Result: PT = `0x07f58450a39d07f9583c188a2a4a441fac358100`, IBT = `0x22f757c0b434d93c93d9653f26c9441d8d06c8ec` (sw-weETH), underlying = WETH (`0x4200000000000000000000000000000000000006`), maturity = July 15 2026.
2. `[off-chain]` Call `onchainos wallet addresses` to get user EVM address.
3. `[off-chain]` Call `onchainos wallet balance --chain 8453` to verify WETH balance >= 0.01.
4. `[off-chain]` `eth_call` → `previewDeposit(10000000000000000)` on PT to estimate PT shares to receive.
5. Show user: depositing 0.01 WETH → ~X PT (fixed rate), maturity July 15 2026, current implied APY.
6. **Ask user to confirm.**
7. `[on-chain]` Approve WETH for PT contract (if allowance < 0.01 WETH):
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x4200000000000000000000000000000000000006 \
     --input-data 0x095ea7b3\
   00000000000000000000000007f58450a39d07f9583c188a2a4a441fac358100\
   ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
     --from <WALLET>
   ```
8. `[on-chain]` Call `deposit(uint256,address,address,uint256)` on PT:
   - `assets` = 10000000000000000 (0.01 WETH in wei)
   - `ptReceiver` = user wallet
   - `ytReceiver` = user wallet
   - `minShares` = estimated shares * 0.995 (0.5% slippage)
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0x07f58450a39d07f9583c188a2a4a441fac358100 \
     --input-data 0xe4cca4b0\
   <assets_padded_32bytes>\
   000000000000000000000000<ptReceiver_no0x>\
   000000000000000000000000<ytReceiver_no0x>\
   <minShares_padded_32bytes> \
     --from <WALLET> \
     --force
   ```
9. Confirm PT and YT received; show maturity date and implied fixed APY.

---

### Scenario 2: Claim Accrued Yield from YT Holdings

**User says**: "I hold YT-weETH on Base. Claim all my accrued yield."

**Agent action sequence**:

1. `[off-chain]` `onchainos wallet addresses` → get user wallet.
2. `[off-chain]` `onchainos wallet balance --chain 8453` → find YT token balance (look for YT-prefixed token symbols).
3. `[off-chain]` For each YT held, find the corresponding PT address (YT stores it via `PT.getYT()` reverse lookup, or via pools.json data).
4. `[off-chain]` `eth_call` → `getCurrentYieldOfUserInIBT(<user_address>)` on the PT contract (selector `0x0e1b6d89`) → returns pending yield in IBT units.
5. `[off-chain]` Fetch IBT rate via `getIBTRate()` to convert IBT yield to underlying USD value.
6. Show user: pending yield = X IBT tokens = $Y USD for each pool.
7. **Ask user to confirm** (skip if yield is negligible < $0.01).
8. `[on-chain]` Call `claimYield(address)` on each PT contract:
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to <PT_ADDRESS> \
     --input-data 0x999927df\
   000000000000000000000000<receiver_address_no0x> \
     --from <WALLET> \
     --force
   ```
9. Show received yield amount and transaction hash.

---

### Scenario 3: Redeem PT After Maturity

**User says**: "My Spectra weETH PT matured. Redeem all my PT for WETH."

**Agent action sequence**:

1. `[off-chain]` `onchainos wallet addresses` → user wallet.
2. `[off-chain]` `onchainos wallet balance --chain 8453` → find PT token balances.
3. `[off-chain]` For each PT held: `eth_call` → `maturity()` (selector `0x204f83f9`) → check if Unix timestamp < now (current date: April 2026).
4. If maturity has NOT passed: warn user "PT has not matured yet. You can redeem early by also providing equal YT (use `withdraw`), or wait for maturity."
5. If maturity HAS passed:
   - `[off-chain]` `eth_call` → `previewRedeem(<ptBalance>)` to estimate underlying received.
   - Show user: redeeming X PT → ~Y WETH (1:1 minus fees).
6. **Ask user to confirm.**
7. `[on-chain]` Call `redeem(uint256,address,address,uint256)` on PT:
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to <PT_ADDRESS> \
     --input-data 0x9f40a7b3\
   <shares_padded_32bytes>\
   000000000000000000000000<receiver_no0x>\
   000000000000000000000000<owner_no0x>\
   <minAssets_padded_32bytes> \
     --from <WALLET> \
     --force
   ```
8. Confirm WETH received; show transaction hash.

---

### Scenario 4: Swap PT for IBT (Exit Fixed Rate Position Early via Router)

**User says**: "Sell my PT-weETH for WETH now, before maturity."

**Agent action sequence**:

1. `[off-chain]` Find PT balance and corresponding Curve pool address from pools.json.
   - PT: `0x07f58450a39d07f9583c188a2a4a441fac358100`, Curve pool: `0x3870a9498cd7ced8d134f19b0092931ef83aec1e`
2. `[off-chain]` `eth_call` → `previewSpotRate(bytes,bytes[])` on Router to estimate swap output.
3. Show: selling X PT → ~Y WETH (with price impact warning if > 1%).
4. **Ask user to confirm.**
5. `[on-chain]` Approve PT for Router:
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to <PT_ADDRESS> \
     --input-data 0x095ea7b3\
   000000000000000000000000c03309de321a4d3df734f5609b80cc731ae28e6d\
   ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff \
     --from <WALLET>
   ```
6. `[on-chain]` Build Router `execute` calldata:
   - Command 1: `TRANSFER_FROM` — transfer PT from user to Router
   - Command 2: `CURVE_SWAP` — swap PT → IBT in Curve pool (i=1/PT side, j=0/IBT side)
   - (Optional) Command 3: `REDEEM_IBT_FOR_ASSET` — unwrap IBT → underlying WETH
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to 0xc03309de321a4d3df734f5609b80cc731ae28e6d \
     --input-data <encoded_execute_calldata> \
     --from <WALLET> \
     --force
   ```
7. Confirm WETH received; show price impact and transaction hash.

---

### Scenario 5: Check All Positions and Pending Yield

**User says**: "Show me all my Spectra positions on Base — PT balances, YT balances, and pending yield."

**Agent action sequence**:

1. `[off-chain]` `onchainos wallet addresses` → user wallet.
2. `[off-chain]` Fetch pools.json to get all Base pool addresses.
3. `[off-chain]` `onchainos wallet balance --chain 8453` → identify ERC-20 balances matching known PT/YT addresses.
4. For each held PT/YT:
   - `eth_call` → `maturity()` → days until maturity.
   - `eth_call` → `getCurrentYieldOfUserInIBT(user)` → pending yield (in IBT).
   - `eth_call` → `previewRedeem(ptBalance)` → current redemption value.
5. Display:
   ```
   Pool: weETH (Ether.fi)  Chain: Base  Maturity: Jul 15 2026 (101 days)
   PT Balance: 0.0095 PT-sw-weETH  Redemption Value: ~0.0095 WETH
   YT Balance: 0.0095 YT-sw-weETH  Pending Yield: 0.0001 WETH ($0.21)
   ```
6. Show total portfolio value and unclaimed yield sum.

---

## §4 External API Dependencies

| Dependency | URL / RPC | Purpose | Auth | Notes |
|-----------|-----------|---------|------|-------|
| Spectra App Data | `https://app.spectra.finance/_next/data/{buildId}/pools.json` | Pool list: PT/YT/IBT addresses, Curve pool address, maturity, APY, TVL | None | Build ID changes on deploy. Cache response for session. Filter by `chainId`. See fragility note in §6. |
| Base RPC | `https://mainnet.base.org` | `eth_call` for read operations | None | Primary. Use `https://base-rpc.publicnode.com` if rate-limited (KNOWLEDGE_HUB: `-32016 over rate limit` on `mainnet.base.org`) |
| Arbitrum RPC | `https://arb1.arbitrum.io/rpc` | `eth_call` for Arbitrum reads | None | Fallback to `https://arbitrum-rpc.publicnode.com` |

> **No API key required.** Spectra has no public API. All on-chain reads use public RPCs; pool data scraped from the app's Next.js data endpoint.

---

## §5 Config Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `default_chain_id` | `u64` | `8453` | Default chain (Base — lower gas, active pools) |
| `default_slippage` | `f64` | `0.005` | Default slippage tolerance (0.5%) for deposit/redeem |
| `swap_slippage` | `f64` | `0.01` | Slippage for AMM swaps (1% — Curve pools have rate-adjustment, less slippage) |
| `max_price_impact_warn` | `f64` | `0.03` | Warn user if PT swap price impact > 3% |
| `max_price_impact_block` | `f64` | `0.10` | Block PT swap if price impact > 10% |
| `dry_run` | `bool` | `false` | Dry-run mode — simulate without broadcasting. Handle in Rust wrapper, never pass `--dry-run` to onchainos CLI (unsupported flag). |

### Known Deployed Contracts

#### Base (chain 8453) — PRIMARY TEST CHAIN

| Contract | Address | Verified |
|---------|---------|---------|
| Registry | `0x786da12e9836a9ff9b7d92e8bac1c849e2ace378` | Live `eth_call` (`pTCount()` → 69 PTs) ✅ |
| Router (proxy) | `0xc03309de321a4d3df734f5609b80cc731ae28e6d` | Live `eth_call` (`execute()` returns `0x`) ✅ |
| Factory (proxy) | `0xdbe5b6aac70eea77c5b59b6c54d8f21dffaa8d84` | Via Registry `getFactory()` ✅ |
| PT Beacon | `0x3da466f5be8024405a366538ed7949b4ce9f015d` | Embedded immutable in PT contracts ✅ |

#### Active Base Pools (as of April 2026) — Top by TVL

| Pool Name | PT Address | YT Address | IBT Address | Underlying | Curve Pool | Maturity | TVL |
|-----------|-----------|-----------|------------|-----------|-----------|---------|-----|
| weETH (Ether.fi) | `0x07f58450a39d07f9583c188a2a4a441fac358100` | `0xd29fb7faFdBee7164C781A56A623b38E040030bB` | `0x22f757c0b434d93c93d9653f26c9441d8d06c8ec` (sw-weETH) | WETH `0x4200...0006` | `0x3870a9498cd7ced8d134f19b0092931ef83aec1e` | Jul 15 2026 | ~$77K |
| sjEUR (Jarvis) | `0x3928cbccc982efbadbc004977827325b0be4c346` | `0x97b6D8d8534455d9A9A36ca7a95CC862c9c05E0B` | `0x89cc2a57223fa803852b6b4e65c6e376d49909f9` (sjEUR) | jEURx `0xfcde...e36` | `0xa86bee5400d9f58aa2ff168fed6ab4bcb36bcc91` | Jul 16 2026 | ~$341K |
| wsuperOETHb (Origin) | `0x1dc1b09d656c07404aa2747a9930c0b4d297b4f3` | `0x3CdC2D0AE59bE92A4fd7bc92B66C215609857B2b` | `0x7fcd174e80f264448ebee8c88a7c4476aaf58ea6` (wsuperOETHb) | superOETHb `0xdbfe...0da3` | `0xd296a4ec9cde7f864c87f1d37a9529fb02ceb129` | Jun 1 2026 | ~$4K |
| pfUSDC-73 (Pendle Finance) | `0xb695e4e5eb27b0b67cf54cf18df5f160eac4573d` | `0x22b34bbEb1FbC9650909D2F796AD7ACE74AF23a2` | `0xf0de996292a195dbb5fc94ff1899781c874a9750` (pfUSDC-73) | USDC `0x8335...913` | `0x1bbb15588d85a2f4a92404195ead75454cde8353` | Apr 20 2026 | ~$31 |

> **Recommended test pool**: `weETH / WETH` on Base — live WETH underlying (easy to hold), active Curve pool, July 2026 maturity (not expired), ~$77K TVL.

> **Note on Arbitrum**: Spectra has minimal Arbitrum activity (only 1 registered pool in the pools.json as of April 2026). Base is the primary deployment. Contract addresses differ per chain — always resolve from Registry on the target chain.

---

## §6 Known Risks / Gotchas

### Critical: No Hosted SDK (vs Pendle)

Unlike Pendle (which has `POST /v3/sdk/{chainId}/convert` to generate calldata), Spectra requires **manual ABI encoding** of all contract calls. This means:

- `deposit` / `redeem` / `claimYield` — direct PT function calls with ABI-encoded inputs
- `swap-pt` — requires building the Router `execute` dispatcher calldata with correct command bytes and ABI-encoded inputs array. The command enum values must be read from the Router source code on GitHub.

### 1. Router `execute` Command Enum Values

The `commands` byte sequence uses integer enum values defined in `Dispatcher.sol`. These must be fetched from the GitHub source at build time. Do NOT hardcode guessed values. Implementation step: `gh api repos/perspectivefi/spectra-core/contents/src/router/Dispatcher.sol` to get the actual enum integers.

### 2. Pre-Expiry vs Post-Expiry Redeem Logic

Before maturity: requires BOTH equal amounts of PT and YT → use `withdraw(uint256 assets, address receiver, address owner)`.
After maturity: PT alone is sufficient → use `redeem(uint256 shares, address receiver, address owner, uint256 minAssets)`.

Always call `maturity()` on the PT contract first and compare against `block.timestamp` (current time) to branch correctly.

### 3. IBT Wrapping Layer

Spectra pools hold **IBT** (ERC-4626 Interest Bearing Tokens) not the raw underlying asset. The `sw-weETH` IBT is a Spectra-wrapped version of `weETH`. When depositing:
- Call `deposit(uint256 assets, ...)` with the **underlying** (WETH) → PT contract auto-wraps to IBT internally
- OR call `depositIBT(uint256 ibts, ...)` if the user already holds the IBT form

On Base, the IBT addresses often have a `sw-` prefix (Spectra Wrapper), indicating a Spectra-created ERC-4626 wrapper around the base yield token. Resolve IBT address via `getIBT()` on the PT contract (selector `0xc644fe94`).

### 4. Pools Data API Fragility

The `app.spectra.finance/_next/data/{buildId}/pools.json` endpoint embeds a build ID in the URL that changes on each deployment. Implement resilient fetching: if the build ID in the stored URL returns 404, parse the build ID from the app's HTML `<head>` (`/_next/static/{buildId}/_buildManifest.js` reference) and reconstruct the URL. Alternatively, use an in-process cache with a TTL of ~1 hour.

### 5. `--force` Flag Required

All on-chain write operations via `onchainos wallet contract-call` must include `--force` flag. Without it, the CLI may prompt for confirmation. Per KNOWLEDGE_HUB: "wallet contract-call needs `--force` for DEX ops — Add `--force` flag to all DEX wallet_contract_call invocations."

### 6. Approve Before Write Operations

PT deposit requires ERC-20 approve for the IBT/underlying token. Router swaps require ERC-20 approve for PT or IBT being sold. Always check allowance first; skip approve if sufficient. Build approve calldata: `0x095ea7b3` + `<spender_32bytes>` + `ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` (max uint256).

### 7. Spectra vs Pendle — Key Architectural Differences

| Aspect | Spectra | Pendle |
|--------|---------|--------|
| Integration | Direct contract calls | REST API generates calldata |
| AMM | Curve-based Rate Adjusted StableSwap (IBT/PT pair) | Custom AMM (SY/PT pair) |
| IBT Concept | ERC-4626 Interest Bearing Token (Spectra often wraps with `sw-` prefix) | SY (Standardized Yield) wrapper |
| Router Pattern | Command dispatcher: `execute(bytes commands, bytes[] inputs)` | Single router with per-function selectors |
| Yield Claiming | `PT.claimYield(receiver)` directly on PT contract | Via Pendle YTv2 `redeemDueInterestAndRewards` |
| Pool Discovery | No public API — Next.js data endpoint or on-chain Registry enumeration | REST API at `api-v2.pendle.finance` |
| Slippage | Set in individual Router command inputs (`minAmountOut` per CURVE_SWAP) | Set globally in SDK request body |
| Multi-chain | Factory+Registry per chain (different addresses per chain) | Same Router address across all chains |

### 8. Selector Verification Method

**IMPORTANT**: Python's `hashlib.sha3_256` is NOT Ethereum Keccak-256 (it's NIST SHA-3). All selectors in this document were computed using `Crypto.Hash.keccak` (pyca/cryptography keccak implementation) and verified against 4byte.directory. When implementing in Rust, use `alloy-sol-types sol!{}` macro or `keccak256()` from `alloy-primitives`.

### 9. Claimable Yield Precision

`getCurrentYieldOfUserInIBT(address)` returns yield in **IBT units** (not underlying). Convert to underlying using `previewRedeem(1 IBT unit)` or by multiplying by `getIBTRate()` (returns rate in Ray = 1e27). Display in underlying for user clarity.

### 10. Tokenization Fee

Spectra takes a tokenization fee on deposit (set per Registry, typically ~0.1%). This is already reflected in `previewDeposit` output. Users receive slightly fewer PT+YT than mathematically ideal. Always use `previewDeposit` to quote amounts, never derive manually.

---

## §7 References

- Spectra Developer Docs: https://dev.spectra.finance
- Protocol Overview: https://docs.spectra.finance
- GitHub (core contracts): https://github.com/perspectivefi/spectra-core
- Router contract: https://dev.spectra.finance/technical-reference/contract-functions/router
- Principal Token: https://dev.spectra.finance/technical-reference/contract-functions/principal-token
- Factory: https://dev.spectra.finance/technical-reference/contract-functions/factory
- Registry: https://dev.spectra.finance/technical-reference/contract-functions/registry
- AMM: https://dev.spectra.finance/technical-reference/spectras-automated-market-makers/rate-adjusted-stableswap-pools
- Routing Guide: https://dev.spectra.finance/guides/routing
- Spectra App (for pool data): https://app.spectra.finance/pools
- Base Registry on-chain: `0x786da12e9836a9ff9b7d92e8bac1c849e2ace378` (verified via `eth_call`)
- 4byte.directory selector verification: https://www.4byte.directory
