# Aerodrome AMM — Plugin Store 接入 PRD

> 通过 onchainos CLI 接入 Aerodrome Finance 经典 AMM 池（volatile + stable），使 AI Agent 能完成链上 swap、流动性管理、查询报价、领取 gauge 奖励等核心操作。

---

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | `aerodrome-amm` |
| dapp_name | Aerodrome AMM |
| dapp_repo | https://github.com/aerodrome-finance/contracts |
| dapp_alias | Aerodrome Classic AMM, Aerodrome V2, Velodrome V2 style AMM |
| one_liner | Classic AMM DEX on Base — swap tokens and manage volatile/stable LP positions with gauge AERO rewards |
| category | defi-protocol |
| tags | dex, amm, aerodrome, classic-amm, stable, volatile, base |
| target_chains | EVM (Base, chain ID 8453) |
| target_protocols | Aerodrome Finance Classic AMM (volatile + stable pools) |

**重要区别：** 本插件为 Aerodrome Finance **经典 AMM 池**（volatile/stable），使用 `bool stable` 标识池类型，与 Aerodrome Slipstream（CLMM，使用 `tickSpacing`）是不同的合约系统。

---

## §1 接入可行性调研

### 这个 DApp 是什么

Aerodrome Finance 是 Base 链最大的 DEX 和流动性市场，由 Velodrome Finance（Optimism）派生而来。经典 AMM 模块（本插件目标）使用 Uniswap V2 / Curve 混合风格的常数积公式：
- **Volatile pools**: `x³y + y³x = k`（类 UniV2 常数积）
- **Stable pools**: `x³y + y³x = k`（低滑点稳定币曲线）

池子类型由 `bool stable` 标志区分，不使用 fee tier 或 tickSpacing。

与 Slipstream（CLMM）的关键差异：
- Router 合约不同（`0xcF77a3Ba...` vs `0xBE6D8f0d...`）
- 流动性 token 是 ERC-20（LP token），不是 ERC-721 NFT
- 路由参数使用 `Route[] { from, to, stable, factory }` 结构体数组
- 支持 gauge 质押 LP token 获取 AERO 排放

### 接入可行性表

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | No — 无官方 Rust SDK |
| SDK 支持哪些技术栈？ | 无官方 SDK；TypeScript 社区工具；无 Rust 支持 |
| 有 REST API？ | No — 仅链上合约调用 |
| 有官方 Skill？ | No |
| 开源社区有类似 Skill？ | `aerodrome-finance-mcp` 社区参考（TypeScript） |
| 支持哪些链？ | Base (8453) |
| 是否需要 onchainos 广播？ | Yes — swap、approve、addLiquidity、removeLiquidity、claimRewards 全部需要 |
| 合约是否开源验证？ | Yes — BaseScan 已验证，GitHub 全开源 |

### 接入路径判定

无 SDK，无 REST API。合约为 Velodrome V2 style，可直接手动 ABI 编码调用。接入路径：**直接 ABI 编码 + onchainos wallet contract-call**。

---

## §2 接口映射表

### 合约地址（Base 主网，chain ID 8453）

| 合约 | 地址 | 用途 |
|------|------|------|
| Router (Classic AMM) | `0xcF77a3Ba9A5CA399B7c97c74d54e5b1Beb874E43` | swap / addLiquidity / removeLiquidity |
| PoolFactory | `0x420DD381b31aEf6683db6B902084cB0FFECe40Da` | pool 地址查询、allPools 枚举 |
| Voter | `0x16613524E02ad97eDfeF371bC883F2F5d6C480A5` | gauges(pool) → gauge 地址 |
| AERO Token | `0x940181a94A35A4569E4529A3CDfB74e38FD98631` | AERO 排放代币 |
| WETH (Base) | `0x4200000000000000000000000000000000000006` | 原生 ETH wrap |

> **注意：** Classic AMM Router (`0xcF77a3Ba...`) 与 Slipstream CL Router (`0xBE6D8f0d...`) 是不同合约，不可混用。

### Route Struct

Aerodrome Classic AMM Router 使用 `Route` 结构体数组作为路径参数：

```solidity
struct Route {
    address from;
    address to;
    bool stable;
    address factory;
}
```

