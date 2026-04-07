# Ion Protocol — Plugin Design

## §0 Plugin Meta

- **plugin_name**: ion-protocol
- **dapp_name**: Ion Protocol
- **version**: 0.1.0
- **target_chains**: [ethereum (chain 1)]
- **category**: defi-protocol
- **integration_path**: direct contract calls (IonPool + GemJoin)

---

## §1 Feasibility

| Level | Operation | Feasibility | Notes |
|-------|-----------|-------------|-------|
| L1 | Compile & link | PASS | Direct contract calls, no SDK needed |
| L2 | Read ops (dry-run) | PASS | All read selectors verified on-chain |
| L3 | Write dry-run | PASS | ABI-encode verified; proof=[] confirmed open |
| L4 | Live write ops | PASS | Whitelist roots = 0x00...00 (open access) |

**Key feasibility finding:** The Whitelist contract (`0x7E317f99aA313669AaCDd8dB3927ff3aCB562dAD`) has both `lendersRoot` and `borrowersRoot` set to `bytes32(0)`, meaning access is **open to all addresses** — empty proof `bytes32[] = []` works for every user.

---

## §2 Interface Mapping

### Pool Registry (Ethereum Mainnet, chain 1)

Ion Protocol deploys one `IonPool` per collateral/lend pair. Each pool has exactly **one ilk at ilkIndex=0**.

| Pool Name | IonPool Address | GemJoin Address | Collateral Token | Lend Token | Approx TVL |
|-----------|----------------|----------------|-----------------|------------|------------|
| rsETH/wstETH | `0x0000000000E33e35EE6052fae87bfcFac61b1da9` | `0x3bC3AC09d1ee05393F2848d82cb420f347954432` | rsETH `0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7` | wstETH | ~6.5 wstETH (primary market) |
| rswETH/wstETH | `0x00000000007C8105548f9d0eE081987378a6bE93` | `0xD696f9EA3299113324B9065ab19b70758256cf16` | rswETH `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` | wstETH | ~0.7 wstETH |
| ezETH/WETH | `0x00000000008a3A77bd91bC738Ed2Efaa262c3763` | `0xe3692b2E55Eb2494cA73610c3b027F53815CCD39` | ezETH `0xbf5495Efe5DB9ce00f80364C8B423567e58d2110` | WETH | ~0.006 WETH (low activity) |
| weETH/wstETH | `0x0000000000eaEbd95dAfcA37A39fd09745739b78` | `0x3f6119b0328c27190be39597213ea1729f061876` | weETH `0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee` | wstETH | ~2.2 wstETH |

**Shared contracts (all pools):**
- **Whitelist**: `0x7E317f99aA313669AaCDd8dB3927ff3aCB562dAD` (roots=0, open access)
- **YieldOracle**: `0x437CC840e234C2127f54CD59B0B18aF59c586760`
- **IonPool Implementation**: `0x77ca0d4b78d8b4f3c71e20f8c8771c4cb7abe201`

**Token addresses:**
- wstETH: `0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0`
- WETH: `0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2`

---

### On-chain Write Operations (EVM)

All selectors verified via `pycryptodome` keccak256 and tested via live `eth_call` on Ethereum mainnet.

#### Lender Side (supply wstETH/WETH to earn yield)

| Operation | Contract | Function Signature | Selector (verified ✅) | Param Order |
|-----------|----------|-------------------|----------------------|-------------|
| Approve lend token | wstETH or WETH | `approve(address,uint256)` | `0x095ea7b3` | spender=IonPool, amount |
| Supply lend token | IonPool | `supply(address,uint256,bytes32[])` | `0x7ca5643d` | user, amount\_wad, proof=[] |
| Withdraw lend token | IonPool | `withdraw(address,uint256)` | `0xf3fef3a3` | receiverOfUnderlying, amount\_wad |

#### Borrower Side (deposit LRT collateral, borrow wstETH/WETH)

The borrower flow is a **4-step process**: approve LST to GemJoin → GemJoin.join → depositCollateral → borrow.

| Step | Operation | Contract | Function Signature | Selector (verified ✅) | Param Order |
|------|-----------|----------|-------------------|----------------------|-------------|
| 1 | Approve collateral to GemJoin | LST token (rsETH/rswETH/ezETH/weETH) | `approve(address,uint256)` | `0x095ea7b3` | spender=GemJoin, amount |
| 2 | Deposit collateral into gem system | GemJoin | `join(address,uint256)` | `0x3b4da69f` | user, amount\_wad |
| 3 | Move gem to vault collateral | IonPool | `depositCollateral(uint8,address,address,uint256,bytes32[])` | `0x918a2f42` | ilkIndex=0, user, depositor=user, amount\_wad, proof=[] |
| 4 | Borrow lend token | IonPool | `borrow(uint8,address,address,uint256,bytes32[])` | `0x9306f2f8` | ilkIndex=0, user, recipient=user, normalizedDebt, proof=[] |

