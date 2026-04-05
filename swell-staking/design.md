# Swell Staking Plugin — Design Document

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `swell-staking` |
| `dapp_name` | Swell Network |
| `target_chains` | Ethereum mainnet (chain ID: 1) |
| `target_protocols` | Liquid staking (swETH), Liquid restaking (rswETH) |
| `category` | staking |
| `version` | 0.1.0 |

---

## §1 接入可行性调研

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无官方 Rust SDK。直接与 EVM 合约交互（eth_call + wallet contract-call）。 |
| SDK 支持哪些技术栈？ | 无 SDK；纯合约交互 |
| 有 REST API？ | 无需 REST API；所有操作通过 Ethereum 合约 eth_call + onchainos |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | 无，需从头实现 |
| 支持哪些链？ | Ethereum mainnet (chain 1) only |
| 是否需要 onchainos 广播？ | Yes — deposit (ETH→swETH / ETH→rswETH) 是链上写操作，需通过 onchainos wallet contract-call |

**接入路径：** 直接 EVM 合约交互（API = eth_call RPC）

---

## §2 接口映射

### 2a 需要接入的操作

| 操作 | 类型 | 说明 |
|------|------|------|
| `rates` | 链下查询 | 获取 swETH/rswETH 当前兑换汇率 |
| `positions` | 链下查询 | 查询用户 swETH 和 rswETH 持仓 |
| `stake` | 链上写操作 | 存入 ETH，接收 swETH (liquid staking) |
| `restake` | 链上写操作 | 存入 ETH，接收 rswETH (liquid restaking via EigenLayer) |

> **注意：** swETH 和 rswETH 的提款（unstake）涉及 NFT 机制（swEXIT NFT）和 1-7 天等待期，且需要专门的 withdrawal manager 合约（未在主合约中暴露）。本版本不实现 unstake；用户可在 Swell app 操作。

### 2b 链下查询

#### `rates` — 获取汇率

**swETH rate via eth_call:**
- Contract: `0xf951E335afb289353dc249e82926178EaC7DEd78` (swETH proxy)
- Function: `swETHToETHRate()` → `uint256` — 1 swETH = X ETH (18 decimals)
- Function: `ethToSwETHRate()` → `uint256` — 1 ETH = X swETH (18 decimals)
- Selector (verified): `swETHToETHRate()` = `0xd68b2cb6`, `ethToSwETHRate()` = `0x0de3ff57`

**rswETH rate via eth_call:**
- Contract: `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` (rswETH proxy)
- Function: `rswETHToETHRate()` → `uint256` — 1 rswETH = X ETH (18 decimals)
- Function: `ethToRswETHRate()` → `uint256` — 1 ETH = X rswETH (18 decimals)
- Selector (verified): `rswETHToETHRate()` = `0xa7b9544e`, `ethToRswETHRate()` = `0x780a47e0`

#### `positions` — 查询持仓

**swETH balance:**
- Contract: `0xf951E335afb289353dc249e82926178EaC7DEd78`
- Function: `balanceOf(address)` → `uint256` (18 decimals)
- Selector: `0x70a08231`

**rswETH balance:**
- Contract: `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0`
- Function: `balanceOf(address)` → `uint256` (18 decimals)
- Selector: `0x70a08231`

### 2c 链上写操作

#### `stake` — ETH → swETH

| Field | Value |
|-------|-------|
| Contract | `0xf951E335afb289353dc249e82926178EaC7DEd78` (swETH proxy) |
| Function | `deposit()` payable |
| Selector | `0xd0e30db0` (verified via `cast sig "deposit()"`) |
| ETH value | `--amt <wei>` e.g., `--amt 50000000000000` = 0.00005 ETH |
| Calldata | `0xd0e30db0` (just the 4-byte selector, no parameters) |

onchainos 命令:
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xf951E335afb289353dc249e82926178EaC7DEd78 \
  --input-data 0xd0e30db0 \
  --amt <wei_amount>
```

#### `restake` — ETH → rswETH

| Field | Value |
|-------|-------|
| Contract | `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` (rswETH proxy) |
| Function | `deposit()` payable |
| Selector | `0xd0e30db0` (verified via `cast sig "deposit()"`) |
| ETH value | `--amt <wei>` |
| Calldata | `0xd0e30db0` |

onchainos 命令:
```bash
onchainos wallet contract-call \
  --chain 1 \
  --to 0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0 \
  --input-data 0xd0e30db0 \
  --amt <wei_amount>
