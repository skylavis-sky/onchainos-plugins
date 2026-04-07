# Aura Finance — Plugin Design

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `aura-finance` |
| dapp_name | Aura Finance |
| version | 0.1.0 |
| target_chains | Ethereum (chain 1) primary; Arbitrum (chain 42161) secondary |
| category | defi-protocol |
| tags | aura, balancer, bpt, bal, yield, staking, vlAURA, governance |
| author | onchainos-pipeline |

---

## §1 Feasibility

### Comparison to Convex Finance (Reference Plugin)

Aura Finance is architecturally near-identical to Convex Finance — it is to Balancer what Convex is to Curve:

| Axis | Convex Finance | Aura Finance |
|------|---------------|--------------|
| Underlying protocol | Curve Finance | Balancer V2 |
| LP token deposited | Curve LP tokens (cLP) | Balancer Pool Tokens (BPT) |
| Main deposit contract | `Booster` (`0xF403C135...`) | `Booster` (`0xA57b8d98...`) |
| Reward pool per pool | `BaseRewardPool` (Convex fork) | `BaseRewardPool` (same code) |
| Lock token | vlCVX (16 weeks) | vlAURA (16 weeks) |
| Boosted reward token | CRV + CVX | BAL + AURA |
| Wrapped governance token | cvxCRV | auraBAL |
| REST API for pool data | Curve API | Aura/Balancer API (or on-chain) |

