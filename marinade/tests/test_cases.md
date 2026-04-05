# Marinade Plugin — Test Cases

## DApp 支持的链

Solana (501) — 仅 Solana

---

## L1 — Compile + Lint

| # | 测试 | 命令 | 预期 |
|---|------|------|------|
| 1 | 编译 debug build | `cargo build` | 0 errors |
| 2 | 编译 release build | `cargo build --release` | 0 errors |
| 3 | Lint | `cargo clean && plugin-store lint .` | 0 errors |

---

## L2 — Read Tests (No Gas)

| # | 场景 | 命令 | 预期 |
|---|------|------|------|
| 4 | 查询 mSOL/SOL 汇率和 APY | `marinade rates` | `ok: true`, `msol_per_sol > 1.0` |
| 5 | 查询用户 mSOL 持仓 | `marinade positions` | `ok: true`, `wallet` non-empty |

---

## L3 — Dry-Run Tests (No Gas)

| # | 场景 | 命令 | 预期 |
|---|------|------|------|
| 6 | 模拟质押 0.001 SOL | `marinade --dry-run stake --amount 0.001` | `dry_run: true`, `from: SOL_NATIVE` |
| 7 | 模拟解质押 0.001 mSOL | `marinade --dry-run unstake --amount 0.001` | `dry_run: true`, `from: MSOL_MINT` |

---

## L4 — On-chain Write Tests (Need Lock, Spend Gas)

| # | 场景 | 命令 | 预期 |
|---|------|------|------|
| 8 | 质押 0.001 SOL → mSOL | `marinade stake --amount 0.001` | txHash, `action: stake` |
| 9 | 解质押 0.0005 mSOL → SOL | `marinade unstake --amount 0.0005` | txHash, `action: unstake` |
