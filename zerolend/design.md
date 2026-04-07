# ZeroLend — Plugin Design

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | zerolend |
| dapp_name | ZeroLend |
| dapp_url | https://app.zerolend.xyz |
| dapp_docs | https://docs.zerolend.xyz |
| version | 0.1.0 |
| target_chains | zksync (324), linea (59144), blast (81457) |
| category | defi-protocol |
| tags | lending, borrowing, defi, earn, zerolend, aave-fork, collateral, health-factor, l2 |
| architecture | Skill + Binary (Rust) |
| integration_path | direct contract calls (Aave V3 fork — reuse aave-v3 patterns ~90%) |

---

## §1 Feasibility

### Protocol Identity

ZeroLend is explicitly confirmed as an Aave V3 fork:

> "ZeroLend is a dynamic lending protocol that closely resembles Aave V3. It was developed as a fork from the original Aave protocol."
> — ZeroLend documentation (docs.zerolend.xyz/llms-full.txt)

The ZeroLend audit (Mundus) confirms: "The forked repositories do not contain any changes to the Aave codebase that would compromise the protocol's security" and "The ZeroLend codebase contains no changes that undermine the security of logic provided by Aave."

**Implication:** The ABI is identical to Aave V3. All function selectors, calldata encoding, and RPC decoding logic in the existing `aave-v3` plugin can be reused verbatim. The only differences are:
1. Chain IDs (zkSync 324, Linea 59144, Blast 81457 vs. Ethereum 1, Polygon 137, Arbitrum 42161, Base 8453)
2. `PoolAddressesProvider` addresses per chain (listed in §5)
3. RPC endpoints for the new L2s
4. One Blast-specific contract addition (BlastAToken — relevant to aToken metadata but does NOT change Pool ABI)

### Feasibility Checklist

| Check | Result |
|-------|--------|
| Is it an Aave V3 fork? | YES — confirmed by docs and audit |
| ABI deviations from Aave V3? | None in Pool interface. Blast adds BlastAToken implementation (aToken-level, not Pool-level). No custom Pool functions. |
| Selectors identical to aave-v3? | YES — same Solidity source, identical keccak256 selectors |
| PoolAddressesProvider → getPool() pattern? | YES — standard Aave V3 registry pattern (selector 0x026b1d5f verified in aave-v3 plugin) |
| onchainos defi positions supported on these chains? | UNKNOWN — zkSync/Linea/Blast are newer L2s; fall back to on-chain RPC (same as positions fallback in aave-v3) |
| onchainos wallet contract-call on zkSync (324)? | Must verify — zkSync uses native account abstraction. Chain 324 may need validation. |
| RPC availability? | zkSync: `mainnet.era.zksync.io` (official). Linea: `rpc.linea.build` (official). Blast: `rpc.blast.io` (official). All support standard `eth_call`. |

### Integration Path

Identical to `aave-v3`: **Rust binary + onchainos CLI**

- **Read-only queries** (`get-reserves`, `get-position`): Rust binary calls `Pool.getReservesList()` + `Pool.getReserveData()` + `Pool.getUserAccountData()` via `eth_call` through on-chain RPC.
- **Write operations** (`supply`, `borrow`, `repay`, `withdraw`, `set-collateral`): Rust binary ABI-encodes calldata using `alloy-sol-types sol!` macro, submits via `onchainos wallet contract-call --chain <id> --to <Pool> --input-data <hex>`.
- **Pool address resolution**: Always call `PoolAddressesProvider.getPool()` at runtime (selector `0x026b1d5f` — same as aave-v3, verified empirically).

---

## §2 Interface Mapping

**This section documents ONLY the differences from the aave-v3 plugin. All other interface details are identical — see `/tmp/onchainos-plugins/aave-v3/design.md §2` for full ABI reference.**

### Critical Difference: PoolAddressesProvider Addresses Per Chain

| Chain | Chain ID | PoolAddressesProvider Address | Source |
|-------|----------|-------------------------------|--------|
| zkSync Era | 324 | `0x4f285Ea117eF0067B59853D6d16a5dE8088bA259` | docs.zerolend.xyz/security/deployed-addresses |
| Linea | 59144 | `0xC44827C51d00381ed4C52646aeAB45b455d200eB` | docs.zerolend.xyz/security/deployed-addresses |
| Blast | 81457 | `0xb0811a1FC9Fb9972ee683Ba04c32Cb828Bcf587B` | docs.zerolend.xyz/security/deployed-addresses |