#### Repay and Withdraw Collateral

| Step | Operation | Contract | Function Signature | Selector (verified ✅) | Param Order |
|------|-----------|----------|-------------------|----------------------|-------------|
| 1 | Approve lend token to IonPool | wstETH or WETH | `approve(address,uint256)` | `0x095ea7b3` | spender=IonPool, amount |
| 2 | Repay debt | IonPool | `repay(uint8,address,address,uint256)` | `0x8459b437` | ilkIndex=0, user, payer=user, normalizedDebt |
| 3 | Move collateral from vault to gem | IonPool | `withdrawCollateral(uint8,address,address,uint256)` | `0x743f9c0c` | ilkIndex=0, user, recipient=user, amount\_wad |
| 4 | Withdraw collateral from gem system | GemJoin | `exit(address,uint256)` | `0xef693bed` | user, amount\_wad |

---

### Off-chain Read Operations

All selectors verified via live `eth_call` against rsETH/wstETH pool (primary pool).

| Operation | Contract | Function Signature | Selector (verified ✅) | Returns |
|-----------|----------|-------------------|----------------------|---------|
| Get pool name | IonPool | `name()` | `0x06fdde03` | string (e.g. "Ion rsETH wstETH Token") |
| Get underlying (lend) token | IonPool | `underlying()` | `0x6f307dc3` | address |
| Get collateral token at ilk 0 | IonPool | `getIlkAddress(uint256)` | `0xefff005f` | address |
| Get accumulated rate (RAY, 1e27) | IonPool | `rate(uint8)` | `0x3c04b547` | uint256 |
| Get borrow APR (per-second RAY) | IonPool | `getCurrentBorrowRate(uint8)` | `0x6908d3df` | (uint256 borrowRate, uint256 reserveFactor) |
| Get user vault (collateral+debt) | IonPool | `vault(uint8,address)` | `0x9a3db79b` | (uint256 collateral\_wad, uint256 normalizedDebt\_wad) |
| Get user collateral in vault | IonPool | `collateral(uint8,address)` | `0x6f424d76` | uint256 (WAD) |
| Get user normalized debt | IonPool | `normalizedDebt(uint8,address)` | `0x57fc90b2` | uint256 (WAD) |
| Get total lender supply | IonPool | `totalSupply()` | `0x18160ddd` | uint256 (WAD) |
| Get user lender balance | IonPool | `balanceOf(address)` | `0x70a08231` | uint256 (WAD) |
| Check if pool is paused | IonPool | `paused()` | `0x5c975abb` | bool |
| Get GemJoin pool address | GemJoin | `POOL()` | `0x7535d246` | address |
| Get GemJoin collateral token | GemJoin | `GEM()` | `0x4dc65411` | address |
| Get GemJoin ilk index | GemJoin | `ILK_INDEX()` | `0xed0cee97` | uint8 |
| Get total collateral in GemJoin | GemJoin | `totalGem()` | `0x83e8d3b8` | uint256 (WAD) |

---

### Data Encoding Notes

**normalizedDebt calculation:**
- `normalizedDebt = actualDebt_wad * RAY / rate`
- `rate` = `IonPool.rate(0)` in RAY (1e27 precision)
- When borrowing X wstETH: `normalizedDebt = X_wad * 1e27 / rate`
- When repaying: use `normalizedDebt(0, user)` to get exact amount to repay

**Borrow APY calculation:**
- `getCurrentBorrowRate(0)` returns per-second rate in RAY
- Annual: `(borrowRate_per_sec / 1e27)^31_536_000 - 1`

**proof parameter:**
- All whitelisted functions accept `bytes32[] calldata proof`
- Currently OPEN (roots=0): pass empty array `[]` → ABI encoded as `0x...00000040 00000000`
- Full ABI encoding for empty bytes32[]: offset=0x40 (32 bytes), length=0x00 (0 elements)

---

## §3 User Scenarios

### Scenario A — Lend wstETH (earn yield)

A user supplies wstETH to the rsETH/wstETH pool to earn interest from borrowers.

```
1. wstETH.approve(ionPool_rsETH, amount)
2. IonPool_rsETH.supply(user, amount_wad, [])
   → user receives ion-wstETH supply tokens (ERC-20)
3. IonPool_rsETH.withdraw(user, amount_wad)
   → burns supply tokens, returns wstETH
```

### Scenario B — Borrow wstETH against rsETH collateral (leveraged staking)

A user uses rsETH as collateral to borrow wstETH for a leveraged staking position.