```

---

## §3 用户场景

### 场景 1：查询当前质押汇率

用户说："查询 Swell 的 swETH 汇率" / "What's the current swETH rate?"

1. [链下] eth_call `swETHToETHRate()` on swETH contract → rate_18d
2. [链下] eth_call `ethToSwETHRate()` on swETH contract → inverse_rate
3. [链下] eth_call `rswETHToETHRate()` on rswETH contract → restaking rate
4. 输出：`{"swETH_per_ETH": "0.894...", "ETH_per_swETH": "1.119...", "rswETH_per_ETH": "...", "ETH_per_rswETH": "..."}`

### 场景 2：查询用户持仓

用户说："我有多少 swETH？" / "Show my Swell positions"

1. [链下] 解析钱包地址：onchainos wallet balance --chain 1 → 提取 EVM 地址
2. [链下] eth_call `balanceOf(wallet)` on swETH → swETH balance
3. [链下] eth_call `balanceOf(wallet)` on rswETH → rswETH balance
4. 输出：持仓 + 等值 ETH 价值（用汇率计算）

### 场景 3：质押 ETH 获得 swETH

用户说："帮我质押 0.00005 ETH" / "Stake 0.0001 ETH on Swell"

1. [链下] eth_call `ethToSwETHRate()` → 预估收到的 swETH
2. **向用户确认**: "将质押 X ETH，预计收到 Y swETH。请确认。"
3. [链上] 执行：`onchainos wallet contract-call --chain 1 --to 0xf951... --input-data 0xd0e30db0 --amt <wei>`
4. 提取 txHash，返回结果

### 场景 4：再质押 ETH 获得 rswETH (EigenLayer)

用户说："用 0.0001 ETH 购买 rswETH" / "Restake 0.0001 ETH for EigenLayer rewards"

1. [链下] eth_call `ethToRswETHRate()` → 预估收到的 rswETH
2. **向用户确认**: "将存入 X ETH，预计收到 Y rswETH (EigenLayer restaking)。请确认。"
3. [链上] 执行：`onchainos wallet contract-call --chain 1 --to 0xFAe1... --input-data 0xd0e30db0 --amt <wei>`
4. 提取 txHash，返回结果

---

## §4 外部 API 依赖

| API | URL | 用途 |
|-----|-----|------|
| Ethereum RPC (eth_call) | `https://ethereum.publicnode.com` | 链上读取：汇率、余额 |
| swETH contract | `0xf951E335afb289353dc249e82926178EaC7DEd78` | liquid staking |
| rswETH contract | `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` | liquid restaking |

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `--chain` | u64 | 1 | Chain ID (只支持 Ethereum mainnet = 1) |
| `--amount` | String | 必填 | ETH 数量（人类可读，如 "0.0001"）→ 内部转换为 wei |
| `--dry-run` | bool | false | 模拟模式，不发送真实交易 |

---

## §6 合约地址摘要

| 合约 | 地址 | 验证来源 |
|------|------|---------|
| swETH (Proxy) | `0xf951E335afb289353dc249e82926178EaC7DEd78` | Etherscan |
| swETH (Impl) | `0xce95ba824ae9a4df9b303c0bbf4d605ba2affbfc` | Etherscan |
| rswETH (Proxy) | `0xFAe103DC9cf190eD75350761e95403b7b8aFa6c0` | Etherscan |
| rswETH (Impl) | `0x4796D939b22027c2876d5ce9fde52da9ec4e2362` | Etherscan |

---

## §7 Function Selector 验证清单

| 函数签名 | cast sig 结果 | 状态 |
|---------|-------------|------|
| `deposit()` | `0xd0e30db0` | ✅ verified |
| `depositWithReferral(address)` | `0xc18d7cb7` | ✅ verified |
| `swETHToETHRate()` | `0xd68b2cb6` | ✅ verified (eth_call returns 1.119e18) |
| `ethToSwETHRate()` | `0x0de3ff57` | ✅ verified |
| `getRate()` | `0x679aefce` | ✅ verified |
| `rswETHToETHRate()` | `0xa7b9544e` | ✅ verified (eth_call returns 1.069e18) |
| `ethToRswETHRate()` | `0x780a47e0` | ✅ verified (eth_call returns 0.935e18) |
| `balanceOf(address)` | `0x70a08231` | ✅ standard ERC20 |
| `totalSupply()` | `0x18160ddd` | ✅ standard ERC20 |
