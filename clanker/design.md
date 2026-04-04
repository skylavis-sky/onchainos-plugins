# Clanker Plugin — design.md

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| `plugin_name` | `clanker` |
| `dapp_name` | Clanker |
| `version` | 0.1.0 |
| `target_chains` | Base (8453), Arbitrum One (42161) |
| `target_protocols` | Token deployment (ERC-20), Uniswap V4 LP |
| `category` | defi-protocol |
| `tags` | token-launch, meme, erc20, uniswap-v4, base |
| `binary_name` | clanker |
| `source_repo` | skylavis-sky/onchainos-plugins |
| `source_dir` | clanker |
| `priority_rank` | 11 |

---

## §1 接入可行性调研表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **No.** Clanker only ships a TypeScript SDK (`clanker-sdk` on npm, [github.com/clanker-devco/clanker-sdk](https://github.com/clanker-devco/clanker-sdk)). No Rust crate exists. |
| SDK 支持哪些技术栈？ | TypeScript / Node.js only. Viem-based direct contract interaction. |
| 有 REST API？ | **Yes.** `https://www.clanker.world/api` — Public (no auth) and Authenticated (partner API key via `x-api-key` header). Docs: [clanker.gitbook.io/clanker-documentation](https://clanker.gitbook.io/clanker-documentation). |
| 有官方 Skill？ | **Yes.** Official Clanker Agent Skill at `https://www.clanker.world/skill/skill.md` (referenced in docs as "add clanker skill"). Covers deploy, rewards, vault, airdrop, metadata updates. |
| 开源社区有类似 Skill？ | No dedicated onchainos community skill found for Clanker. The official skill.md is the best reference. |
| 支持哪些链？ | Base (8453, default), Arbitrum One (42161), Unichain (130), Ethereum Mainnet (1). OnchainOS integration targets **Base (8453)** and **Arbitrum One (42161)** only. |
| 是否需要 onchainos 广播？ | **Yes.** Token deployment is an on-chain write operation — must go through `onchainos wallet contract-call`. Reward claiming is also on-chain. Read operations (token search, list) are off-chain REST API calls. |

**接入路径：** `API` — No Rust SDK; the plugin calls the Clanker REST API for read/query operations and uses `onchainos wallet contract-call` for on-chain write operations (deploy token, claim rewards). For the deploy operation the plugin can use **either** the REST API (authenticated, enqueues on-chain tx server-side) or direct on-chain contract call. We use the REST API path for deploy (simpler, handles pool setup), and direct contract calls for claim-rewards and vault-withdraw.

---

## §2 接口映射

### 2a. 需要接入的操作表

| 操作 | 类型 | 说明 |
|------|------|------|
| `list-tokens` | 链下查询 | 列出最新部署的 Clanker 代币（分页、按链过滤） |
| `search-tokens` | 链下查询 | 按创建者地址/Farcaster 用户名搜索代币 |
| `token-info` | 链下查询 | 查询单个代币的详细信息（名称、符号、合约地址、市值等） |
| `deploy-token` | 链上操作 | 通过 REST API 部署新 ERC-20 代币（含 Uniswap V4 LP） |
| `claim-rewards` | 链上操作 | 认领创作者 LP 手续费奖励 |

### 2b. 链下查询表

#### `list-tokens`

- **Endpoint:** `GET https://clanker.world/api/tokens`
- **Auth:** None (public)
- **Parameters:**

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `page` | integer | No | 1 | 页码 |
| `limit` | integer | No | 20 | 每页数量（最大 50） |
| `sort` | string | No | `desc` | 排序方向 `asc`/`desc` |
| `chain_id` | integer | No | — | 按链过滤（8453 = Base, 42161 = Arbitrum） |

- **Response 关键字段:**

| 字段 | 类型 | 说明 |
|------|------|------|
| `tokens[].contract_address` | string | 代币合约地址 |
| `tokens[].name` | string | 代币名称 |
| `tokens[].symbol` | string | 代币符号 |
| `tokens[].chain_id` | integer | 链 ID |
| `tokens[].deployed_at` | string | 部署时间 (ISO 8601) |
| `tokens[].img_url` | string | 代币 logo URL |
| `total` | integer | 总记录数 |
| `hasMore` | boolean | 是否有更多页 |

---

#### `search-tokens`

- **Endpoint:** `GET https://clanker.world/api/search-creator`
- **Auth:** None (public)
- **Parameters:**

| 参数名 | 类型 | 必填 | 默认值 | 说明 |
|--------|------|------|--------|------|
| `q` | string | Yes | — | Farcaster 用户名 或 钱包地址 |
| `limit` | integer | No | 20 | 最大 50 |
| `offset` | integer | No | 0 | 偏移量 |
| `sort` | string | No | `desc` | `asc`/`desc` |
| `trustedOnly` | boolean | No | false | 只返回可信部署者 |

- **Response 关键字段:**

| 字段 | 类型 | 说明 |
|------|------|------|
| `tokens[].contract_address` | string | 代币合约地址 |
| `tokens[].name` | string | 代币名称 |
| `tokens[].symbol` | string | 代币符号 |
| `tokens[].chain_id` | integer | 链 ID |
| `tokens[].deployed_at` | string | 部署时间 |
| `tokens[].trustStatus.isTrustedClanker` | boolean | 是否为可信 Clanker |
| `user` | object | Farcaster 用户信息 |
| `searchedAddress` | string | 查询使用的钱包地址 |
| `total` | integer | 总记录数 |

---

#### `token-info`

Use `onchainos token info` + `onchainos token price-info` from OnchainOS built-in skills, supplemented by on-chain lookup via `eth_call` on the token contract to read `name()`, `symbol()`, `decimals()`, `totalSupply()`.

Alternatively call Clanker's search-creator endpoint if a creator address is known. No dedicated single-token-by-address public REST endpoint is documented; use OnchainOS token info.

- **onchainos commands used:**
  - `onchainos token info --address <contract_address> --chain <chain_id>`
  - `onchainos token price-info --address <contract_address> --chain <chain_id>`

---

### 2c. 链上写操作表

#### `deploy-token`

**接入路径：REST API（发起链上部署）**

The Clanker REST API (`POST /api/tokens/deploy`) accepts deployment parameters and enqueues a server-side on-chain transaction through Clanker's own deployer wallet. The response returns `expectedAddress`. **No direct user wallet transaction is submitted for deployment via this path** — the API handles the on-chain tx.

However, the user should be aware that:
1. An API key (`x-api-key`) is required — partner key issued by Clanker team.
2. After deployment, token ownership and admin rights are transferred to `tokenAdmin` (user's wallet).

**REST API Call (Rust `reqwest`):**

```
POST https://www.clanker.world/api/tokens/deploy
Headers:
  Content-Type: application/json
  x-api-key: <CLANKER_API_KEY>

Body (minimum viable):
{
  "token": {
    "name": "<NAME>",
    "symbol": "<SYMBOL>",
    "tokenAdmin": "<USER_WALLET_ADDRESS>",
    "requestKey": "<32-char UUID>"
  },
  "rewards": [
    {
      "admin": "<USER_WALLET_ADDRESS>",
      "recipient": "<USER_WALLET_ADDRESS>",
      "allocation": 100
    }
  ]
}
```

**Response:**
```json
{
  "message": "Token deployment enqueued. Expected address: 0x...",
  "expectedAddress": "0x...",
  "success": true
}
```

**Optional fields for full deployment:**

| Field | Type | Description |
|-------|------|-------------|
| `token.image` | string | IPFS or HTTPS image URL |
| `token.description` | string | Token description |
| `token.socialMediaUrls` | array | `[{"platform":"twitter","url":"..."}]` |
| `pool.pairedToken` | string | Quote token address (default: WETH on Base) |
| `pool.initialMarketCap` | number | Starting market cap in paired token |
| `pool.type` | string | `"standard"` or `"project"` |
| `fees.type` | string | `"static"` or `"dynamic"` |
| `fees.clankerFee` | number | Max 5% (static mode) |
| `vault.percentage` | number | 0–90% of supply locked |
| `vault.lockupDuration` | number | Days, minimum 7 |
| `vault.vestingDuration` | number | Days for linear vesting |
| `chainId` | number | `8453` (Base) or `42161` (Arbitrum) |

**Default chain:** Base (8453).

---

#### `claim-rewards`

**接入路径：直接合约调用**

Creator LP fee rewards are held in `ClankerFeeLocker` (Base: `0xF3622742b1E446D92e45E22923Ef11C2fcD55D68`). The reward recipient must call `collectFees(address token)` or equivalent on the locker contract.

**Note:** The exact locker contract address should be resolved at runtime via the Clanker API or on-chain registry — do NOT hardcode as v4.x upgrades may deploy new lockers. Query `https://clanker.world/api/tokens/<contract_address>` to get the token's associated locker contract, or read the `feeLocker` field returned during deployment.

**Calldata construction for `collectFees(address token)`:**

```
Function selector: keccak256("collectFees(address)")[0..4]
  = 0x[compute at runtime using alloy-sol-types]

ABI encoding:
  bytes4(selector) ++ abi.encode(token_address_as_address32)

Example (token addr = 0xAbCd...1234):
  0x<4-byte-selector>
  000000000000000000000000AbCd...1234
```

**onchainos command:**
```bash
onchainos wallet contract-call \
  --chain 8453 \
  --to <FEE_LOCKER_ADDRESS> \
  --input-data <HEX_CALLDATA> \
  --from <RECIPIENT_ADDRESS> \
  --force
```

**Pre-flight:** Resolve the fee locker contract address for the specific token at runtime. If the token was deployed via the API, store the locker address from the deployment response or fetch from on-chain by calling the Clanker factory's view function `feeLockerForToken(address token)`.

**Contract address resolution:**
- Factory (Base v4.0): `0xE85A59c628F7d27878ACeB4bf3b35733630083a9`
- FeeLocker (Base v4.0): `0xF3622742b1E446D92e45E22923Ef11C2fcD55D68`
- Factory (Arbitrum v4.0): resolve from `https://clanker.gitbook.io/clanker-documentation/references/deployed-contracts` at integration time and embed as constants keyed by chain ID.
- For v4.1.x and future versions: always resolve at runtime via on-chain registry or API.

**User confirmation required before submitting the `contract-call`.**

---

## §3 用户场景

### 场景 1：部署新代币（核心 happy path）

**用户说：**
> "Help me deploy a new token called 'SkyDog' with symbol 'SKYDOG' on Base. Use my wallet as the admin and set my wallet as the reward recipient."

**Agent 动作序列：**

1. **[链下查询]** `onchainos wallet status` — 确认用户已登录
2. **[链下查询]** `onchainos wallet addresses` — 获取用户 EVM 钱包地址（`data.evm[0].address`）
3. **[链下查询]** `onchainos wallet balance --chain 8453` — 确认 Base 链上有足够 ETH 支付 gas（部署本身由 Clanker 服务器执行，但用户需要 ETH 用于后续操作如 claim）
4. **[确认步骤]** 向用户展示部署参数：名称 = "SkyDog"，符号 = "SKYDOG"，链 = Base (8453)，tokenAdmin = 用户地址，奖励接收者 = 用户地址。**请求用户确认。**
5. **[链下操作]** 生成 32 字符唯一 `requestKey`（UUID v4，去连字符）
6. **[链下操作]** 调用 REST API: `POST https://www.clanker.world/api/tokens/deploy` with:
   ```json
   {
     "token": { "name": "SkyDog", "symbol": "SKYDOG", "tokenAdmin": "<wallet>", "requestKey": "<uuid>" },
     "rewards": [{ "admin": "<wallet>", "recipient": "<wallet>", "allocation": 100 }],
     "chainId": 8453
   }
   ```
7. **[结果]** 解析响应中的 `expectedAddress` — 向用户展示预期合约地址
8. **[链下查询]** 等待约 30 秒后调用 `onchainos token info --address <expectedAddress> --chain 8453` 确认代币已上链

---

### 场景 2：查询某创建者部署的代币

**用户说：**
> "Show me all tokens deployed by wallet address 0xabc123...def456 on Base."

**Agent 动作序列：**

1. **[链下查询]** 调用 REST API: `GET https://clanker.world/api/search-creator?q=0xabc123...def456&limit=20&sort=desc`
2. **[链下查询]** 对返回的每个代币，获取价格信息（可选，如用户请求）: `onchainos token price-info --address <contract_address> --chain 8453`
3. **[结果]** 向用户展示代币列表，包括：名称、符号、合约地址、部署时间、trust 状态

---

### 场景 3：认领 LP 奖励（含风控检查）

**用户说：**
> "Claim the LP fee rewards for my Clanker token at 0xTokenAddress on Base."

**Agent 动作序列：**

1. **[链下查询]** `onchainos wallet status` — 确认已登录
2. **[链下查询]** `onchainos wallet addresses` — 获取用户钱包地址
3. **[链下查询]** `onchainos security token-scan --address 0xTokenAddress --chain 8453` — 确认代币不是蜜罐；如 scan 失败则终止
4. **[链下查询]** 通过 `eth_call` 查询 `ClankerFeeLocker.pendingRewards(address recipient, address token)` — 确认有可认领的奖励金额（如为 0，提示用户"无可认领奖励"）
5. **[链下查询]** 确认 fee locker 合约地址：调用 factory `0xE85A59c628F7d27878ACeB4bf3b35733630083a9` 的 view 函数 `feeLockerForToken(address)` 以动态解析（防止硬编码版本漂移）
6. **[确认步骤]** 构造 `collectFees(address)` calldata；向用户展示：目标合约 = FeeLocker 地址，方法 = collectFees，参数 = 代币地址。**请求用户确认。**
7. **[链上操作]** 用户确认后，执行:
   ```bash
   onchainos wallet contract-call \
     --chain 8453 \
     --to <FEE_LOCKER_ADDRESS> \
     --input-data <HEX_CALLDATA> \
     --from <USER_WALLET> \
     --force
   ```
8. **[结果]** 从响应 `.data.txHash` 提取交易哈希，向用户展示确认信息

---

### 场景 4：列出最新发布的代币（查询类）

**用户说：**
> "What are the latest tokens launched on Clanker today?"

**Agent 动作序列：**

1. **[链下查询]** 调用 REST API: `GET https://clanker.world/api/tokens?limit=10&sort=desc&chain_id=8453`
2. **[链下查询]** 对热门代币可选调用 `onchainos token price-info` 获取实时价格
3. **[结果]** 展示最新 10 个代币列表（名称、符号、合约地址、部署时间）

---

## §4 外部 API 依赖

| API | Base URL | Auth | 用途 |
|-----|----------|------|------|
| Clanker REST API (Public) | `https://clanker.world/api` | None | 代币列表、创建者搜索 |
| Clanker REST API (Authenticated) | `https://clanker.world/api` | `x-api-key` header | 代币部署 |
| Base RPC | `https://base-rpc.publicnode.com` | None | 链上 eth_call（fee locker 查询、代币信息） |
| Arbitrum RPC | `https://arb1.arbitrum.io/rpc` | None | Arbitrum 链上 eth_call |
| onchainos token info | (built-in) | — | 代币元数据 |
| onchainos token price-info | (built-in) | — | 代币价格 |
| onchainos security token-scan | (built-in) | — | 安全检查 |

---

## §5 配置参数

| 参数名 | 类型 | 默认值 | 说明 |
|--------|------|--------|------|
| `clanker_api_key` | string | `""` | Clanker Partner API Key（部署代币必填，由 Clanker 团队发放） |
| `default_chain_id` | u64 | `8453` | 默认链（Base），支持 8453 / 42161 |
| `base_rpc_url` | string | `"https://base-rpc.publicnode.com"` | Base 链 RPC 端点 |
| `arbitrum_rpc_url` | string | `"https://arb1.arbitrum.io/rpc"` | Arbitrum 链 RPC 端点 |
| `default_token_list_limit` | u32 | `20` | 列表查询默认每页条数 |
| `dry_run` | bool | `false` | 为 true 时跳过所有链上写操作，仅输出将要执行的命令 |

**Note on `dry_run`:** The `--dry-run` flag is NOT passed to `onchainos wallet contract-call` (it does not accept that flag). Instead, `dry_run = true` causes the plugin to short-circuit before issuing the contract-call and print the calldata that would have been submitted.

---

## §6 合约地址参考

> **NEVER hardcode these in logic paths. Resolve at runtime or embed as chain-keyed constants with version annotation. Update when Clanker releases new contract versions.**

### Base (8453) — v4.0.0

| Contract | Address |
|----------|---------|
| Clanker (factory) | `0xE85A59c628F7d27878ACeB4bf3b35733630083a9` |
| ClankerFeeLocker | `0xF3622742b1E446D92e45E22923Ef11C2fcD55D68` |
| ClankerLpLocker | `0x29d17C1A8D851d7d4cA97FAe97AcAdb398D9cCE0` |
| ClankerVault | `0x8E845EAd15737bF71904A30BdDD3aEE76d6ADF6C` |
| ClankerHookDynamicFee | `0x34a45c6B61876d739400Bd71228CbcbD4F53E8cC` |
| ClankerHookStaticFee | `0xDd5EeaFf7BD481AD55Db083062b13a3cdf0A68CC` |

### Base (8453) — v4.1.0 (additional)

| Contract | Address |
|----------|---------|
| ClankerHookDynamicFeeV2 | `0xd60D6B218116cFd801E28F78d011a203D2b068Cc` |
| ClankerHookStaticFeeV2 | `0xb429d62f8f3bFFb98CdB9569533eA23bF0Ba28CC` |
| ClankerAirdropV2 | `0xf652B3610D75D81871bf96DB50825d9af28391E0` |

**Source:** [clanker.gitbook.io/clanker-documentation/references/deployed-contracts](https://clanker.gitbook.io/clanker-documentation/references/deployed-contracts)

---

## §7 技术说明 & 开发注意事项

### 接入路径选择理由

- **Deploy:** REST API path (`POST /api/tokens/deploy`) is preferred over direct contract call. The deployment requires complex ABI-encoding of nested structs (`DeploymentConfig`, `TokenConfig`, `PoolConfig`, `LockerConfig`, `MevModuleConfig`, `ExtensionConfig[]`) and setting up a Uniswap V4 pool in a single transaction. The REST API abstracts this complexity. Requires a partner API key.
- **Claim Rewards:** Direct contract call via `onchainos wallet contract-call`. Simpler ABI (`collectFees(address)`), and gives user full custody/control.
- **Read Ops:** REST API (public, no auth).

### Dry-Run Handling

```rust
if config.dry_run {
    println!("DRY RUN: would call onchainos wallet contract-call \
        --chain {} --to {} --input-data {} --from {} --force",
        chain_id, contract_addr, calldata_hex, wallet_addr);
    return Ok(());
}
// only here do we actually invoke onchainos
```

### 安全要求

- Always run `onchainos security token-scan` before `claim-rewards` on a token address.
- Always require user confirmation before any `wallet contract-call`.
- Validate `expectedAddress` from deploy API before displaying — should be a valid `0x`-prefixed 42-char hex address.
- The `requestKey` in deploy requests must be unique per call (UUID v4 without hyphens). Store and deduplicate to prevent accidental double-deployment.

### Fee Locker Address Resolution

The fee locker contract may differ between token versions. Resolve dynamically:
1. On deploy: capture the `feeLockerAddress` from the deploy API response (if returned).
2. On claim: call factory view `feeLockerForToken(address token) -> address` on the v4.0 factory. If the token predates v4.0 or is on a different factory version, fall back to the known static address for that version.

### 链支持范围

- **Base (8453):** Full support — deploy, list, search, claim-rewards.
- **Arbitrum One (42161):** Support deploy and claim-rewards. List/search REST API supports `chain_id=42161` filter.
- **Unichain (130), Ethereum (1):** Clanker supports these but OnchainOS integration does NOT include them in this plugin version.

---

## §8 参考资料

- Clanker Documentation: https://clanker.gitbook.io/clanker-documentation
- Deploy Token API: https://clanker.gitbook.io/clanker-documentation/authenticated/deploy-token-v4.0.0
- Public API: https://clanker.gitbook.io/clanker-documentation/public
- TypeScript SDK: https://github.com/clanker-devco/clanker-sdk
- v4 Contracts GitHub: https://github.com/clanker-devco/v4-contracts
- Deployed Contracts: https://clanker.gitbook.io/clanker-documentation/references/deployed-contracts
- Official Agent Skill: https://www.clanker.world/skill/skill.md
- BaseScan Factory: https://basescan.org/address/0xe85a59c628f7d27878aceb4bf3b35733630083a9