> Note: Manta Pacific and X Layer are also supported by ZeroLend (Manta shares the same PoolAddressesProvider address as Linea: `0xC44827C51d00381ed4C52646aeAB45b455d200eB`; X Layer uses `0x2f7e54ff5d45f77bFfa11f2aee67bD7621Eb8a93`). These are out of scope for v0.1.0 — add in v0.2.0 once core chains are tested.

### RPC Endpoints (New L2s)

| Chain | Chain ID | Recommended RPC | Notes |
|-------|----------|-----------------|-------|
| zkSync Era | 324 | `https://mainnet.era.zksync.io` | Official zkSync RPC |
| Linea | 59144 | `https://rpc.linea.build` | Official Consensys RPC |
| Blast | 81457 | `https://rpc.blast.io` | Official Blast RPC |

Fallback RPCs (for rate limit resilience):
- zkSync: `https://zksync.drpc.org`
- Linea: `https://linea.drpc.org`
- Blast: `https://blast.drpc.org`

### Blast-Specific Contract Note

ZeroLend on Blast deploys `BlastAToken` (an aToken variant that captures native Blast yield) instead of the standard `AToken`. This is an aToken implementation detail only — the `Pool` contract interface and all ABI function signatures are unchanged. The `borrow`, `repay`, `supply`, `withdraw`, `setUserUseReserveAsCollateral`, and `getUserAccountData` selectors are identical. No code changes required in the binary.

### onchainos Chain Name Mapping (IMPORTANT)

The aave-v3 plugin's `chain_id_to_name()` function maps IDs to onchainos chain name strings. This mapping must be extended for ZeroLend chains. Current mapping in aave-v3:

```rust
1 => "ethereum", 137 => "polygon", 42161 => "arbitrum", 8453 => "base"
```

Required addition in zerolend's `onchainos.rs`:

```rust
324   => "zksync",
59144 => "linea",
81457 => "blast",
```

Verify these chain name strings against `onchainos wallet contract-call --help` output before finalizing. If onchainos does not support a chain name for these L2s, pass the numeric chain ID directly (the `wallet_contract_call` function already passes `chain_id.to_string()` for the `--chain` flag — keep that pattern).

### On-chain Write Operations — Selectors (Verified Identical to Aave V3)

All selectors are generated by `alloy-sol-types sol!` macro from the same Solidity function signatures — identical to Aave V3:

| Function | Selector | Generated by |
|----------|----------|-------------|
| `supply(address,uint256,address,uint16)` | `0x617ba037` | `alloy-sol-types sol!` |
| `withdraw(address,uint256,address)` | `0x69328dec` | `alloy-sol-types sol!` |
| `borrow(address,uint256,uint256,uint16,address)` | `0xa415bcad` | `alloy-sol-types sol!` |
| `repay(address,uint256,uint256,address)` | `0x573ade81` | `alloy-sol-types sol!` |
| `setUserUseReserveAsCollateral(address,bool)` | `0x5a3b74b9` | `alloy-sol-types sol!` |
| `setUserEMode(uint8)` | `0x28530a47` | `alloy-sol-types sol!` |
| `getUserAccountData(address)` | `0xbf92857c` | `alloy-sol-types sol!` |
| `getPool()` (PoolAddressesProvider) | `0x026b1d5f` | verified empirically in aave-v3 |
| `getReservesList()` | `0xd1946dbc` | verified empirically in aave-v3 |
| `getReserveData(address)` | `0x35ea6a75` | verified empirically in aave-v3 |

---

## §3 User Scenarios

### Scenario 1 — Supply USDC to earn yield (Linea, default chain)
- User: *"Supply 500 USDC to ZeroLend on Linea"*
- Steps:
  1. [read] Resolve Pool: `PoolAddressesProvider(0xC44827...).getPool()` on Linea RPC
  2. [read] Check current USDC supply APY via `Pool.getReserveData(USDC_addr)` slot 2
  3. [write] Encode `approve(Pool, 500e6)` calldata; submit via `onchainos wallet contract-call --chain 59144 --to <USDC> --input-data <hex>`
  4. [write] Encode `supply(USDC, 500e6, user, 0)` calldata; submit via `onchainos wallet contract-call --chain 59144 --to <Pool> --input-data <hex>`
  5. Return: tx hash, supply APY, new aUSDC balance estimate