ABI 编码（每个 Route = 4 × 32 字节 = 128 字节）：
- `from`: 32 字节地址
- `to`: 32 字节地址
- `stable`: 32 字节 bool（0=volatile, 1=stable）
- `factory`: 32 字节地址（使用 PoolFactory 地址）

### Function Selectors（已通过 keccak-256 验证）

#### Router 函数

| 函数签名 | Selector | 合约 |
|---------|---------|------|
| `swapExactTokensForTokens(uint256,uint256,(address,address,bool,address)[],address,uint256)` | `0xcac88ea9` | Router |
| `swapExactETHForTokens(uint256,(address,address,bool,address)[],address,uint256)` | `0x903638a4` | Router (payable) |
| `swapExactTokensForETH(uint256,uint256,(address,address,bool,address)[],address,uint256)` | `0xc6b7f1b6` | Router |
| `addLiquidity(address,address,bool,uint256,uint256,uint256,uint256,address,uint256)` | `0x5a47ddc3` | Router |
| `addLiquidityETH(address,bool,uint256,uint256,uint256,address,uint256)` | `0xb7e0d4c0` | Router (payable) |
| `removeLiquidity(address,address,bool,uint256,uint256,uint256,address,uint256)` | `0x0dede6c4` | Router |
| `removeLiquidityETH(address,bool,uint256,uint256,uint256,address,uint256)` | `0xd7b0e0a5` | Router |
| `getAmountsOut(uint256,(address,address,bool,address)[])` | `0x5509a1ac` | Router (view) |
| `quoteAddLiquidity(address,address,bool,address,uint256,uint256)` | `0xce700c29` | Router (view) |
| `quoteRemoveLiquidity(address,address,bool,address,uint256)` | `0xc92de3ec` | Router (view) |
| `poolFor(address,address,bool,address)` | `0x874029d9` | Router (view) |

#### PoolFactory 函数

| 函数签名 | Selector | 合约 |
|---------|---------|------|
| `allPools(uint256)` | `0x41d1de97` | PoolFactory (view) |
| `allPoolsLength()` | `0xefde4e64` | PoolFactory (view) |
| `getPool(address,address,bool)` | `0x79bc57d5` | PoolFactory (view) |

#### Pool 函数（ERC-20 LP Token）

| 函数签名 | Selector | 合约 |
|---------|---------|------|
| `token0()` | `0x0dfe1681` | Pool (view) |
| `token1()` | `0xd21220a7` | Pool (view) |
| `stable()` | `0x22be3de1` | Pool (view) |
| `getReserves()` | `0x0902f1ac` | Pool (view) |
| `balanceOf(address)` | `0x70a08231` | Pool / ERC-20 (view) |
| `totalSupply()` | `0x18160ddd` | Pool / ERC-20 (view) |
| `approve(address,uint256)` | `0x095ea7b3` | ERC-20 |
| `allowance(address,address)` | `0xdd62ed3e` | ERC-20 (view) |

#### Voter / Gauge 函数

| 函数签名 | Selector | 合约 |
|---------|---------|------|
| `gauges(address)` | `0xb9a09fd5` | Voter (view) |
| `getReward(address)` | `0xc00007b0` | Gauge (write) |
| `earned(address)` | `0x008cc262` | Gauge (view) |
| `deposit(uint256)` | `0xb6b55f25` | Gauge (write) |
| `withdraw(uint256)` | `0x2e1a7d4d` | Gauge (write) |

### 操作清单

| # | 操作 | CLI 命令 | 说明 | 链上/链下 |
|---|------|----------|------|-----------|
| 1 | `quote` | `aerodrome-amm quote` | getAmountsOut via Router eth_call | 链下 |
| 2 | `swap` | `aerodrome-amm swap` | swapExactTokensForTokens via Router | 链上 |
| 3 | `pools` | `aerodrome-amm pools` | PoolFactory.getPool() | 链下 |
| 4 | `positions` | `aerodrome-amm positions` | LP token balanceOf / pool info | 链下 |
| 5 | `add-liquidity` | `aerodrome-amm add-liquidity` | approve x2 + addLiquidity via Router | 链上 |
| 6 | `remove-liquidity` | `aerodrome-amm remove-liquidity` | approve LP + removeLiquidity via Router | 链上 |
| 7 | `claim-rewards` | `aerodrome-amm claim-rewards` | Gauge.getReward(wallet) | 链上 |

