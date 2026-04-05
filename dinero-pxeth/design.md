# Dinero pxETH Plugin — Design Document

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `dinero-pxeth` |
| dapp_name | Dinero pxETH |
| target_chains | Ethereum mainnet (chain ID: 1) |
| target_protocols | PirexEth (ETH→pxETH), AutoPxEth/apxETH (ERC-4626 vault) |
| category | defi-protocol |
| tags | liquid-staking, pxETH, apxETH, ERC-4626, dinero, pirex |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无专用 Rust SDK；直接 ABI 编码合约调用 |
| SDK 支持哪些技术栈？ | JavaScript (web3); 本插件直接 ABI 编码 |
| 有 REST API？ | 无公开 REST API for rates; 纯链上 eth_call |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | Frax Ether (frax-ether) 是完美参考 (同模式: ETH→液态代币→ERC-4626 vault) |
| 支持哪些链？ | Ethereum mainnet 仅 (chain ID: 1) |
| 是否需要 onchainos 广播？ | 是 — ETH deposit 和 pxETH→apxETH deposit 均需链上广播 |

**接入路径**：参考已有 frax-ether Skill（同模式），直接 ABI 编码合约调用，通过 `onchainos wallet contract-call` 广播。

**⚠️ 协议状态**：PirexEth 主合约 (`0xD664b74274DfEB538d9baC494F3a4760828B02b0`) 当前处于 **paused** 状态（`paused() = true`）。所有 `deposit`、`initiateRedemption`、`instantRedeemWithPxEth` 等函数均有 `whenNotPaused` 保护，当前调用会 revert。

apxETH vault (`0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6`) 无 pause 机制，`deposit`/`redeem` 仍正常工作（用于持有 pxETH 的用户 stake→apxETH）。

---

## §2 接口映射

### 需要接入的操作表

| 操作 | 类型 | 说明 | 状态 |
|------|------|------|------|
| deposit | 链上写 | ETH → pxETH (PirexEth.deposit, payable) | ⚠️ PAUSED |
| stake | 链上写 | pxETH → apxETH (ERC-4626 deposit) | ✅ Active |
| redeem | 链上写 | apxETH → pxETH (ERC-4626 redeem) | ✅ Active |
| rates | 链下查询 | 查询 apxETH APR 和汇率 | ✅ Active |
| positions | 链下查询 | 查询 pxETH + apxETH 余额和价值 | ✅ Active |

---

### 链下查询表

#### `rates` — 获取 apxETH APR 和汇率

- **eth_call**: `convertToAssets(1e18)` on apxETH → current pxETH per apxETH
  - Selector: `0x07a2d13a` (cast sig "convertToAssets(uint256)" ✅)
- **eth_call**: `totalAssets()` on apxETH → total pxETH in vault
  - Selector: `0x01e1d114` (cast sig "totalAssets()" ✅)
- **eth_call**: `totalSupply()` on apxETH → total apxETH shares
  - Selector: `0x18160ddd` (cast sig "totalSupply()" ✅)
- No REST API available; all data from on-chain

Verified on-chain:
- apxETH totalAssets: ~2598 ETH worth of pxETH
- convertToAssets(1e18): ~1.116e18 (1 apxETH ≈ 1.116 pxETH)
- pxETH totalSupply: ~2981 ETH

#### `positions` — 查询用户持仓

- **eth_call**: `balanceOf(address)` on pxETH (0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6)
  - Selector: `0x70a08231` (cast sig "balanceOf(address)" ✅)
- **eth_call**: `balanceOf(address)` on apxETH (0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6)
  - Selector: `0x70a08231`
- **eth_call**: `convertToAssets(uint256)` on apxETH → pxETH value of apxETH balance
  - Selector: `0x07a2d13a`

---

### 链上写操作表

#### `deposit` — ETH → pxETH ⚠️ PAUSED

**Contract**: PirexEth `0xD664b74274DfEB538d9baC494F3a4760828B02b0`

**Function**: `deposit(address receiver, bool shouldCompound)` — payable

**Selector**: `0xadc9740c` (cast sig "deposit(address,bool)" ✅)

**Calldata construction**:
```
0xadc9740c
+ receiver address padded 32 bytes
+ shouldCompound (bool) padded 32 bytes (0 = false = get pxETH; 1 = true = auto-compound to apxETH)
```

**onchainos command**:
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xD664b74274DfEB538d9baC494F3a4760828B02b0 \
  --input-data 0xadc9740c<receiver_padded><00..01 for shouldCompound> \
  --amt <wei_amount>
```

Note: `shouldCompound=false` → receive pxETH; `shouldCompound=true` → auto-compound to apxETH directly.

⚠️ This operation is currently paused. The plugin will note this and skip L4 testing.

---

#### `stake` — pxETH → apxETH ✅

**Contract**: apxETH (ERC-4626) `0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6`

**Step 1 — ERC-20 approve**:
- Token: pxETH `0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6`
- Spender: apxETH `0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6`
- Selector: `0x095ea7b3` (cast sig "approve(address,uint256)" ✅)

**Step 2 — ERC-4626 deposit**:
- Function: `deposit(uint256 assets, address receiver)`
- Selector: `0x6e553f65` (cast sig "deposit(uint256,address)" ✅)
- `assets` = pxETH amount in wei
- `receiver` = user wallet address

**onchainos commands**:
```bash
# Step 1: approve pxETH to apxETH vault
onchainos wallet contract-call \
  --chain 1 \
  --to 0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6 \
  --input-data 0x095ea7b3<spender_padded><amount_hex>