### Scenario 2 — Borrow WETH against USDC collateral (zkSync Era)
- User: *"Borrow 0.1 WETH on ZeroLend on zkSync"*
- Steps:
  1. [read] Resolve Pool from `0x4f285Ea...` on zkSync RPC
  2. [read] `Pool.getUserAccountData(user)` — check availableBorrowsBase, health factor
  3. [write] Encode `borrow(WETH, 0.1e18, 2, 0, user)` calldata; submit via `wallet contract-call --chain 324`
  4. Return: tx hash, updated health factor, variable borrow APY

### Scenario 3 — Check health factor (Blast)
- User: *"What's my health factor on ZeroLend Blast?"*
- Steps:
  1. [read] Resolve Pool from `0xb0811a1F...` on Blast RPC
  2. [read] `Pool.getUserAccountData(user)` → decode 6 return values
  3. Return: health factor (formatted as `hf / 1e18`), collateral USD, debt USD, liquidation threshold, available borrows; flag if HF < 1.1

### Scenario 4 — Repay full debt (Linea)
- User: *"Repay all my USDC debt on ZeroLend Linea"*
- Steps:
  1. [read] `Pool.getUserAccountData(user)` → fetch currentVariableDebt for USDC
  2. [write] Encode `approve(Pool, wallet_balance)` for USDC; submit
  3. [write] Encode `repay(USDC, wallet_balance, 2, user)` using wallet balance (NOT uint256.max — avoids revert if accrued interest > balance per KNOWLEDGE_HUB repay-all-pitfall); submit via `wallet contract-call --chain 59144`
  4. Return: tx hash, confirmed debt balance

### Scenario 5 — List all markets (zkSync)
- User: *"Show me ZeroLend markets on zkSync"*
- Steps:
  1. [read] Resolve Pool on zkSync
  2. [read] `Pool.getReservesList()` → array of asset addresses
  3. [read] For each asset: `Pool.getReserveData(asset)` → slots 2+4 for supply/borrow APY
  4. Return: table of assets with supplyApy, variableBorrowApy

### Scenario 6 — Enable/disable collateral (Linea)
- User: *"Disable my WETH as collateral on ZeroLend Linea"*
- Steps:
  1. [read] `Pool.getUserAccountData(user)` — simulate HF impact; warn if HF would drop below 1.1
  2. [write] Encode `setUserUseReserveAsCollateral(WETH, false)` calldata; submit
  3. Return: tx hash, updated health factor

---

## §4 External API Dependencies

All data flows use on-chain `eth_call` via standard JSON-RPC — **zero external API keys required**. This is identical to the aave-v3 plugin's "Option B" decision.

| Dependency | Type | Purpose | Auth |
|------------|------|---------|------|
| zkSync Era RPC (`mainnet.era.zksync.io`) | On-chain RPC | `eth_call` for all read ops and contract address resolution | None — standard JSON-RPC |
| Linea RPC (`rpc.linea.build`) | On-chain RPC | `eth_call` for all read ops and contract address resolution | None — standard JSON-RPC |
| Blast RPC (`rpc.blast.io`) | On-chain RPC | `eth_call` for all read ops and contract address resolution | None — standard JSON-RPC |
| onchainos CLI | Local CLI | `wallet contract-call` for all write ops; `wallet status` for address resolution | onchainos login session |

No subgraph / The Graph dependency. All reserve data and user position data fetched via `Pool.getReserveData()` and `Pool.getUserAccountData()` on-chain, as tested in the aave-v3 plugin.

---

## §5 Config Parameters

The only structural difference from the aave-v3 plugin `config.rs` is the `CHAINS` static array. Replace it entirely with:

```rust
// src/config.rs — zerolend plugin
// Only change from aave-v3: chain IDs, PoolAddressesProvider addresses, RPC URLs, names.
// All other constants (INTEREST_RATE_MODE_VARIABLE, REFERRAL_CODE, HF thresholds) are identical.

pub static CHAINS: &[ChainConfig] = &[
    ChainConfig {
        chain_id: 324,
        pool_addresses_provider: "0x4f285Ea117eF0067B59853D6d16a5dE8088bA259",
        rpc_url: "https://mainnet.era.zksync.io",
        name: "zkSync Era",
    },
    ChainConfig {
        chain_id: 59144,
        pool_addresses_provider: "0xC44827C51d00381ed4C52646aeAB45b455d200eB",
        rpc_url: "https://rpc.linea.build",
        name: "Linea",
    },
    ChainConfig {
        chain_id: 81457,
        pool_addresses_provider: "0xb0811a1FC9Fb9972ee683Ba04c32Cb828Bcf587B",
        rpc_url: "https://rpc.blast.io",
        name: "Blast",
    },
];
```

