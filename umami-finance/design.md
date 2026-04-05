# Umami Finance Plugin — design.md

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `umami-finance` |
| dapp_name | Umami Finance |
| version | 0.1.0 |
| target_chains | EVM (Arbitrum, chain 42161) |
| target_protocols | Yield Vaults (ERC-4626 GM Vaults on GMX V2) |
| category | defi |
| tags | yield, vault, arbitrum, gmx, erc4626 |
| author | GeoGu360 |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | 无 Rust SDK |
| SDK 支持哪些技术栈？ | 无官方 SDK；合约为 ERC-4626 标准，直接通过 ABI 调用 |
| 有 REST API？ | 无公开 REST API — 所有数据通过链上 eth_call 读取 |
| 有官方 Skill？ | 无 |
| 开源社区有类似 Skill？ | 无直接参考 |
| 支持哪些链？ | Arbitrum (chain 42161) |
| 是否需要 onchainos 广播？ | Yes — deposit/redeem 需要 onchainos wallet contract-call |

**接入路径：** 直接合约调用（ERC-4626 标准）— 不依赖 SDK 或 REST API，所有查询通过 eth_call，所有写操作通过 onchainos wallet contract-call

---

## §2 接口映射

### 需要接入的操作表

| 操作 | 类型 | 说明 |
|------|------|------|
| list-vaults | 链下查询 | 列出所有 GM vaults 及 TVL、PPSShare、APR |
| vault-info | 链下查询 | 查询指定 vault 的详细信息（TVL、pricePerShare、decimals） |
| positions | 链下查询 | 查询用户在各 vault 的持仓（share 余额、对应资产价值） |
| deposit | 链上写操作 | 存入资产到指定 vault（ERC-4626 deposit） |
| redeem | 链上写操作 | 从 vault 赎回资产（ERC-4626 redeem） |

### 合约地址（GM Vaults on Arbitrum 42161）

| Vault | 名称 | 合约地址 | 底层资产 | 资产地址 |
|-------|------|---------|---------|---------|
| gmUSDC (ETH-backed) | GM USDC ETH Vault | `0x959f3807f0Aa7921E18c78B00B2819ba91E52FeF` | USDC | `0xaf88d065e77c8cc2239327c5edb3a432268e5831` |
| gmUSDC (BTC-backed) | GM USDC BTC Vault | `0x5f851F67D24419982EcD7b7765deFD64fBb50a97` | USDC | `0xaf88d065e77c8cc2239327c5edb3a432268e5831` |
| gmWETH | GM WETH Vault | `0x4bCA8D73561aaEee2D3a584b9F4665310de1dD69` | WETH | `0x82af49447d8a07e3bd95bd0d56f35241523fbab1` |
| gmWBTC | GM WBTC Vault | `0xcd8011AaB161A75058eAb24e0965BAb0b918aF29` | WBTC | `0x2f2a2543b76a4166549f7aab2e75bef0aefc5b0f` |

| 辅助合约 | 说明 | 地址 |
|---------|------|------|
| AggregateVault | 聚合 vault 管理 | `0x0Ca62954b46AfEe430D645dA493C6C783448C4eD` |
| StorageViewer | 链上数据视图 | `0xAF037670ed7B2ca464c61BBfC07365747038250f` |
| RequestHandler | 处理存取款请求 | `0x33a4484d5E1754210bBFBe05d3F51cDD33cC1E91` |

### 链下查询表（eth_call）

#### list-vaults / vault-info

对每个 vault 合约调用以下 ERC-4626 函数：