---

## §3 链下查询（eth_call via RPC）

| 操作 | 合约 | 函数 | 关键参数 | 返回值 |
|------|------|------|---------|--------|
| `quote` | Router | `getAmountsOut` | amountIn, routes[] | uint256[] amounts |
| `pools` | PoolFactory | `getPool` | tokenA, tokenB, stable | pool address |
| `pools/list` | PoolFactory | `allPoolsLength` + `allPools` | index | pool addresses |
| `positions` | Pool | `balanceOf`, `getReserves`, `totalSupply` | wallet, pool | LP balance, reserves |
| check earned | Gauge | `earned` | wallet address | uint256 pending |
| get gauge | Voter | `gauges` | pool address | gauge address |
| allowance | ERC-20 | `allowance` | owner, spender | uint256 |
| balance | ERC-20 | `balanceOf` | owner | uint256 |

---

## §4 链上写操作 calldata 构造

### ERC-20 Approve

```
selector: 0x095ea7b3
calldata: 0x095ea7b3 + pad32(spender) + pad32(amount)
onchainos wallet contract-call --chain 8453 --to <TOKEN> --input-data <calldata> --force
```

### Swap (swapExactTokensForTokens)

selector: `0xcac88ea9`

```
calldata: 0xcac88ea9
  + pad32(amountIn)
  + pad32(amountOutMin)
  + offset to routes array (= 0xa0 = 160 bytes for 5 static params)
  + pad32(to / recipient)
  + pad32(deadline)
  + pad32(routes.length)
  + [for each route: pad32(from) + pad32(to) + pad32(stable) + pad32(factory)]
```

**注意：** Route[] 是动态数组，ABI encoding 使用 offset + length + data 格式。

### Add Liquidity (addLiquidity)

selector: `0x5a47ddc3`

```
calldata: 0x5a47ddc3
  + pad32(tokenA)
  + pad32(tokenB)
  + pad32(stable)         ← bool: 0 or 1
  + pad32(amountADesired)
  + pad32(amountBDesired)
  + pad32(amountAMin)
  + pad32(amountBMin)
  + pad32(to)
  + pad32(deadline)
```

Returns: `(uint256 amountA, uint256 amountB, uint256 liquidity)`

### Remove Liquidity (removeLiquidity)

selector: `0x0dede6c4`

Step 1: approve LP token → Router
Step 2: removeLiquidity

```
calldata: 0x0dede6c4
  + pad32(tokenA)
  + pad32(tokenB)
  + pad32(stable)
  + pad32(liquidity)      ← LP token amount to burn
  + pad32(amountAMin)
  + pad32(amountBMin)
  + pad32(to)
  + pad32(deadline)
```

Returns: `(uint256 amountA, uint256 amountB)`

### Claim Rewards (getReward)

selector: `0xc00007b0`

```
calldata: 0xc00007b0 + pad32(account)
onchainos wallet contract-call --chain 8453 --to <GAUGE> --input-data <calldata> --force
```

---

## §5 用户场景

### 场景 1：Token Swap

**用户说：**「在 Base 上用 0.00005 ETH 换 USDC，走 Aerodrome AMM volatile 池」

**Agent 动作序列：**
1. 解析参数：tokenIn=WETH, tokenOut=USDC, amountIn=50000000000000 (0.00005e18), stable=false
2. 链下查询：PoolFactory `getPool(WETH, USDC, false)` 确认 volatile 池已部署
3. 链下查询：Router `getAmountsOut(amountIn, [{WETH, USDC, false, factory}])` → amountOut
4. 计算 amountOutMin = amountOut × (1 - slippage/100)
5. 解析钱包地址
6. 链下查询：检查 WETH allowance → Router（WETH 是 ERC-20，需要 approve）
7. 向用户展示报价，**请用户确认**后执行
8. 链上操作（如需）：approve WETH → Router (--force)；等待 3 秒
9. 链上操作：swapExactTokensForTokens(amountIn, amountOutMin, routes, to, deadline) (--force)
10. 返回 txHash 和预期 USDC 输出量

### 场景 2：添加 volatile 流动性

**用户说：**「向 WETH/USDC volatile 池添加流动性，投入 0.0001 WETH」

