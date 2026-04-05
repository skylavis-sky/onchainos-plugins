# Plugin Design: Stader ETHx Liquid Staking

**Plugin name:** `stader`  
**DApp:** Stader Labs  
**Category:** defi-protocol  
**Tags:** staking, liquid-staking, eth, ethx, stader  
**Author:** Plugin Dev Pipeline — Phase 1 Researcher Agent  
**Date:** 2026-04-05  
**Status:** Draft

---

## §0 Plugin Meta

| Field | Value |
|---|---|
| plugin_name | stader |
| dapp_name | Stader |
| target_chains | Ethereum (chain ID 1) |
| target_protocols | Liquid Staking (ETHx) |
| category | defi-protocol |

---

## §1 Feasibility Table

| Dimension | Assessment |
|---|---|
| Official Rust SDK | None — Stader has no Rust SDK |
| REST API (off-chain) | None for write ops; contract read via eth_call |
| Official Plugin / Skill | None published |
| Community Skill | None found |
| Integration path | Direct EVM contract calls via `onchainos wallet contract-call` |
| onchainos broadcast needed | Yes — for `stake`, `request-unstake`, `claim` operations |
| Primary chain | Ethereum mainnet (chain ID 1) |
| Other supported chains | Ethereum mainnet only for ETHx staking |
| Key risk | Unstake involves 2-tx flow (approve ETHx + requestWithdraw); withdrawal finalization varies (3–10 days) |

---

## §2 Interface Mapping

### Contract Addresses (Ethereum Mainnet, chain ID 1)

| Contract | Address |
|---|---|
| StaderStakePoolsManager (proxy) | `0xcf5EA1b38380f6aF39068375516Daf40Ed70D299` |
| UserWithdrawManager | `0x9F0491B32DBce587c50c4C43AB303b06478193A7` |
| ETHx Token | `0xA35b1B31Ce002FBF2058D22F30f95D405200A15b` |
| StaderOracle | `0xF64bAe65f6f2a5277571143A24FaaFDFC0C2a737` |
| StaderConfig | `0x4ABEF2263d5A5ED582FC9A9789a41D85b68d69DB` |

---

### Verified Function Selectors

All selectors verified with `cast sig`.

#### StaderStakePoolsManager (`0xcf5EA1b38380f6aF39068375516Daf40Ed70D299`)

| Function Signature | Selector | Verified |
|---|---|---|
| `deposit(address)` | `0xf340fa01` | ✅ cast sig |
| `getExchangeRate()` | `0xe6aa216c` | ✅ cast sig |
| `convertToShares(uint256)` | `0xc6e6f592` | ✅ cast sig |
| `convertToAssets(uint256)` | `0x07a2d13a` | ✅ cast sig |
| `maxDeposit()` | `0x6083e59a` | ✅ cast sig |
| `minDeposit()` | `0x41b3d185` | ✅ cast sig |
| `previewDeposit(uint256)` | `0xef8b30f7` | ✅ cast sig |
| `totalAssets()` | `0x01e1d114` | ✅ cast sig |
| `isVaultHealthy()` | `0xd5c9cfb0` | ✅ cast sig |

#### UserWithdrawManager (`0x9F0491B32DBce587c50c4C43AB303b06478193A7`)

| Function Signature | Selector | Verified |
|---|---|---|
| `requestWithdraw(uint256,address)` | `0xccc143b8` | ✅ cast sig |
| `claim(uint256)` | `0x379607f5` | ✅ cast sig |
| `userWithdrawRequests(uint256)` | `0x911f7acd` | ✅ cast sig |
| `nextRequestId()` | `0x6a84a985` | ✅ cast sig |
| `nextRequestIdToFinalize()` | `0xbbb84362` | ✅ cast sig |
| `getRequestIdsByUser(address)` | `0x7a99ab07` | ✅ cast sig |
| `finalizeUserWithdrawalRequest()` | `0xad8a16dc` | ✅ cast sig |