**Assessment: High feasibility.** The Convex plugin patterns apply directly.
The main difference is that users must already hold Balancer Pool Tokens (BPT),
not Curve LP tokens. BPT acquisition requires interacting with Balancer separately
(outside this plugin's scope).

### Feasibility Checklist

| Check | Result |
|-------|--------|
| Official Rust SDK? | None. Direct EVM ABI calls + REST API |
| REST API available? | Aura data API exists (`https://data.aura.finance/graphql`) but GraphQL; Balancer REST API (`https://api.balancer.fi`) usable for pool data |
| Official Skill? | None |
| Needs onchainos broadcast? | Yes — deposit, withdraw, claim-rewards, lock-aura are all on-chain write ops |
| Supported chains | Ethereum mainnet (chain 1) primary; Arbitrum One (chain 42161) secondary |
| BPT prerequisite | **Critical**: Users must already hold the specific Balancer LP token (BPT) for a pool before depositing into Aura. The plugin does NOT handle Balancer liquidity provision. |

**Integration path**: Balancer REST API (pool/APY data) + direct EVM `eth_call` (position reads) + `onchainos wallet contract-call` (write operations).

---

## §2 Interface Mapping

### Operations Overview

| Operation | Type | Description |
|-----------|------|-------------|
| `get-pools` | Off-chain read | List Aura-supported pools with APY, TVL, pool IDs |
| `get-position` | Off-chain read | User's staked BPT balance and pending BAL/AURA rewards |
| `deposit` | On-chain write | Approve BPT + deposit into Aura Booster |
| `withdraw` | On-chain write | Withdraw staked BPT from BaseRewardPool |
| `claim-rewards` | On-chain write | Claim BAL + AURA rewards from BaseRewardPool |
| `lock-aura` | On-chain write | Lock AURA tokens for vlAURA voting power (16-week lock) |

---

### On-chain Write Operations (EVM)

All write operations go through `onchainos wallet contract-call --chain 1 --to <address> --input-data <calldata>`.

> **Note on selectors**: All selectors verified via pycryptodome keccak256 (correct Ethereum keccak, not NIST SHA3). Cross-checked against Convex Finance plugin for identical function signatures. `approve`, `balanceOf`, `earned`, `getReward(address,bool)`, `getReward()`, `processExpiredLocks(bool)` selectors match Convex design.md exactly ✅.

| Operation | Contract Address | Function Signature | Selector (keccak256 ✅) | Param Order |
|-----------|-----------------|-------------------|------------------------|-------------|
| ERC-20 approve BPT | `<bptTokenAddress>` (varies per pool) | `approve(address,uint256)` | `0x095ea7b3` ✅ | spender=Booster, amount |
| `deposit` BPT into pool | `0xA57b8d98dAE62B26Ec3bcC4a365338157060B234` (Booster) | `deposit(uint256,uint256,bool)` | `0x43a0d066` ✅ | _pid, _amount, _stake(=true) |
| `depositAll` BPT into pool | `0xA57b8d98dAE62B26Ec3bcC4a365338157060B234` (Booster) | `depositAll(uint256,bool)` | `0x60759fce` ✅ | _pid, _stake(=true) |
| `withdraw` from BaseRewardPool | `<crvRewards>` (per-pool BaseRewardPool) | `withdrawAndUnwrap(uint256,bool)` | `0xc32e7202` ✅ | amount, claim(=false) |
| `withdrawAll` from BaseRewardPool | `<crvRewards>` (per-pool BaseRewardPool) | `withdrawAllAndUnwrap(bool)` | `0x49f039a2` ✅ | claim(=false) |
| `claim-rewards` from pool | `<crvRewards>` (per-pool BaseRewardPool) | `getReward(address,bool)` | `0x7050ccd9` ✅ | _account(=wallet), _claimExtras(=true) |
| ERC-20 approve AURA | `0xC0c293ce456fF0ED870ADd98a0828Dd4d2903DBF` (AURA token) | `approve(address,uint256)` | `0x095ea7b3` ✅ | spender=AuraLocker, amount |
| `lock-aura` (vlAURA) | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` (AuraLocker) | `lock(address,uint256)` | `0x282d3fdf` ✅ | _account(=wallet), _amount |
| unlock expired vlAURA | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` (AuraLocker) | `processExpiredLocks(bool)` | `0x312ff839` ✅ | _relock(=false) |
| claim vlAURA rewards | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` (AuraLocker) | `getReward(address,bool)` | `0x7050ccd9` ✅ | _account(=wallet), _stake(=false) |

---

### Off-chain Read Operations

#### `get-pools`

Fetches Aura-supported pool list with APY and TVL data.

- **Primary**: `GET https://api.balancer.fi/pools/1` — returns Balancer pools on Ethereum mainnet with `tvlUsd`, `apr`, `tokens`, `address`
- **On-chain fallback**: Call `Booster.poolLength()` then iterate `Booster.poolInfo(pid)` for each pool
- **Key fields returned**: `pid` (Aura pool ID), `lptoken` (BPT address), `crvRewards` (BaseRewardPool address), `gauge`, `shutdown` flag

| Call | Contract | Function Signature | Selector | Params |
|------|----------|--------------------|----------|--------|
| `poolLength` | `0xA57b8d98dAE62B26Ec3bcC4a365338157060B234` | `poolLength()` | `0x081e3eda` ✅ | — |
| `poolInfo` | `0xA57b8d98dAE62B26Ec3bcC4a365338157060B234` | `poolInfo(uint256)` | `0x1526fe27` ✅ | pid |

`poolInfo(uint256)` returns: `(address lptoken, address token, address gauge, address crvRewards, address stash, bool shutdown)`

#### `get-position`

Reads a user's staked balance and pending rewards for a given pool.

| Call | Contract | Function Signature | Selector | Params |
|------|----------|--------------------|----------|--------|
| staked BPT balance | `<crvRewards>` (per-pool BaseRewardPool) | `balanceOf(address)` | `0x70a08231` ✅ | user_wallet |
| pending BAL rewards | `<crvRewards>` (per-pool BaseRewardPool) | `earned(address)` | `0x008cc262` ✅ | user_wallet |
| vlAURA locked balance | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` | `balanceOf(address)` | `0x70a08231` ✅ | user_wallet |
| vlAURA lock details | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` | `lockedBalances(address)` | `0x0483a7f6` ✅ | user_wallet |
| vlAURA claimable rewards | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` | `claimableRewards(address)` | `0xdc01f60d` ✅ | user_wallet |
| BPT allowance check | `<bptTokenAddress>` | `allowance(address,address)` | `0xdd62ed3e` ✅ | owner, spender=Booster |
| AURA balance | `0xC0c293ce456fF0ED870ADd98a0828Dd4d2903DBF` | `balanceOf(address)` | `0x70a08231` ✅ | user_wallet |

---

### Key Contract Addresses (Ethereum Mainnet, Chain 1)

| Contract | Address | Notes |
|----------|---------|-------|
| Booster | `0xA57b8d98dAE62B26Ec3bcC4a365338157060B234` | Main deposit entry point; manages all pools |
| AURA token | `0xC0c293ce456fF0ED870ADd98a0828Dd4d2903DBF` | Governance/reward token |
| auraBAL token | `0x616e8BfA43F920657B3497DBf40D6b1A02D4608d` | Wrapped governance BAL (irreversible conversion) |
| AuraLocker (vlAURA) | `0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC` | Lock AURA for 16 weeks; vote on gauges |
| VoterProxy | `0xaF52695E1bB01A16D33D7194C28C42b10e0Dbec2` | Protocol-owned veBAL holder |
| PoolManager | `0x8Dd8cDb1f3d419CCDCbf4388bC05F4a7C8aEBD64` | Adds new Balancer pools to Booster |
| Reward Factory | `0xBC8d9cAf4B6bf34773976c5707ad1F2778332DcA` | Deploys BaseRewardPool per pool |
| auraBAL Rewards | `0x00A7BA8Ae7bca0B10A32Ea1f8e2a1Da980c6CAd2` | BaseRewardPool for staked auraBAL |
| BAL token | `0xba100000625a3754423978a60c9317c58a424e3D` | Balancer governance token (earned as reward) |
| RPC (Ethereum) | `https://ethereum.publicnode.com` | Preferred (no rate limit issues per KB) |

### Notable Pool Examples (Ethereum Mainnet)

> Pool IDs below correspond to Aura `pid` values (integer index in Booster.pools[]). The per-pool `BaseRewardPool` address must be read from `Booster.poolInfo(pid).crvRewards` at runtime — it changes per deployment.

| Aura PID | Pool Name | Balancer Pool ID | Notes |
|----------|-----------|-----------------|-------|
| 29 | wstETH/WETH 50/50 | `0x32296969ef14eb0c6d29669c550d4a0449130230...` | Largest TVL pool |
| 109 | rETH/WETH | `0x1e19cf2d73a72ef1332c882f20534b6519be0276...` | LSD pool |
| 100 | USDC/DAI/USDT (stable) | Various | Stablecoin pool |

> **Implementor note**: Do NOT hardcode BaseRewardPool addresses per pool — always call `Booster.poolInfo(pid)` to get `crvRewards` address dynamically. Pools can be migrated.

---

## §3 User Scenarios

### Scenario 1: List Top Aura Pools

**User**: "Show me the top Aura Finance pools and their yields"

**Action sequence**:
1. (Off-chain) `GET https://api.balancer.fi/pools/1` — fetch Balancer pool data with TVL and APY
2. (On-chain eth_call) `Booster.poolLength()` — get total pool count
3. (On-chain eth_call, batched) `Booster.poolInfo(0..N)` for first N active pools — get `lptoken`, `crvRewards`, `shutdown`
4. Cross-reference Balancer API data (by lptoken address) with Aura pool data
5. Filter out `shutdown=true` pools
6. Sort by `tvlUsd` descending, return top 10
7. Display: pool name, tokens, Aura PID, TVL, BAL APY, AURA APY, BaseRewardPool address

---

### Scenario 2: Check User Position

**User**: "What are my Aura Finance positions?"

**Action sequence**:
1. (Off-chain) Resolve wallet address: `onchainos wallet balance --chain 1` — parse `data.details[0].tokenAssets[0].address`
2. (On-chain eth_call) For each pool of interest: `BaseRewardPool.balanceOf(wallet)` → staked BPT amount
3. (On-chain eth_call) For each pool with balance: `BaseRewardPool.earned(wallet)` → pending BAL rewards
4. (On-chain eth_call) `AuraLocker.balanceOf(wallet)` → vlAURA locked amount
5. (On-chain eth_call) `AuraLocker.lockedBalances(wallet)` → lock expiry details
6. (On-chain eth_call) `AuraLocker.claimableRewards(wallet)` → pending vlAURA rewards
7. (On-chain eth_call) `AURA.balanceOf(wallet)` → liquid AURA
8. Format and display position summary

---

### Scenario 3: Deposit BPT into Aura Pool

**User**: "Deposit my wstETH/WETH BPT into Aura pool 29"

**Prerequisites check**: User must already hold wstETH/WETH BPT. If they don't, explain they need to add liquidity on Balancer first (outside this plugin's scope).