| 函数 | Selector | 返回值 |
|------|---------|-------|
| `totalAssets()` | `0x01e1d114` | uint256 — 总底层资产（已验证 ✅ via eth_call） |
| `totalSupply()` | `0x18160ddd` | uint256 — 总份额 |
| `convertToAssets(uint256 shares)` | `0x07a2d13a` | uint256 — 每份额对应资产值（price per share） |
| `maxDeposit(address)` | `0x402d267d` | uint256 — 最大允许存入量（已验证 ✅） |
| `previewDeposit(uint256 assets)` | `0xef8b30f7` | uint256 — 预计获得份额 |
| `previewRedeem(uint256 shares)` | `0x4cdad506` | uint256 — 预计取回资产 |
| `asset()` | `0x38d52e0f` | address — 底层资产地址 |
| `decimals()` | `0x313ce567` | uint8 — vault token decimals |
| `balanceOf(address)` | `0x70a08231` | uint256 — 用户份额余额 |

所有 selector 均通过 `cast sig` 验证 ✅

#### positions

```
对每个 vault 调用 balanceOf(wallet_address)
若余额 > 0，再调用 convertToAssets(balance) 得到对应资产价值
```

### 链上写操作表

#### deposit（存入资产）

**注意：Umami GM Vaults 使用自定义的 deposit 函数（非标准 ERC-4626）**
实际函数包含 minSharesOut 参数（滑点保护）。

**前置：ERC-20 approve**

```
approve(address spender, uint256 amount)
selector: 0x095ea7b3
calldata: 0x095ea7b3 + {vault_addr_padded_32} + {amount_uint256}
to: 底层资产合约地址（USDC/WETH/WBTC）
```

**deposit 调用（自定义函数）**

```
deposit(uint256 assets, uint256 minSharesOut, address receiver)
selector: 0x8dbdbe6d (已通过 cast sig + 链上 tx 分析验证 ✅)
calldata: 0x8dbdbe6d + {amount_uint256} + {min_shares_uint256} + {receiver_addr_padded_32}
to: vault 合约地址
chain: 42161
```

onchainos 命令：
```bash
onchainos wallet contract-call --chain 42161 --to <VAULT_ADDR> --input-data 0x8dbdbe6d<amount_hex><min_shares_hex><receiver_hex> --force
```

**重要限制：** Umami GM vaults 的 deposit() 函数需要 keeper/oracle 协调（Chainlink Data Streams），直接用户调用在生产环境中会被阻断（类似 GMX V2 的 keeper 模型）。

#### redeem（赎回资产）

```
redeem(uint256 shares, uint256 minAssetsOut, address receiver, address owner)
selector: 0x0169a996 (已通过 cast sig + 链上 tx 分析验证 ✅)
calldata: 0x0169a996 + {shares_uint256} + {min_assets_uint256} + {receiver_addr_padded_32} + {owner_addr_padded_32}
to: vault 合约地址
chain: 42161
```

onchainos 命令：
```bash
onchainos wallet contract-call --chain 42161 --to <VAULT_ADDR> --input-data 0x0169a996<shares_hex><min_assets_hex><receiver_hex><owner_hex> --force
```

---

## §3 用户场景

### 场景 1：查看所有 vault 收益率和 TVL

用户说："Show me all Umami Finance vaults with APR and TVL"

1. [链下] 对每个 vault 调用 `totalAssets()` 读取 TVL
2. [链下] 调用 `convertToAssets(1e18)` 计算 pricePerShare
3. [链下] 调用 `asset()` 和 `decimals()` 获取 vault metadata
4. 格式化输出：vault 名称、底层资产、TVL、share 价格

### 场景 2：存入 USDC 到 GM USDC ETH vault

用户说："Deposit 10 USDC into Umami gmUSDC ETH vault on Arbitrum"

1. [链下] 调用 `previewDeposit(10000000)` 预算可获得份额
2. [链下] 调用 `maxDeposit(wallet)` 验证额度充足
3. [链上] ERC-20 approve: ask user to confirm, then `onchainos wallet contract-call --chain 42161 --to 0xaf88d065e77c8cc2239327c5edb3a432268e5831 --input-data 0x095ea7b3{vault_padded}{amount_hex}`
4. [链上] deposit: ask user to confirm, then `onchainos wallet contract-call --chain 42161 --to 0x959f3807f0Aa7921E18c78B00B2819ba91E52FeF --input-data 0x6e553f65{amount_hex}{receiver_hex}`
5. 返回 txHash，显示获得的 gmUSDC 份额