#### ETHx Token (`0xA35b1B31Ce002FBF2058D22F30f95D405200A15b`)

| Function Signature | Selector | Verified |
|---|---|---|
| `approve(address,uint256)` | `0x095ea7b3` | ✅ standard ERC-20 |
| `balanceOf(address)` | `0x70a08231` | ✅ standard ERC-20 |
| `allowance(address,address)` | `0xdd62ed3e` | ✅ standard ERC-20 |

---

### Operation: `stake`

**Description:** Deposit ETH → receive ETHx. The `deposit(address _receiver)` function is payable.

**Solidity:**
```solidity
// StaderStakePoolsManager: 0xcf5EA1b38380f6aF39068375516Daf40Ed70D299
function deposit(address _receiver) external payable returns (uint256 _shares);
```

**Minimum deposit:** 0.0001 ETH (100000000000000 wei) — protocol enforced.

**ABI encoding:**
- Selector: `0xf340fa01`
- Param: 32-byte padded receiver address
- ETH value: `--amt <wei>`

**onchainos command (0.0001 ETH to receiver):**
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xcf5EA1b38380f6aF39068375516Daf40Ed70D299 \
  --input-data 0xf340fa01000000000000000000000000<RECEIVER_NO_0x_PADDED_32B> \
  --amt 100000000000000
```

---

### Operation: `rates`

**Description:** Query current ETH → ETHx exchange rate and protocol stats.

**Read calls (eth_call, no wallet required):**

1. `getExchangeRate()` — returns exchange rate (1 ETHx in wei, scaled by 1e18)
2. `totalAssets()` — total ETH managed by protocol
3. `minDeposit()` / `maxDeposit()` — deposit bounds
4. `previewDeposit(uint256)` — preview ETHx received for given ETH amount

**No onchainos command needed — direct eth_call via public RPC.**

---

### Operation: `unstake` (request withdrawal)

**Description:** Initiate unstake by approving ETHx allowance and calling `requestWithdraw`.

**2-step flow:**

Step 1 — ERC-20 approve ETHx to UserWithdrawManager:
```bash
# approve(address,uint256) selector = 0x095ea7b3
onchainos wallet contract-call \
  --chain 1 \
  --to 0xA35b1B31Ce002FBF2058D22F30f95D405200A15b \
  --input-data 0x095ea7b3\
0000000000000000000000009f0491b32dbce587c50c4c43ab303b06478193a7\
<AMOUNT_32B_HEX>
```

Step 2 — requestWithdraw:
```bash
# requestWithdraw(uint256,address) selector = 0xccc143b8
onchainos wallet contract-call \
  --chain 1 \
  --to 0x9F0491B32DBce587c50c4C43AB303b06478193A7 \
  --input-data 0xccc143b8<ETHX_AMOUNT_32B><OWNER_32B>
```

**Returns:** `requestId` (uint256) — save for claiming.

---

### Operation: `positions`

**Description:** Query user's ETHx balance and pending withdrawal requests.

**Read calls:**
1. `balanceOf(address)` on ETHx token — current ETHx balance
2. `getRequestIdsByUser(address)` on UserWithdrawManager — array of pending request IDs
3. For each requestId: `userWithdrawRequests(uint256)` — returns `UserWithdrawInfo` struct

**`UserWithdrawInfo` struct:**
```solidity
struct UserWithdrawInfo {
    uint256 ethXAmount;    // ETHx locked
    uint256 ethExpected;   // ETH expected at request time
    uint256 ethFinalized;  // ETH claimable (set after finalization)
    uint256 requestBlock;  // block when request was made
    address owner;
}
```

---

### Operation: `claim`

**Description:** Claim finalized ETH withdrawal.

**Prerequisites:**
- `ethFinalized > 0` in the `userWithdrawRequests` struct for the given requestId

**Solidity:**
```solidity
// UserWithdrawManager: 0x9F0491B32DBce587c50c4C43AB303b06478193A7
function claim(uint256 _requestId) external;
```

**onchainos command:**
```bash
# claim(uint256) selector = 0x379607f5
onchainos wallet contract-call \
  --chain 1 \
  --to 0x9F0491B32DBce587c50c4C43AB303b06478193A7 \
  --input-data 0x379607f5<REQUEST_ID_32B_HEX>
