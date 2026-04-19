# 测试结果报告 — uniswap-v3

- 日期: 2026-04-19
- DApp 支持的链: EVM only (Ethereum, Arbitrum, Base, Optimism, Polygon)
- EVM 测试链: Base (8453)
- 编译: ✅
- Lint: ✅
- 整体结论: PASS

---

## 汇总

| 总数 | L1编译 | L0路由 | L2读取 | L3模拟 | L4链上 | 失败 | 阻塞 |
|------|--------|--------|--------|--------|--------|------|------|
| 57   | 2/2    | 44/44  | 6/6    | 3/3    | 2/2    | 0    | 0    |

---

## 详细结果

| # | 场景（用户视角） | Level | 命令 | 结果 | TxHash / Calldata | 备注 |
|---|----------------|-------|------|------|-------------------|------|
| 1 | 编译 release binary | L1 | `cargo build --release` | ✅ PASS | — | Binary at target/release/uniswap-v3 |
| 2 | Lint 检查 | L1 | `plugin-store lint .` | ✅ PASS | — | "passed all checks" |
| 3 | 路由正向案例 (24个) | L0 | 多条自然语言触发短语 | ✅ 24/24 PASS | — | 覆盖 get-quote/swap/get-pools/get-positions/add-liquidity/remove-liquidity |
| 4 | 路由负向案例 (20个) | L0 | 多条应排除短语 | ✅ 20/20 PASS | — | pancakeswap/raydium/aave 等竞品正确排除 |
| 5 | get-quote: 0.0001 WETH→USDC (自动最优费率) | L2 | `get-quote --token-in WETH --token-out USDC --amount 0.0001 --chain 8453` | ✅ PASS | — | fee_tier=0.01%, amount_out=0.232758 USDC, rate=1 WETH=2327.58 USDC |
| 6 | get-quote: 0.0001 WETH→USDC (指定fee=3000) | L2 | `get-quote --token-in WETH --token-out USDC --amount 0.0001 --chain 8453 --fee 3000` | ✅ PASS | — | fee_tier=0.3%, amount_out=0.232292 USDC |
| 7 | get-pools: Base 上 WETH/USDC 全部池子 | L2 | `get-pools --token-a WETH --token-b USDC --chain 8453` | ✅ PASS | — | 4个池子，0.01%/0.05%/0.3%/1%，均返回非零地址和流动性 |
| 8 | get-pools: Ethereum 上 WETH/USDC 全部池子 | L2 | `get-pools --token-a WETH --token-b USDC --chain 1` | ✅ PASS | — | 4个池子，factory=0x1F98431c8aD98523631AE4a59f267346ea31F984，与Base地址不同 |
| 9 | get-positions: 当前钱包在 Base 上的 LP 仓位 | L2 | `get-positions --chain 8453` | ✅ PASS | — | 返回空仓位（"No Uniswap V3 LP positions found"）, 可接受 |
| 10 | get-positions: 查询特定 token-id=1 | L2 | `get-positions --token-id 1 --chain 8453` | ✅ PASS | — | 返回 Position #1 (BALD/WETH 0.3% 池), 含完整字段 |
| 11 | swap dry-run: WETH→USDC 基础测试 | L3 | `swap --token-in WETH --token-out USDC --amount 0.0001 --slippage-bps 50 --chain 8453 --dry-run` | ✅ PASS | Approve: `0x095ea7b3...` / Swap: `0x04e45aaf...` | selector=0x04e45aaf ✅, to=SwapRouter02 0x2626664c...e481 ✅, --force 传递 ✅ |
| 12 | swap dry-run: 指定 fee=3000 | L3 | `swap --token-in WETH --token-out USDC --amount 0.0001 --chain 8453 --fee 3000 --dry-run` | ✅ PASS | Swap: `0x04e45aaf...0bb8...` | fee=0x0BB8 (3000) 正确编码在 calldata 中 ✅ |
| 13 | add-liquidity dry-run: WETH+USDC full range | L3 | `add-liquidity --token-a WETH --token-b USDC --fee 3000 --amount-a 0.0001 --amount-b 0.01 --tick-lower -887220 --tick-upper 887220 --chain 8453 --dry-run` | ✅ PASS | Approve: `0x095ea7b3...`, Mint: `0x88316456...` | NFPM=0x03a520b3...4f1 ✅, selector=0x88316456 ✅, 两步 approve 均传递 --force ✅ |
| 14 | swap 链上真实交易: 0.00005 WETH→USDC | L4 | `swap --token-in WETH --token-out USDC --amount 0.00005 --fee 3000 --slippage-bps 50 --chain 8453` | ✅ PASS | Approve: `0x468b9a3338c44c7532792467074b147702507c091ce8c3ea8b10b435c2c2a847` / Swap: `0x78f1378f94363c0f71ad1f338418e98494073214643da432c1908af29fc6a3ea` | 0.00005 WETH → ~0.116146 USDC, approve+swap 两笔均上链确认 |

---

## L3 Calldata 验证细节

### L3-01 swap dry-run (fee auto = 0.01%)
- Approve calldata: `0x095ea7b3` + SwapRouter02 address + max uint256 — CORRECT
- Swap calldata prefix: `0x04e45aaf` — CORRECT (SwapRouter02 exactInputSingle)
- `to`: `0x2626664c2603336E57B271c5C0b26F421741e481` (SwapRouter02 on Base) — CORRECT
- `--force` flag: present in both dry-run onchainos calls — CORRECT

### L3-02 swap dry-run (fee = 3000 = 0x0BB8)
- Fee field in calldata at bytes 68-75: `0000000000000000000000000000000000000000000000000000000000000bb8` — CORRECT

### L3-03 add-liquidity dry-run
- Approve WETH calldata: `0x095ea7b3` + NFPM address — CORRECT
- Approve USDC calldata: `0x095ea7b3` + NFPM address — CORRECT
- Mint calldata prefix: `0x88316456` — CORRECT (NFPM mint)
- `to`: `0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1` (NFPM on Base) — CORRECT

---

## L4 链上交易详情

### L4-01: Approve WETH for SwapRouter02
- TxHash: `0x468b9a3338c44c7532792467074b147702507c091ce8c3ea8b10b435c2c2a847`
- Link: https://basescan.org/tx/0x468b9a3338c44c7532792467074b147702507c091ce8c3ea8b10b435c2c2a847
- 状态: Confirmed

### L4-02: exactInputSingle Swap (0.00005 WETH → ~0.116146 USDC)
- TxHash: `0x78f1378f94363c0f71ad1f338418e98494073214643da432c1908af29fc6a3ea`
- Link: https://basescan.org/tx/0x78f1378f94363c0f71ad1f338418e98494073214643da432c1908af29fc6a3ea
- 状态: Confirmed
- 输入: 0.00005 WETH
- 输出: ~0.116146 USDC
- 最低可接受输出: 0.115565 USDC (50 bps slippage)
- 费率: 0.3% (fee=3000)

---

## 修复记录

无需修复 — 插件一次性通过所有测试级别。

### 观察到的已知行为（非缺陷）

1. **get-positions 返回空列表**: 测试钱包 (0xee385ac7...bae9) 在 Base 上没有 Uniswap V3 LP 仓位，返回空是正确行为。
2. **L0 路由 flag 名称**: SKILL.md 中 `get-quote`/`swap` 命令示例使用 `--amount`（非 `--amount-in`），与实际 binary 一致，无误。
3. **swap 输出费率显示**: L3-01 dry-run 使用自动最优费率 0.01%；L4 链上测试指定 fee=3000 (0.3%) 均按预期工作。