**Agent 动作序列：**
1. 解析参数：tokenA=WETH, tokenB=USDC, stable=false
2. 链下查询：`getPool(WETH, USDC, false)` 确认池已部署
3. 链下查询：`quoteAddLiquidity(WETH, USDC, false, factory, amountADesired, MAX)` → 估算所需 USDC 数量
4. 链下查询：检查 WETH 和 USDC 余额
5. 向用户展示参数，**请用户确认**
6. 链上操作：approve WETH → Router (--force)；等待 5 秒
7. 链上操作：approve USDC → Router (--force)；等待 5 秒
8. 链上操作：addLiquidity(WETH, USDC, false, amountA, amountB, minA, minB, to, deadline) (--force)
9. 返回 txHash 和获得的 LP token 数量

### 场景 3：移除流动性

**用户说：**「移除我在 WETH/USDC volatile 池的全部流动性」

**Agent 动作序列：**
1. 解析参数：tokenA=WETH, tokenB=USDC, stable=false
2. 链下查询：`getPool(WETH, USDC, false)` → pool 地址
3. 链下查询：Pool `balanceOf(wallet)` → LP token 余额
4. 如果余额为 0，提示无流动性
5. 链下查询：`quoteRemoveLiquidity(WETH, USDC, false, factory, liquidity)` → 预期回收代币量
6. 向用户展示将回收的代币量，**请用户确认**
7. 链上操作：approve LP token → Router (--force)；等待 3 秒
8. 链上操作：removeLiquidity(WETH, USDC, false, liquidity, 0, 0, to, deadline) (--force)
9. 返回 txHash 和回收的 tokenA/tokenB 数量

### 场景 4：查询池子信息

**用户说：**「查询 Base 上 USDC/DAI stable 池的信息」

**Agent 动作序列：**
1. `getPool(USDC, DAI, true)` → pool 地址
2. Pool `getReserves()` → reserve0, reserve1
3. Pool `totalSupply()` → 总 LP
4. 格式化输出池子地址、代币、储备量、稳定/波动类型

### 场景 5：领取 Gauge 奖励

**用户说：**「领取我在 WETH/USDC volatile 池 gauge 的 AERO 奖励」

**Agent 动作序列：**
1. 链下查询：`getPool(WETH, USDC, false)` → pool 地址
2. 链下查询：Voter `gauges(pool)` → gauge 地址
3. 链下查询：Gauge `earned(wallet)` → 待领取 AERO 数量
4. 如果 earned=0，提示暂无奖励
5. 向用户展示待领取数量，**请用户确认**
6. 链上操作：Gauge `getReward(wallet)` (--force)
7. 返回 txHash

---

## §6 API 依赖

| API | Base URL | 用途 | 需要 API Key？ |
|-----|----------|------|---------------|
| Base RPC | `https://base-rpc.publicnode.com` | 链上读写（eth_call, eth_sendRawTransaction） | No |

---

## §7 配置参数

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `chain` | u64 | `8453` | 目标链 ID（仅 Base 8453） |
| `slippage` | f64 | `0.5` | 最大允许滑点（百分比） |
| `dry_run` | bool | `false` | 模拟模式，不广播交易 |
| `deadline_minutes` | u64 | `20` | 交易过期时间（分钟） |
| `stable` | bool | auto | 是否使用 stable 池（默认 volatile=false） |

---

## §8 开发注意事项

### Classic AMM vs Slipstream 关键差异

| 特性 | Classic AMM (本插件) | Aerodrome Slipstream |
|------|--------------------|----------------------|
| 池标识参数 | `bool stable` | `int24 tickSpacing` |
| Router | `0xcF77a3Ba...` | `0xBE6D8f0d...` |
| LP Token 类型 | ERC-20 | ERC-721 NFT |
| 路由参数 | `Route[] {from, to, stable, factory}` | 单 tick spacing 参数 |
| 流动性 NFT | 无 | 有 (`AERO-CL-POS`) |
| 奖励领取 | Gauge.getReward(wallet) | Gauge 质押 NFT 获取 |

### Route[] 动态数组 ABI 编码