Other config constants (copy verbatim from aave-v3):

| Constant | Value | Notes |
|----------|-------|-------|
| `INTEREST_RATE_MODE_VARIABLE` | `2` | Identical — variable rate only |
| `INTEREST_RATE_MODE_STABLE` | `1` | Identical — deprecated, blocked |
| `REFERRAL_CODE` | `0` | Identical — no referral |
| `HF_WARN_THRESHOLD` | `1.1` | Identical |
| `HF_DANGER_THRESHOLD` | `1.05` | Identical |
| default chain | `59144` (Linea) | Changed from aave-v3's `8453` (Base). Linea is ZeroLend's primary chain (ZERO token is native to Linea). |

---

## §6 Developer Porting Guide — Exact Changes to Make

The zerolend plugin is a copy of aave-v3 with surgical edits. Here is the complete change list:

### Files to copy verbatim (zero changes needed)

| File | Notes |
|------|-------|
| `src/calldata.rs` | All `sol!` ABI encoding — identical function signatures |
| `src/rpc.rs` | All `eth_call` helpers, `get_pool()`, `get_user_account_data()`, selectors — identical |
| `src/commands/borrow.rs` | Logic identical; config resolved via `get_chain_config()` |
| `src/commands/repay.rs` | Logic identical |
| `src/commands/supply.rs` | Logic identical |
| `src/commands/withdraw.rs` | Logic identical |
| `src/commands/health_factor.rs` | Logic identical |
| `src/commands/reserves.rs` | Logic identical |
| `src/commands/set_collateral.rs` | Logic identical |
| `src/commands/set_emode.rs` | Logic identical |
| `src/commands/positions.rs` | Logic identical |
| `src/commands/claim_rewards.rs` | Logic identical |
| `src/commands/mod.rs` | Logic identical |

### Files requiring edits

**`src/config.rs`** — Replace `CHAINS` static array with the 3-chain ZeroLend array shown in §5 above. Change default chain ID from `8453` to `59144` (Linea). All other constants unchanged.

**`src/main.rs`** — Change:
- `name = "aave-v3"` → `name = "zerolend"`
- `about = "Aave V3 lending..."` → `about = "ZeroLend lending and borrowing via OnchaionOS"`
- `default_value = "8453"` (for `--chain` arg) → `default_value = "59144"`
- Version string (if present) stays `"0.1.0"`

