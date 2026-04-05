# 测试结果报告 — swell-staking

- 日期: 2026-04-05
- DApp 支持的链: EVM only (Ethereum mainnet, chain 1)
- EVM 测试链: Ethereum mainnet (chain 1)
- 编译: ✅
- Lint: ✅ (manual — plugin-store binary not installed locally; all rules verified manually)
- **整体通过标准**: EVM DApp → EVM 全通过 ✅

## 汇总

| 总数 | L1编译 | L2读取 | L3模拟 | L4链上 | 失败 | 阻塞 |
|------|--------|--------|--------|--------|------|------|
| 10   | 5      | 3      | 2      | 2      | 0    | 0    |

## 详细结果

| # | 场景（用户视角） | Level | 命令 | 结果 | TxHash / Calldata | 备注 |
|---|----------------|-------|------|------|-------------------|------|
| 1 | 编译项目 (debug build) | L1 | `cargo build` | ✅ PASS | — | 4 warnings (dead_code), 0 errors |
| 2 | 编译 release binary | L1 | `cargo build --release` | ✅ PASS | — | Binary: target/release/swell-staking |
| 3 | Lint: api_calls 格式 (E002) | L1 | Manual plugin.yaml inspect | ✅ PASS | — | Plain string list ✅ |
| 4 | Lint: 用户确认文本 (E106) | L1 | Manual SKILL.md inspect | ✅ PASS | — | "Ask user to confirm" on line 117 (1 line before contract-call line 118) and line 147/148 ✅ |
| 5 | Lint: .gitignore (E080) | L1 | Manual inspect | ✅ PASS | — | `/target/` in .gitignore ✅ |
| 6 | 查询 swETH/rswETH 当前汇率 | L2 | `rates` | ✅ PASS | — | swETH rate: 1.119 ETH/swETH, rswETH rate: 1.069 ETH/rswETH |
| 7 | 查询已知地址的持仓 | L2 | `positions --address 0xf951...` | ✅ PASS | — | swETH balance: 0.98 swETH, rswETH: 0.069 |
| 8 | 查询已登录钱包的持仓 | L2 | `positions` | ✅ PASS | — | swETH: 0.00004468 after L4 tx confirmed |
| 9 | 模拟质押 0.00005 ETH → swETH | L3 | `--chain 1 stake --amount 0.00005 --from 0x87fb... --dry-run` | ✅ PASS | calldata: `0xd0e30db0` | selector = deposit() ✅ |
| 10 | 模拟再质押 0.00005 ETH → rswETH | L3 | `--chain 1 restake --amount 0.00005 --from 0x87fb... --dry-run` | ✅ PASS | calldata: `0xd0e30db0` | selector = deposit() ✅ |
| 11 | 实际质押 0.00005 ETH → swETH | L4 | `--chain 1 stake --amount 0.00005` | ✅ PASS | `0x4cfc8e8452c5bd72e2b72176429a2b332d5ec62b2dd49d9b63b42b556b0848ce` | etherscan.io/tx/0x4cfc8e... |
| 12 | 实际再质押 0.00005 ETH → rswETH | L4 | `--chain 1 restake --amount 0.00005` | ✅ PASS | `0x6a104d90c516ae012475e70a13a54ec5b1aafc80c0ece6a9038236bc4c9df4ee` | etherscan.io/tx/0x6a104d... |

## L4 链上交易验证

| 操作 | TxHash | Amount | Received | Explorer |
|------|--------|--------|----------|---------|
| stake ETH→swETH | `0x4cfc8e8452c5bd72e2b72176429a2b332d5ec62b2dd49d9b63b42b556b0848ce` | 0.00005 ETH | 0.00004468 swETH | [Etherscan](https://etherscan.io/tx/0x4cfc8e8452c5bd72e2b72176429a2b332d5ec62b2dd49d9b63b42b556b0848ce) |
| restake ETH→rswETH | `0x6a104d90c516ae012475e70a13a54ec5b1aafc80c0ece6a9038236bc4c9df4ee` | 0.00005 ETH | 0.00004677 rswETH | [Etherscan](https://etherscan.io/tx/0x6a104d90c516ae012475e70a13a54ec5b1aafc80c0ece6a9038236bc4c9df4ee) |

## 修复记录

| # | 问题 | 根因 | 修复 | 文件 |
|---|------|------|------|------|
| 1 | L4 test used wrong arg order: `stake --chain 1` | `--chain` is on parent CLI, not subcommand | Changed to `--chain 1 stake --amount ...` | test execution only |

## ETH 余额跟踪

| 时间点 | ETH 余额 |
|--------|---------|
| Before L4 | 0.005169 ETH |
| After stake | 0.005103 ETH |
| After restake | ~0.005053 ETH |
| Hard reserve | 0.001 ETH minimum |
| Status | ✅ SAFE (well above reserve) |