```rust
// swapExactTokensForTokens 的 routes 参数是动态数组
// ABI encoding:
// offset (32 bytes) → points to length
// [static params: amountIn, amountOutMin, offset, to, deadline]
// then: length of routes array (32 bytes)
// then: [route0.from, route0.to, route0.stable, route0.factory] × n

fn encode_routes(routes: &[Route]) -> String {
    // static offset = 0xa0 (5 static words × 32 bytes = 160 bytes before routes offset)
    // BUT actually offset is relative to the start of the calldata params
    // For single arg position 2 (0-indexed): offset = 5 * 32 = 160 = 0xa0
    let mut data = String::new();
    data.push_str(&format!("{:0>64x}", routes.len())); // length
    for r in routes {
        data.push_str(&pad_address(&r.from));
        data.push_str(&pad_address(&r.to));
        data.push_str(&format!("{:0>64x}", r.stable as u64));
        data.push_str(&pad_address(&r.factory));
    }
    data
}
```

### approve → 链上操作之间的延迟

- approve → swap: 等待 3 秒
- approve tokenA → approve tokenB: 等待 5 秒
- approve → addLiquidity: 等待 5 秒（最后一次 approve 后）
- approve LP → removeLiquidity: 等待 3 秒

### 所有写操作必须带 --force

```bash
onchainos wallet contract-call --chain 8453 --to <ADDR> --input-data <HEX> --force
```

### resolve_wallet (dry_run guard)

```rust
let recipient = if dry_run {
    "0x0000000000000000000000000000000000000000".to_string()
} else {
    resolve_wallet(CHAIN_ID)?
};
```

### ETH swap 特殊处理

- **ETH → Token**: 使用 `swapExactETHForTokens` (payable)，`--amt <wei>` 传递 ETH 数量，无需 approve
- **Token → ETH**: 使用 `swapExactTokensForETH`，需要 approve token 到 Router
- **Token → Token**: 使用 `swapExactTokensForTokens`，需要 approve tokenIn 到 Router

### 常用代币地址（Base 主网）

| Symbol | Address |
|--------|---------|
| WETH / ETH | `0x4200000000000000000000000000000000000006` |
| USDC | `0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913` |
| CBBTC | `0xcbB7C0000aB88B473b1f5aFd9ef808440eed33Bf` |
| AERO | `0x940181a94A35A4569E4529A3CDfB74e38FD98631` |
| DAI | `0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb` |
| USDT | `0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2` |
| WSTETH | `0xc1CBa3fCea344f92D9239c08C0568f6F2F0ee452` |

---

## §9 Function Selector 验证记录

所有 selector 通过 `keccak256(signature).hex()[:8]` 验证：

```
0xcac88ea9  swapExactTokensForTokens(uint256,uint256,(address,address,bool,address)[],address,uint256)
0x5a47ddc3  addLiquidity(address,address,bool,uint256,uint256,uint256,uint256,address,uint256)
0x0dede6c4  removeLiquidity(address,address,bool,uint256,uint256,uint256,address,uint256)
0x5509a1ac  getAmountsOut(uint256,(address,address,bool,address)[])
0xce700c29  quoteAddLiquidity(address,address,bool,address,uint256,uint256)
0xc92de3ec  quoteRemoveLiquidity(address,address,bool,address,uint256)
0x0902f1ac  getReserves()
0xc00007b0  getReward(address)
0x41d1de97  allPools(uint256)
0xefde4e64  allPoolsLength()
0x79bc57d5  getPool(address,address,bool)
0x095ea7b3  approve(address,uint256)
0x903638a4  swapExactETHForTokens(uint256,(address,address,bool,address)[],address,uint256)
0xc6b7f1b6  swapExactTokensForETH(uint256,uint256,(address,address,bool,address)[],address,uint256)
0xb7e0d4c0  addLiquidityETH(address,bool,uint256,uint256,uint256,address,uint256)
0xd7b0e0a5  removeLiquidityETH(address,bool,uint256,uint256,uint256,address,uint256)
0x874029d9  poolFor(address,address,bool,address)
0x0dfe1681  token0()
0xd21220a7  token1()
0x22be3de1  stable()
0xb9a09fd5  gauges(address)
0x008cc262  earned(address)
0xb6b55f25  deposit(uint256)
0x2e1a7d4d  withdraw(uint256)
0x70a08231  balanceOf(address)
0xdd62ed3e  allowance(address,address)
0x18160ddd  totalSupply()
```