```
1. rsETH.approve(gemJoin_rsETH, collateral_amount)
2. GemJoin_rsETH.join(user, collateral_amount)       [moves rsETH to GemJoin]
3. IonPool_rsETH.depositCollateral(0, user, user, collateral_amount, [])
                                                       [credits vault.collateral]
4. IonPool_rsETH.borrow(0, user, user, normalizedDebt, [])
                                                       [transfers wstETH to user]
```

### Scenario C — Repay and retrieve collateral

```
1. wstETH.approve(ionPool_rsETH, repay_amount)
2. IonPool_rsETH.repay(0, user, user, normalizedDebt)   [reduces debt]
3. IonPool_rsETH.withdrawCollateral(0, user, user, collateral_amount)
                                                         [moves vault → gem]
4. GemJoin_rsETH.exit(user, collateral_amount)           [moves gem → rsETH to user]
```

### Scenario D — Read position

```
vault(0, user) → (collateral_wad, normalizedDebt_wad)
actual_debt = normalizedDebt * rate / 1e27
```

### Scenario E — Get market data

```
For each pool:
  rate(0)                      → accumulated rate (RAY)
  getCurrentBorrowRate(0)      → per-second borrow rate + reserve factor
  totalSupply()                → total wstETH/WETH lent out
  getIlkAddress(0)             → collateral token address
  underlying()                 → lend token address
  
  utilization ≈ totalDebt / totalSupply
  totalDebt uses calculateRewardAndDebtDistribution() for accrued value
```

---

## §4 External API Dependencies

None. All data comes directly from on-chain contract calls.

- **RPC endpoint**: `https://ethereum.publicnode.com` (Ethereum mainnet, chain 1)
  - Use `publicnode.com` instead of `llamarpc.com` to avoid rate limits on multi-call reads
- No subgraph, no REST API, no TheGraph required

---

## §5 Config Parameters

```yaml
# plugin.yaml
chains:
  - id: 1
    rpc: https://ethereum.publicnode.com

pools:
  rsETH_wstETH:
    ionPool: "0x0000000000E33e35EE6052fae87bfcFac61b1da9"
    gemJoin: "0x3bC3AC09d1ee05393F2848d82cb420f347954432"
    collateral: "0xA1290d69c65A6Fe4DF752f95823fae25cB99e5A7"  # rsETH
    lendToken: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0"  # wstETH
    ilkIndex: 0
    symbol: "rsETH"
    
  rswETH_wstETH:
    ionPool: "0x00000000007C8105548f9d0eE081987378a6bE93"
    gemJoin: "0xD696f9EA3299113324B9065ab19b70758256cf16"
    collateral: "0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0"  # rswETH
    lendToken: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0"  # wstETH
    ilkIndex: 0
    symbol: "rswETH"
    
  ezETH_WETH:
    ionPool: "0x00000000008a3A77bd91bC738Ed2Efaa262c3763"
    gemJoin: "0xe3692b2E55Eb2494cA73610c3b027F53815CCD39"
    collateral: "0xbf5495Efe5DB9ce00f80364C8B423567e58d2110"  # ezETH
    lendToken: "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2"  # WETH
    ilkIndex: 0
    symbol: "ezETH"
    
  weETH_wstETH:
    ionPool: "0x0000000000eaEbd95dAfcA37A39fd09745739b78"
    gemJoin: "0x3f6119b0328c27190be39597213ea1729f061876"
    collateral: "0xCd5fE23C85820F7B72D0926FC9b05b43E359b7ee"  # weETH
    lendToken: "0x7f39C581F595B53c5cb19bD0b3f8dA6c935E2Ca0"  # wstETH
    ilkIndex: 0
    symbol: "weETH"

whitelist: "0x7E317f99aA313669AaCDd8dB3927ff3aCB562dAD"
yieldOracle: "0x437CC840e234C2127f54CD59B0B18aF59c586760"
```

---

## §6 Known Risks / Gotchas

### 1. LST-as-collateral direction (critical)
Ion Protocol is **collateral-in, lend-out**: users deposit LRTs (rsETH, weETH, etc.) as **collateral** and borrow wstETH or WETH. This is the **opposite** of typical Aave-style lending where you supply a token to earn yield. The "supply" operation in Ion (`IonPool.supply`) is for **lenders** of wstETH/WETH, not for depositing LST collateral. Do NOT confuse `supply()` (lender) with `depositCollateral()` (borrower).

### 2. 4-step borrower flow via GemJoin
Unlike Aave (single `supply()` call), Ion borrowers must execute 4 separate transactions: approve collateral → `GemJoin.join()` → `IonPool.depositCollateral()` → `IonPool.borrow()`. Each must complete before the next. Add 3-second delays between steps to avoid nonce collisions.

### 3. normalizedDebt vs actual debt
Ion stores debt as `normalizedDebt = actualDebt / rate`. The `borrow()` and `repay()` functions take `normalizedDebt` (WAD), not actual debt amount. Compute: `normalizedDebt = amount_to_borrow * 1e27 / rate(0)`. For repay-all, read `normalizedDebt(0, user)` directly.

