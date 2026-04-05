# 测试结果报告

- 日期: 2026-04-05
- DApp 支持的链: EVM (Ethereum mainnet chain ID: 1)
- EVM 测试链: Ethereum mainnet (1)
- 编译: ✅
- Lint: ✅ (0 errors, 0 warnings)
- **整体通过标准**: EVM DApp → EVM 全通过

## 汇总

| 总数 | L1编译 | L2读取 | L3模拟 | L4链上 | 失败 | 阻塞 |
|------|--------|--------|--------|--------|------|------|
| 9    | 2      | 2      | 4      | 1 pass + 2 skip | 0 | 0 |

## 详细结果

| # | 场景（用户视角） | Level | 命令 | 结果 | TxHash / Calldata | 备注 |
|---|----------------|-------|------|------|-------------------|------|
| 1 | 编译 debug build | L1 | `cargo build` | ✅ PASS | — | 0 warnings |
| 2 | Lint 检查 | L1 | `cargo clean && plugin-store lint .` | ✅ PASS | — | 0 errors, 0 warnings |
| 3 | 查询 apxETH 汇率和 TVL | L2 | `rates` | ✅ PASS | — | apxeth_per_pxeth=1.11605975, total_assets=2598 ETH |
| 4 | 查询测试钱包持仓 | L2 | `positions --address 0x87fb...` | ✅ PASS | — | 正确返回 pxETH=0, apxETH=0 |
| 5 | 模拟存入 0.00005 ETH 到 pxETH | L3 | `deposit --amount 0.00005 --chain 1 --dry-run` | ✅ PASS | calldata: `0xadc9740c...` | selector adc9740c ✅, paused warning included |
| 6 | 模拟存入 ETH 并自动 compound | L3 | `deposit --amount 0.00005 --compound --chain 1 --dry-run` | ✅ PASS | calldata: `0xadc9740c...0001` | compound=1 correctly encoded |
| 7 | 模拟质押 0.00005 pxETH 到 apxETH | L3 | `stake --amount 0.00005 --chain 1 --dry-run` | ✅ PASS | approve=`0x095ea7b3...` deposit=`0x6e553f65...` | both selectors ✅ |
| 8 | 模拟赎回 0.00005 apxETH | L3 | `redeem --amount 0.00005 --chain 1 --dry-run` | ✅ PASS | calldata: `0xba087652...` | selector ba087652 ✅ |
| 9 | Deposit 检测 PirexEth 暂停状态 | L4 | `deposit --amount 0.00005 --chain 1` | ✅ PASS | — | eth_call paused()=true, graceful error returned |
| 10 | 质押 pxETH → apxETH (链上) | L4-SKIP | `stake --amount 0.00005 --chain 1` | ⏭ SKIPPED | — | 无法获取 pxETH (PirexEth 已暂停，无法存入 ETH) |
| 11 | 赎回 apxETH → pxETH (链上) | L4-SKIP | `redeem --amount 0.00005 --chain 1` | ⏭ SKIPPED | — | 需要先完成 L4 stake，当前跳过 |

## 修复记录

无 — 首次测试全部通过。

## 备注

- PirexEth 主合约 (`0xD664b74274DfEB538d9baC494F3a4760828B02b0`) 当前处于 **paused** 状态
- `deposit` 命令通过 eth_call `paused()` 检测并返回友好的错误消息
- apxETH vault (`0x9Ba021B0a9b958B5E75cE9f6dff97C7eE52cb3E6`) 无 pause 机制，`stake`/`redeem` 代码路径正确 (经 L3 dry-run 验证)
- 由于无法在 mainnet 获取 pxETH，`stake` 和 `redeem` L4 测试标记为 SKIPPED