**Action sequence**:
1. (Off-chain) Resolve wallet address
2. (Off-chain) Confirm pool PID = 29; call `Booster.poolInfo(29)` to get `lptoken` (BPT address) and `crvRewards`
3. (On-chain eth_call) `BPT.balanceOf(wallet)` — verify sufficient balance
4. (On-chain eth_call) `BPT.allowance(wallet, Booster)` — check existing approval
5. If allowance < amount:
   - Ask user to confirm ERC-20 approval
   - (On-chain write) `BPT.approve(Booster, amount)`: `onchainos wallet contract-call --chain 1 --to <bptAddress> --input-data 0x095ea7b3<Booster_padded><amount_padded>`
   - Wait ~15s for approval confirmation (same pattern as Convex/Lido)
6. Ask user to confirm deposit
7. (On-chain write) `Booster.deposit(29, amount, true)`: `onchainos wallet contract-call --chain 1 --to 0xA57b8d98dAE62B26Ec3bcC4a365338157060B234 --input-data 0x43a0d066<pid_padded><amount_padded><01>`
   - `_stake=true` immediately stakes the receipt token into BaseRewardPool
8. Report txHash

---

### Scenario 4: Withdraw Staked BPT

**User**: "Withdraw 100 BPT from Aura pool 29"

**Action sequence**:
1. (Off-chain) Resolve wallet address
2. (Off-chain) Get `crvRewards` address from `Booster.poolInfo(29)`
3. (On-chain eth_call) `BaseRewardPool.balanceOf(wallet)` — verify staked balance >= amount
4. Ask user to confirm withdrawal (note: rewards can optionally be claimed simultaneously)
5. (On-chain write) `BaseRewardPool.withdrawAndUnwrap(amount, false)`:
   `onchainos wallet contract-call --chain 1 --to <crvRewards> --input-data 0xc32e7202<amount_padded><00>`
   - `claim=false` — do not claim rewards atomically (handle separately for clarity)