# Step 2: deposit pxETH into apxETH
onchainos wallet contract-call \
  --chain 1 \
  --to 0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6 \
  --input-data 0x6e553f65<amount_padded><receiver_padded>
```

---

#### `redeem` — apxETH → pxETH ✅

**Contract**: apxETH (ERC-4626) `0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6`

**Function**: `redeem(uint256 shares, address receiver, address owner)`
- Selector: `0xba087652` (cast sig "redeem(uint256,address,address)" ✅)
- `shares` = apxETH amount in wei
- `receiver` = user wallet (receives pxETH)
- `owner` = user wallet (owns the apxETH)

**onchainos command**:
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6 \
  --input-data 0xba087652<shares_padded><receiver_padded><owner_padded>
```

---

## §3 用户场景

### 场景 1: 存入 ETH 获得 pxETH (当前暂停)

**用户说**: "deposit 0.00005 ETH to get pxETH on Dinero"

**Agent 动作**:
1. 检查 PirexEth.paused() → true，通知用户当前协议暂停
2. 返回状态信息，说明暂停原因
3. 若仍要提交: 计算 amount_wei = 50000000000000 wei
4. 构造 calldata: `0xadc9740c` + receiver_padded + `0000..0000` (shouldCompound=false)
5. **Ask user to confirm** before proceeding
6. `onchainos wallet contract-call --chain 1 --to 0xD664... --input-data ... --amt 50000000000000`

### 场景 2: 质押 pxETH 获得 apxETH 收益

**用户说**: "stake my pxETH to earn yield with Dinero apxETH"

**Agent 动作**:
1. 查询 pxETH 余额 via eth_call balanceOf
2. 确认数量 (如 0.00005 pxETH = 50000000000000 wei)
3. Step 1 — **Ask user to confirm** approve
4. 链上: ERC-20 approve pxETH to apxETH vault
5. Step 2 — **Ask user to confirm** deposit
6. 链上: ERC-4626 deposit pxETH → apxETH
7. 返回收到的 apxETH 数量和当前 APR

### 场景 3: 赎回 apxETH 换回 pxETH

**用户说**: "unstake my apxETH back to pxETH"

**Agent 动作**:
1. 查询 apxETH 余额
2. **Ask user to confirm** before proceeding
3. 链上: ERC-4626 redeem apxETH → pxETH
4. 返回收到的 pxETH 数量和 txHash

### 场景 4: 查询收益率和汇率

**用户说**: "what is the current apxETH APR and exchange rate?"

**Agent 动作**:
1. eth_call `convertToAssets(1e18)` on apxETH → exchange rate
2. eth_call `totalAssets()` on apxETH → TVL
3. 返回汇率 (apxETH/pxETH), TVL, 协议状态

### 场景 5: 查询持仓

**用户说**: "show my Dinero pxETH positions"

**Agent 动作**:
1. 解析钱包地址 via `onchainos wallet addresses`
2. eth_call `balanceOf(address)` on pxETH
3. eth_call `balanceOf(address)` on apxETH
4. eth_call `convertToAssets(apxeth_balance)` to get pxETH value
5. 返回 pxETH balance, apxETH balance, underlying pxETH value

---

## §4 外部 API 依赖

| API | 用途 |
|-----|------|
| `https://ethereum.publicnode.com` | Ethereum mainnet RPC (eth_call for balances, rates) |

No REST API for rates — all data fetched from on-chain via eth_call.

---

## §5 配置参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| chain | 1 | Ethereum mainnet |
| dry_run | false | 模拟模式，不广播 |
| PirexEth | `0xD664b74274DfEB538d9baC494F3a4760828B02b0` | PirexEth main contract (ETH→pxETH) |
| pxETH | `0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6` | pxETH ERC-20 token |
| apxETH | `0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6` | apxETH ERC-4626 vault |

---

## §6 Key Contract Addresses (Ethereum Mainnet)

| Contract | Address | Verified |
|----------|---------|---------|
| PirexEth (main) | `0xD664b74274DfEB538d9baC494F3a4760828B02b0` | ✅ Etherscan |
| pxETH token | `0x04C154b66CB340F3Ae24111CC767e0184Ed00Cc6` | ✅ totalSupply ~2981 ETH |
| apxETH vault | `0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6` | ✅ totalAssets ~2598 ETH |
| ValidatorQueue | `0x9E0d7D79735e1c63333128149c7b616a0dC0bBDb` | ✅ Etherscan |
| UpxEth | `0x5BF2419a33f82F4C1f075B4006d7fC4104C43868` | ✅ Etherscan |

---

## §7 Function Selector Verification

| Function Signature | cast sig Result | Status |
|-------------------|----------------|--------|
| `deposit(address,bool)` | `0xadc9740c` | ✅ (PirexEth ETH→pxETH; currently paused) |
| `instantRedeemWithPxEth(uint256,address)` | `0xb4a26569` | ✅ (paused) |
| `deposit(uint256,address)` | `0x6e553f65` | ✅ (apxETH ERC-4626) |
| `redeem(uint256,address,address)` | `0xba087652` | ✅ (apxETH ERC-4626) |
| `convertToAssets(uint256)` | `0x07a2d13a` | ✅ |
| `totalAssets()` | `0x01e1d114` | ✅ |
| `totalSupply()` | `0x18160ddd` | ✅ |
| `balanceOf(address)` | `0x70a08231` | ✅ |
| `approve(address,uint256)` | `0x095ea7b3` | ✅ |
| `paused()` | `0x5c975abb` | ✅ (PirexEth only) |
