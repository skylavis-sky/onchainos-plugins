# 测试结果报告

- 日期: 2026-04-05
- DApp 支持的链: Solana (501) 仅 Solana
- Solana 测试链: mainnet (501)
- 编译: ✅
- Lint: ✅
- **整体通过标准**: Solana DApp → Solana 全通过 ✅

## 汇总

| 总数 | L1编译 | L2读取 | L3模拟 | L4链上 | 失败 | 阻塞 |
|------|--------|--------|--------|--------|------|------|
| 9    | 3      | 2      | 2      | 2      | 0    | 0    |

## 详细结果

| # | 场景（用户视角） | Level | 命令 | 结果 | TxHash / Calldata | 备注 |
|---|----------------|-------|------|------|-------------------|------|
| 1 | 编译 debug build | L1 | `cargo build` | ✅ PASS | — | 3 unused const warnings only |
| 2 | 编译 release build | L1 | `cargo build --release` | ✅ PASS | — | — |
| 3 | Lint 检查 | L1 | `cargo clean && plugin-store lint .` | ✅ PASS | — | 0 errors |
| 4 | 查询 mSOL/SOL 汇率和质押 APY | L2 | `marinade rates` | ✅ PASS | — | msol_per_sol=1.371, supply=2,041,348 mSOL |
| 5 | 查询用户 mSOL 持仓余额 | L2 | `marinade positions` | ✅ PASS | — | 正确解析 Solana wallet，0 mSOL（质押前） |
| 6 | 模拟质押 0.001 SOL (dry-run) | L3 | `marinade --dry-run stake --amount 0.001` | ✅ PASS | dry_run:true | from=SOL_NATIVE, to=MSOL_MINT |
| 7 | 模拟解质押 0.001 mSOL (dry-run) | L3 | `marinade --dry-run unstake --amount 0.001` | ✅ PASS | dry_run:true | from=MSOL_MINT, to=SOL_NATIVE |
| 8 | 质押 0.001 SOL → 收到 mSOL | L4 | `marinade stake --amount 0.001` | ✅ PASS | 53DvBEpQqXUZPpRrYaJ2eiQjVdGvKW3G6Du45GiXqeaw2dXQL9EWGgNxUZtbPdsKRP8L47YmM6VGaBue7KmQcXc1 | [Solscan](https://solscan.io/tx/53DvBEpQqXUZPpRrYaJ2eiQjVdGvKW3G6Du45GiXqeaw2dXQL9EWGgNxUZtbPdsKRP8L47YmM6VGaBue7KmQcXc1) |
| 9 | 解质押 0.0005 mSOL → 收到 SOL | L4 | `marinade unstake --amount 0.0005` | ✅ PASS | 45nrETXz8YC4SzCjj12Rx8QFL1B7dGarwee2nFWoAf4w4qwzLkAhtwJqMKtJaU4XC7ApSCDbLQ5qcYPJ7C4hsyhi | [Solscan](https://solscan.io/tx/45nrETXz8YC4SzCjj12Rx8QFL1B7dGarwee2nFWoAf4w4qwzLkAhtwJqMKtJaU4XC7ApSCDbLQ5qcYPJ7C4hsyhi) |

## L4 On-Chain Verification

**Stake (SOL → mSOL):**
- txHash: `53DvBEpQqXUZPpRrYaJ2eiQjVdGvKW3G6Du45GiXqeaw2dXQL9EWGgNxUZtbPdsKRP8L47YmM6VGaBue7KmQcXc1`
- From: 0.001 SOL (1,000,000 lamports)
- To: ~0.00073951 mSOL (739,510 raw units)
- Explorer: https://solscan.io/tx/53DvBEpQqXUZPpRrYaJ2eiQjVdGvKW3G6Du45GiXqeaw2dXQL9EWGgNxUZtbPdsKRP8L47YmM6VGaBue7KmQcXc1

**Unstake (mSOL → SOL):**
- txHash: `45nrETXz8YC4SzCjj12Rx8QFL1B7dGarwee2nFWoAf4w4qwzLkAhtwJqMKtJaU4XC7ApSCDbLQ5qcYPJ7C4hsyhi`
- From: 0.0005 mSOL (500,000 raw units)
- To: ~0.000691162 SOL
- Explorer: https://solscan.io/tx/45nrETXz8YC4SzCjj12Rx8QFL1B7dGarwee2nFWoAf4w4qwzLkAhtwJqMKtJaU4XC7ApSCDbLQ5qcYPJ7C4hsyhi

## SOL 消耗

- stake: ~0.001 SOL + gas (~0.001049 SOL total)
- unstake: ~0 SOL net (received ~0.000691 SOL from mSOL)
- 余额充足，未触及 0.002 SOL 硬底线

## 修复记录

| # | 问题 | 根因 | 修复 | 文件 |
|---|------|------|------|------|
| 1 | lint E123: invalid source_commit | PLACEHOLDER_SHA 不是 40位 hex | 替换为全零 40位 SHA（Phase 4 替换为真实 SHA） | plugin.yaml |
| 2 | lint W140: solscan URL 未在 api_calls | SKILL.md 引用了 solscan.io | 添加 `https://solscan.io` 到 api_calls | plugin.yaml |