```

---

## §3 User Scenarios

### Scenario 1: Alice stakes 0.01 ETH to earn staking rewards

1. Alice asks the Stader agent to "stake 0.01 ETH."
2. Plugin calls `rates` to show current rate (e.g., 1 ETHx ≈ 1.086 ETH).
3. Plugin calls `previewDeposit(10000000000000000)` to show expected ETHx (~0.0092 ETHx).
4. Plugin asks Alice to confirm. Alice confirms.
5. Plugin executes `deposit(alice_address)` with `--amt 10000000000000000`.
6. Alice receives ~0.0092 ETHx in her wallet.

### Scenario 2: Bob checks his staking position and exchange rate

1. Bob asks "show my Stader position."
2. Plugin calls `balanceOf(bob_address)` on ETHx — returns 1.5 ETHx.
3. Plugin calls `getExchangeRate()` — 1.086 ETH per ETHx.
4. Plugin calls `getRequestIdsByUser(bob_address)` — returns `[42, 43]`.
5. Plugin fetches each request status via `userWithdrawRequests(42)` and `userWithdrawRequests(43)`.
6. Display: "ETHx balance: 1.5 (≈1.629 ETH). Pending withdrawals: 2 requests."

### Scenario 3: Carol unstakes and claims

1. Carol asks to "unstake 1 ETHx."
2. Plugin checks ETHx allowance for UserWithdrawManager. If insufficient, first runs `approve`.
3. Plugin executes `requestWithdraw(1000000000000000000, carol_address)`.
4. Carol receives request ID. Plugin displays: "Unstake requested. Claim when finalized (typically 3–10 days)."
5. Later, Carol asks "claim my Stader withdrawal."
6. Plugin checks `userWithdrawRequests(requestId)` → `ethFinalized > 0`.
7. Plugin executes `claim(requestId)`.
8. Carol receives ETH.

---

## §4 External API Dependencies

| API | URL | Purpose |
|---|---|---|
| Ethereum RPC (read) | `https://ethereum.publicnode.com` | eth_call for rates, positions |
| Ethereum RPC (write) | onchainos wallet | Stake / unstake / claim |

No external REST APIs required — all data from on-chain.

---

## §5 Configuration Parameters

| Parameter | Default | Description |
|---|---|---|
| `chain_id` | `1` | Ethereum mainnet |
| `stader_manager` | `0xcf5EA1b38380f6aF39068375516Daf40Ed70D299` | StaderStakePoolsManager proxy |
| `user_withdraw_manager` | `0x9F0491B32DBce587c50c4C43AB303b06478193A7` | UserWithdrawManager |
| `ethx_token` | `0xA35b1B31Ce002FBF2058D22F30f95D405200A15b` | ETHx ERC-20 token |
| `rpc_url` | `https://ethereum.publicnode.com` | Ethereum RPC |
| `dry_run` | `false` | Simulate without broadcasting |

---

## §6 Notes

- **Min deposit:** 0.0001 ETH (protocol enforced). This is 2× the GUARDRAILS L4 limit of 0.00005 ETH. L4 `stake` test uses 0.0001 ETH — user approval required per GUARDRAILS §Hard Rules rule 1.
- **Withdrawal finalization:** Typically 3–10 days. L4 `claim` test is dry-run only.
- **unstake is 2-tx:** approve ETHx allowance, then requestWithdraw. The plugin handles both.
- **No off-chain API needed:** All rate/position data comes from on-chain calls.