**`src/onchainos.rs`** — In `chain_id_to_name()`, add:
```rust
324   => "zksync",
59144 => "linea",
81457 => "blast",
```
and change the catch-all default from `_ => "ethereum"` to `_ => "linea"` (so mismatched IDs default to ZeroLend's primary chain). Also remove the `defi_search`, `defi_positions`, `defi_collect` wrappers if onchainos `defi` does not support these chains (or keep them with the new chain name mapping — they will gracefully fail with an error if the platform is not registered).

**`Cargo.toml`** — Change:
- `name = "aave-v3"` → `name = "zerolend"`
- `description` field if present

**`SKILL.md`** — New file describing the plugin. Key fields: `plugin_name: zerolend`, supported chains, operation list.

---

## §7 Known Risks and Gotchas

### L2 RPC Reliability
- zkSync Era, Linea, and Blast are newer L2s. Their public RPC endpoints may be less stable than mainnet RPCs. Always configure fallback RPCs in the error path (e.g., retry with `zksync.drpc.org` if `mainnet.era.zksync.io` fails).
- `rpc.linea.build` has rate limits under heavy `eth_call` load. Use `linea.drpc.org` as fallback (same pattern as KNOWLEDGE_HUB `base-rpc-rate-limit` gotcha).

### zkSync Era Native Account Abstraction
- zkSync Era (chain 324) uses native account abstraction, which affects how transactions are submitted. `onchainos wallet contract-call --chain 324` must be verified to work on zkSync before L4 testing. If onchainos does not support chain 324, all write operations on zkSync will be blocked (read-only via `eth_call` will still work since zkSync is EVM-compatible for calls).
- Action: In Tester phase, validate `onchainos wallet contract-call --chain 324 --dry-run` before any live tests.

### Token Availability on New L2s
- Not all ERC-20 tokens are bridged to zkSync/Linea/Blast. Test with assets that are confirmed present: WETH (wrapped ETH is always available), USDC (Circle native USDC on Linea and Blast), USDT.
- On Linea: USDC is `0x176211869cA2b568f2A7D4EE941E073a821EE1ff`, WETH is `0xe5D7C2a44FfDDf6b295A15c148167daaAf5Cf34e`.
- On zkSync Era: USDC is `0x3355df6D4c9C3035724Fd0e3914dE96A5a83aaf4`, WETH is `0x5AEa5775959fBC2557Cc8789bC1bf90A239D9a91`.
- On Blast: USDC is `0x4300000000000000000000000000000000000003` (Blast native USDC), WETH is `0x4300000000000000000000000000000000000004` (Blast native WETH).

### Blast Native Yield (BlastAToken)
- Blast aTokens are BlastAToken implementations that accrue native Blast yield (gas rebates + ETH yield). This does NOT affect the Pool ABI. However, users may see higher effective yields than the `currentLiquidityRate` alone suggests. Consider noting this in the `reserves` output.

### repay(uint256.max) Pitfall
- Per KNOWLEDGE_HUB `repay-all-pitfall`: using `uint256.max` for full repay can revert if wallet balance < accrued interest at time of execution. Use `wallet_balance` as the repay amount instead, same as aave-v3 implementation.

### E-Mode Category IDs Are Chain-Specific
- E-Mode category IDs (1 = stablecoins, 2 = ETH-correlated) are configured per ZeroLend deployment and may differ from Aave V3 mainnet. Fetch available categories from `Pool.getEModeCategoryData(categoryId)` or document as "check ZeroLend UI for active categories" rather than hardcoding.

### onchainos defi Positions May Not Support These Chains
- `onchainos defi positions --chains linea` may return empty or error if ZeroLend is not registered in the onchainos defi registry. The positions command should fall back to on-chain `Pool.getUserAccountData()` call directly (this fallback is already present in aave-v3's positions.rs — preserve it).

### PoolAddressesProvider Addresses
- Addresses sourced from `docs.zerolend.xyz/security/deployed-addresses` (fetched April 2026). Verify against ZeroLend GitHub or block explorer before shipping. Linea and Manta Pacific appear to share the same PoolAddressesProvider address (`0xC44827C51d00381ed4C52646aeAB45b455d200eB`) — this mirrors the Aave V3 Polygon/Arbitrum pattern and may be intentional (deterministic deployment) or may indicate a docs error. Confirm on-chain by calling `getPool()` on each.

---

## §8 Test Plan Guidance

### Recommended Test Chain: Linea (59144)
Linea is ZeroLend's primary chain (ZERO governance token is native to Linea). It has the deepest liquidity and most validated RPC infrastructure. Use Linea as the default test chain.

### Test Sequence (same as aave-v3 L1-L4 pattern)
1. **L1 — Build**: `cargo build --release` — should compile with zero changes to calldata.rs and rpc.rs
2. **L2 — Dry-run**: `zerolend reserves --chain 59144` (read-only, no wallet), `zerolend health-factor --chain 59144 --from <addr>`
3. **L3 — Simulation**: `zerolend supply --chain 59144 --asset <USDC_addr> --amount 10 --dry-run`
4. **L4 — Live**: `zerolend supply` → `zerolend borrow` → `zerolend repay` → `zerolend withdraw` on Linea with small amounts (0.01 USDC or 0.001 WETH)

### L4 Prerequisites
- Test wallet must have ETH on Linea for gas and a small USDC or WETH balance for supply tests.
- Verify `onchainos wallet contract-call --chain 59144` is supported before L4.

---

## §9 Reference Links

- ZeroLend App: https://app.zerolend.xyz
- ZeroLend Docs: https://docs.zerolend.xyz
- ZeroLend Deployed Addresses: https://docs.zerolend.xyz/security/deployed-addresses
- ZeroLend GitHub: https://github.com/zerolend (source contracts)
- zkSync Era Explorer: https://explorer.zksync.io
- Linea Explorer: https://lineascan.build
- Blast Explorer: https://blastscan.io
- ZeroLend PoolAddressesProvider (zkSync): https://explorer.zksync.io/address/0x4f285Ea117eF0067B59853D6d16a5dE8088bA259
- ZeroLend PoolAddressesProvider (Linea): https://lineascan.build/address/0xC44827C51d00381ed4C52646aeAB45b455d200eB
- ZeroLend PoolAddressesProvider (Blast): https://blastscan.io/address/0xb0811a1FC9Fb9972ee683Ba04c32Cb828Bcf587B
- aave-v3 plugin (reference implementation): /tmp/onchainos-plugins/aave-v3/
- Aave V3 Pool ABI reference: https://aave.com/docs/aave-v3/smart-contracts/pool