### 场景 3：查看用户持仓并赎回

用户说："Show my Umami positions and withdraw from gmWETH vault"

1. [链下] 对每个 vault 调用 `balanceOf(wallet)` 查询份额
2. [链下] 调用 `convertToAssets(balance)` 计算资产价值
3. [链下] 调用 `previewRedeem(shares)` 预算可取回资产量
4. [链上] redeem: ask user to confirm, then `onchainos wallet contract-call --chain 42161 --to <VAULT_ADDR> --input-data 0xba087652{shares_hex}{receiver_hex}{owner_hex}`
5. 返回 txHash 和取回的资产金额

### 场景 4：查询特定 vault 详情

用户说："What's the current APR and capacity of Umami USDC vault?"

1. [链下] 调用 `totalAssets()` 获取 TVL
2. [链下] 调用 `maxDeposit(wallet)` 获取剩余容量
3. [链下] 调用 `convertToAssets(1e6)` 获取 USDC vault 的 pricePerShare（USDC decimals=6）
4. 格式化输出 vault 状态

---

## §4 外部 API 依赖

| 依赖 | 类型 | URL | 用途 |
|------|------|-----|------|
| Arbitrum RPC | JSON-RPC | `https://arb1.arbitrum.io/rpc` | eth_call 读取链上数据 |

无需外部 REST API — 所有数据通过 on-chain eth_call 获取。

---

## §5 配置参数

| 参数 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `--chain` | u64 | 42161 | 链 ID（目前仅支持 Arbitrum 42161） |
| `--vault` | string | — | vault 名称或地址（gmUSDC-eth / gmUSDC-btc / gmWETH / gmWBTC） |
| `--amount` | f64 | — | 存入/赎回数量（人类可读单位，如 10.0 USDC） |
| `--from` | string | — | 调用方钱包地址（不填则从 onchainos 解析） |
| `--dry-run` | bool | false | 干跑模式：构建 calldata 但不广播 |

---

## §6 合约函数 Selector 验证清单

所有 selector 已通过 `cast sig` 验证：

| 函数签名 | Selector | 验证状态 |
|---------|---------|---------|
| `deposit(uint256,uint256,address)` | `0x8dbdbe6d` | ✅ cast sig + live tx analysis |
| `redeem(uint256,uint256,address,address)` | `0x0169a996` | ✅ cast sig + live tx analysis |
| `withdraw(uint256,address,address)` | `0xb460af94` | ✅ cast sig (standard ERC-4626, not used) |
| `totalAssets()` | `0x01e1d114` | ✅ cast sig + eth_call |
| `totalSupply()` | `0x18160ddd` | ✅ cast sig |
| `convertToAssets(uint256)` | `0x07a2d13a` | ✅ cast sig + eth_call |
| `convertToShares(uint256)` | `0xc6e6f592` | ✅ cast sig |
| `previewDeposit(uint256)` | `0xef8b30f7` | ✅ cast sig + eth_call |
| `previewRedeem(uint256)` | `0x4cdad506` | ✅ cast sig + eth_call |
| `maxDeposit(address)` | `0x402d267d` | ✅ cast sig + eth_call |
| `maxWithdraw(address)` | `0xce96cb77` | ✅ cast sig |
| `asset()` | `0x38d52e0f` | ✅ cast sig + eth_call |
| `decimals()` | `0x313ce567` | ✅ cast sig + eth_call |
| `balanceOf(address)` | `0x70a08231` | ✅ cast sig |
| `approve(address,uint256)` | `0x095ea7b3` | ✅ standard ERC-20 |