6. Report txHash

---

### Scenario 5: Claim BAL + AURA Rewards

**User**: "Claim my rewards from Aura pool 29"

**Action sequence**:
1. (Off-chain) Resolve wallet address
2. (Off-chain) Get `crvRewards` from `Booster.poolInfo(29)`
3. (On-chain eth_call) `BaseRewardPool.earned(wallet)` — check pending BAL rewards
4. If earned > 0:
   - Ask user to confirm
   - (On-chain write) `BaseRewardPool.getReward(wallet, true)`:
     `onchainos wallet contract-call --chain 1 --to <crvRewards> --input-data 0x7050ccd9<wallet_padded><01>`
     - `_claimExtras=true` claims both BAL and AURA from extra reward distributors
5. Report txHash and estimated claimed amounts

---

### Scenario 6: Lock AURA as vlAURA

**User**: "Lock 500 AURA tokens"

**Action sequence**:
1. (Off-chain) Resolve wallet address
2. (On-chain eth_call) `AURA.balanceOf(wallet)` — verify >= 500 AURA
3. (On-chain eth_call) `AURA.allowance(wallet, AuraLocker)` — check approval
4. If allowance < amount:
   - Ask user to confirm approval (warn: AURA will be locked for 16 weeks)
   - (On-chain write) `AURA.approve(AuraLocker, amount)`:
     `onchainos wallet contract-call --chain 1 --to 0xC0c293ce456fF0ED870ADd98a0828Dd4d2903DBF --input-data 0x095ea7b3<AuraLocker_padded><amount_padded>`
   - Wait ~15s