### 4. --output json not supported on chain 1
`onchainos wallet balance --chain 1 --output json` fails with EOF. Use `onchainos wallet addresses` and filter by `chainIndex == "1"` to resolve the wallet address. See [kb/protocols/symbiotic.md#wallet-resolve-chain-1].

### 5. Ethereum mainnet RPC rate limits
Use `https://ethereum.publicnode.com` as the RPC for chain 1. `eth.llamarpc.com` and `cloudflare-eth.com` rate-limit or block sandbox IPs under multi-call load.

### 6. Whitelist currently open - may change
Both `lendersRoot` and `borrowersRoot` on the Whitelist contract are `bytes32(0)`, so `proof=[]` works for all users today. If governance updates the roots, users must obtain Merkle proofs from the Ion frontend. Always check roots before assuming open access.

### 7. GemJoin approval is to GemJoin (not IonPool)
For collateral deposit: `LST.approve(gemJoin, amount)` then `GemJoin.join()`. For lend token supply: `wstETH.approve(ionPool, amount)` then `IonPool.supply()`. For repay: `wstETH.approve(ionPool, amount)` then `IonPool.repay()`. The approval targets are different for each step.

### 8. Dust floor
Each ilk has a `dust` parameter (minimum vault debt, ~4e45 RAD ≈ 4e-3 wstETH in real terms). Borrowing below `dust` will revert. Minimum test borrow should be >0.01 wstETH equivalent.

### 9. Low TVL on ezETH/WETH pool
The ezETH/WETH pool has ~0.006 WETH total supply. It may be near sunset or have zero liquidity available to borrow. Recommend focusing L4 tests on rsETH/wstETH pool (6.5 wstETH TVL, highest activity).

### 10. No claim-rewards operation
Ion Protocol does not have a separate reward token or claim operation. Lenders earn yield automatically via the `supplyFactor` accrual mechanism (similar to Aave's aToken rebasing). No separate `claim-rewards` command is needed.

---

## §7 Test Ordering

For Ion Protocol, the recommended L4 test order is:

```
get-markets → get-position (read ops)
  → supply-lend (supply wstETH to earn yield)
    → borrow (supply rsETH collateral → borrow wstETH)
      → repay (repay wstETH debt)
        → withdraw-collateral (withdraw rsETH)
          → withdraw-lend (withdraw lent wstETH)
```

Use the **rsETH/wstETH pool** as the primary test pool (highest TVL, ~38.7% borrow APY confirms active market).

Test wallet needs both rsETH (for collateral) and wstETH (for lending/repaying). Fund with at least:
- 0.01 wstETH (to supply as lender)
- 0.01 rsETH (to use as collateral)
- ETH for gas on Ethereum mainnet

---

## §8 ABI Encoding Reference

### supply(address user, uint256 amount, bytes32[] proof=[])
```
selector: 0x7ca5643d
arg1: user address (32 bytes, left-padded)
arg2: amount in WAD (32 bytes)
arg3: offset to dynamic array = 0x60 (96 bytes)
arg4: array length = 0x00 (empty)
```
Full calldata (empty proof): `0x7ca5643d` + user(32) + amount(32) + `0000...0060` + `0000...0000`

### depositCollateral(uint8 ilkIndex=0, address user, address depositor, uint256 amount, bytes32[] proof=[])
```
selector: 0x918a2f42
arg1: ilkIndex = 0x00 (32 bytes)
arg2: user (32 bytes)
arg3: depositor = user (32 bytes)
arg4: amount (32 bytes)
arg5: offset to proof = 0xa0 (160 bytes from after selector)
arg6: proof length = 0x00
```

### borrow(uint8 ilkIndex=0, address user, address recipient, uint256 normalizedDebt, bytes32[] proof=[])
```
selector: 0x9306f2f8
arg1: ilkIndex = 0x00
arg2: user
arg3: recipient = user
arg4: normalizedDebt (WAD) = actual_borrow * 1e27 / rate
arg5: proof offset = 0xa0
arg6: proof length = 0x00
```

### repay(uint8 ilkIndex=0, address user, address payer, uint256 normalizedDebt)
```
selector: 0x8459b437
arg1: ilkIndex = 0x00
arg2: user
arg3: payer = user
arg4: normalizedDebt = IonPool.normalizedDebt(0, user)
```
Note: No dynamic proof array — repay does not require whitelist proof.

### withdrawCollateral(uint8 ilkIndex=0, address user, address recipient, uint256 amount)
```
selector: 0x743f9c0c
arg1: ilkIndex = 0x00
arg2: user
arg3: recipient = user
arg4: amount (WAD) — use vault(0, user).collateral for full withdrawal
```