5. Ask user to confirm lock (emphasize 16-week lock period, irreversible until expiry)
6. (On-chain write) `AuraLocker.lock(wallet, amount)`:
   `onchainos wallet contract-call --chain 1 --to 0x3Fa73f1E5d8A792C80F426fc8F84FBF7Ce9bBCAC --input-data 0x282d3fdf<wallet_padded><amount_padded>`
7. Report txHash

---

## §4 External API Dependencies

| API | Endpoint | Purpose | Auth |
|-----|----------|---------|------|
| Balancer REST | `https://api.balancer.fi/pools/1` | Pool list with TVL, APY, tokens for Ethereum mainnet | None |
| Aura GraphQL | `https://data.aura.finance/graphql` | Aura-specific pool APY, auraBAL metrics | None (public) |
| Ethereum RPC | `https://ethereum.publicnode.com` | eth_call for balances, pool info | None |

**Fallback**: If the Balancer API is unavailable, the plugin can fall back to on-chain `poolInfo` calls, but APY data will not be available. Display a warning in that case.

---

## §5 Config Parameters

| Parameter | Default | Description |
|-----------|---------|-------------|
| `--chain` | `1` | Chain ID (primary: 1 = Ethereum mainnet) |
| `--pool-id` | (required for pool ops) | Aura pool PID (integer index in Booster) |
| `--amount` | (required for deposit/withdraw/lock) | Token amount in wei |
| `--from` | (onchainos active wallet) | Override wallet address |
| `--dry-run` | false | Simulate mode; skip on-chain broadcast. Handle dry-run check BEFORE `resolve_wallet()` call (see KB: `dry-run-wallet-ordering`) |
| `--limit` | `10` | Max pools to return for `get-pools` |
| `--claim-extras` | true | Whether to claim extra rewards in `claim-rewards` |

---

## §6 Known Risks / Gotchas

### BPT Acquisition Prerequisite (Critical)
Aura Finance requires users to already hold Balancer Pool Tokens (BPT) for the specific pool they want to deposit into. BPT is acquired by providing liquidity on Balancer — this is a completely separate operation outside this plugin. **The plugin MUST document this prerequisite clearly in SKILL.md** and provide a helpful error message if a user's BPT balance is 0.

### 16-Week Lock for vlAURA
Locking AURA as vlAURA via `AuraLocker.lock()` is irreversible for 16 weeks. Users cannot unlock early. The plugin must display a prominent warning before executing `lock-aura`. After 16 weeks, `processExpiredLocks(false)` can be called to release tokens.

### Two-Transaction Deposit Flow
Depositing BPT requires two on-chain transactions: (1) ERC-20 `approve` on the BPT token, then (2) `Booster.deposit`. Follow the Convex/Lido pattern: wait ~15 seconds between approval and deposit to allow the approval to confirm.

### BaseRewardPool Address Varies Per Pool
The `BaseRewardPool` (reward/withdrawal contract) address is different for every pool. Never hardcode it. Always query `Booster.poolInfo(pid).crvRewards` dynamically.

### `_stake=true` vs `_stake=false` in Deposit
`Booster.deposit(pid, amount, _stake=true)` automatically stakes the receipt token into the BaseRewardPool, enabling reward accrual immediately. If `_stake=false`, the user receives an unstaked receipt token and earns no rewards until they manually stake. Always use `_stake=true` for the standard deposit flow.

### `withdrawAndUnwrap` vs `withdraw`
Use `BaseRewardPool.withdrawAndUnwrap(amount, claim)` rather than `Booster.withdraw()`. The Booster's `withdraw` function directly calls the BaseRewardPool; using the BaseRewardPool directly is the pattern used by Aura's own UI and matches Convex's behavior.

### auraBAL Conversion is Irreversible
`BAL → auraBAL` conversion via `crvDepositorWrapper` is one-way at the contract level. Users can exit auraBAL only via the secondary auraBAL/BAL liquidity pool on Balancer. Do NOT implement a `convert-to-aurabal` operation without this warning.

### `get-pools` Rate Limiting
Iterating all pool PIDs via `Booster.poolLength()` + sequential `poolInfo(i)` calls can result in hundreds of RPC calls. Cap at `MAX_POOLS = 50` most-recent active pools (highest PID, filter `shutdown=false`). Report `total_pool_count` separately.

### `onchainos wallet balance --chain 1` Does NOT Support `--output json`
Per Knowledge Hub: `wallet balance --chain 1 --output json` fails on Ethereum mainnet. Use `wallet balance --chain 1` and parse `data.details[0].tokenAssets[0].address` from the plain output.

### Selector Verification Method
Selectors in this document were computed using pycryptodome `Crypto.Hash.keccak` (correct Ethereum keccak256, NOT Python's `hashlib.sha3_256` which is NIST SHA3 and produces wrong hashes). All standard ERC-20 selectors match known values (`approve=0x095ea7b3`, `balanceOf=0x70a08231`, `allowance=0xdd62ed3e`). Convex-shared selectors (`getReward(address,bool)=0x7050ccd9`, `processExpiredLocks(bool)=0x312ff839`, `earned(address)=0x008cc262`) match the Convex design.md exactly.

### E106 Lint Rule
All `contract-call` invocations must have "ask user to confirm" in the same section of SKILL.md. This applies to approve + deposit, withdraw, claim-rewards, and lock-aura.

### Arbitrum L2 Booster
Aura is deployed on Arbitrum (chain 42161) with a separate `L2Booster` contract. The L2 architecture uses a different coordinator pattern. For v0.1.0 of this plugin, only Ethereum mainnet (chain 1) is fully supported. Arbitrum support is noted as future work pending L2 contract address verification.

---

## §7 Appendix: Selector Reference

| Selector | Function | Contract |
|----------|----------|---------|
| `0x43a0d066` | `deposit(uint256,uint256,bool)` | Booster |
| `0x60759fce` | `depositAll(uint256,bool)` | Booster |
| `0x441a3e70` | `withdraw(uint256,uint256)` | Booster |
| `0x958e2d31` | `withdrawAll(uint256)` | Booster |
| `0x081e3eda` | `poolLength()` | Booster |
| `0x1526fe27` | `poolInfo(uint256)` | Booster |
| `0xcc956f3f` | `earmarkRewards(uint256)` | Booster |
| `0x7050ccd9` | `getReward(address,bool)` | BaseRewardPool |
| `0x3d18b912` | `getReward()` | BaseRewardPool |
| `0x38d07436` | `withdraw(uint256,bool)` | BaseRewardPool |
| `0xc32e7202` | `withdrawAndUnwrap(uint256,bool)` | BaseRewardPool |
| `0x49f039a2` | `withdrawAllAndUnwrap(bool)` | BaseRewardPool |
| `0x008cc262` | `earned(address)` | BaseRewardPool |
| `0x70a08231` | `balanceOf(address)` | BaseRewardPool / ERC-20 |
| `0x72f702f3` | `stakingToken()` | BaseRewardPool |
| `0x18160ddd` | `totalSupply()` | BaseRewardPool |
| `0x282d3fdf` | `lock(address,uint256)` | AuraLocker |
| `0x312ff839` | `processExpiredLocks(bool)` | AuraLocker |
| `0x7050ccd9` | `getReward(address,bool)` | AuraLocker |
| `0xdc01f60d` | `claimableRewards(address)` | AuraLocker |
| `0x0483a7f6` | `lockedBalances(address)` | AuraLocker |
| `0x095ea7b3` | `approve(address,uint256)` | ERC-20 |
| `0xdd62ed3e` | `allowance(address,address)` | ERC-20 |
| `0xa9059cbb` | `transfer(address,uint256)` | ERC-20 |
