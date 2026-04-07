# Ion Protocol Plugin — Test Results

**Date:** 2026-04-07
**Plugin version:** 0.1.0
**Tester:** onchainos plugin tester (automated)
**Chain:** Ethereum Mainnet (chain 1)
**RPC:** https://ethereum.publicnode.com

---

## Overall Recommendation

**APPROVE FOR MERGE** — All testable levels pass (L1, L2, L3). L4 is blocked pending funded test wallet. No P0 issues found. Two P2 observations noted.

---

## Summary Table

| Level | Description | Result |
|-------|-------------|--------|
| L1 | Build + unit tests | PASS |
| L2 | Read operations (live RPC) | PASS |
| L3 | Dry-run write ops + selector verification | PASS |
| L4 | Live on-chain transactions | BLOCKED (no funded wallet) |
| Static Analysis | Code quality checks | PASS |

---

## L1 — Build

**Command:**
```
cargo build --release
cargo test
```

**Result: PASS**

- `cargo build --release` — Finished in 0.07s (pre-compiled). Zero errors, zero warnings promoted to errors.
- `cargo test` — 5/5 tests pass:
  - `test_encode_erc20_approve` — ok
  - `test_encode_supply` — ok
  - `test_encode_borrow` — ok
  - `test_encode_repay` — ok
  - `test_encode_deposit_collateral` — ok

---

## L2 — Read Operations

### get-pools

**Command:** `./target/release/ion-protocol get-pools --chain 1`

**Result: PASS**

All 4 pools returned with live APY/TVL from Ethereum mainnet:

| Pool | Borrow APY | Total Lend Supply |
|------|-----------|------------------|
| rsETH/wstETH | 32.6999% | 6.477897 wstETH |
| rswETH/wstETH | 2.7129% | 0.675760 wstETH |
| ezETH/WETH | 5.6664% | 0.006191 WETH |
| weETH/wstETH | 2.7657% | 2.187789 wstETH |

Response includes `ok: true`, `poolCount: 4`, per-pool `ionPool`, `gemJoin`, `collateral`, `lendToken` addresses and `ilkIndex: 0`. APY calculation via `getCurrentBorrowRate` per-second RAY rate annualized (linear approximation, noted in response).

### get-position

**Command:** `./target/release/ion-protocol get-position --chain 1 --from 0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1`

**Result: PASS**

Returns `ok: true`, `hasPositions: false`. All 4 pools show zero collateral, zero debt, zero lend balance. RPC `vault()` and `balanceOf()` calls succeed. `rateRay` values returned for each pool (confirming rate accumulation is live):
- rsETH/wstETH rate: 1420075720875281982100506497
- rswETH/wstETH rate: 1065890538496326575447522615
- ezETH/WETH rate: 1153277655794482256322856722
- weETH/wstETH rate: 1143988876836755911897044522

All rates > RAY (1e27), confirming debt has accrued since pool inception.

---

## L3 — Dry-Run Write Operations

All commands tested with `--from 0x742d35Cc6634C0532925a3b8D4C9b3f5E3b6b1f1 --dry-run`. Selectors extracted from returned calldata.

### lend

**Command:** `--dry-run lend --chain 1 --pool rsETH --amount 10000000000000000`

**Result: PASS**

- Step 1: `approve` to wstETH (`0x7f39C581...`) — calldata selector `0x095ea7b3` ✓
- Step 2: `supply` to IonPool (`0x0000000000E33e35EE6052...`) — calldata selector `0x7ca5643d` ✓
- Empty proof ABI encoding verified: trailing words `...0x60 (offset) 0x00 (length)` ✓

### borrow

**Command:** `--dry-run borrow --chain 1 --pool rsETH --collateral-amount 10000000000000000 --borrow-amount 5000000000000000`

**Result: PASS**

- Step 1: `approve` rsETH to GemJoin — selector `0x095ea7b3` ✓
- Step 2: `GemJoin.join` — selector `0x3b4da69f` ✓
- Step 3: `IonPool.depositCollateral` — selector `0x918a2f42` ✓
- Step 4: `IonPool.borrow` — selector `0x9306f2f8` ✓
- normalizedDebt computed via live on-chain rate: `3520938131827626` (< 5e15 as expected; rate > RAY) ✓
- Empty `bytes32[]` proof: borrow calldata word[4]=`0x00...a0` (offset=160), word[5]=`0x00...00` (length=0) ✓

### deposit-collateral

**Command:** `--dry-run deposit-collateral --chain 1 --pool rsETH --amount 10000000000000000`

**Result: PASS**

- Step 1: `approve` rsETH to GemJoin — selector `0x095ea7b3` ✓
- Step 2: `GemJoin.join` — selector `0x3b4da69f` ✓
- Step 3: `IonPool.depositCollateral` — selector `0x918a2f42` ✓

### repay

**Command:** `--dry-run repay --chain 1 --pool rsETH --amount 5000000000000000`

**Result: PASS**

- Step 1: `approve` wstETH to IonPool — selector `0x095ea7b3` ✓
- Step 2: `IonPool.repay` — selector `0x8459b437` ✓
- 0.1% buffer verified: `repayAmountHuman: "~0.005005 wstETH"` (input 0.005 wstETH + 0.1%) ✓
- normalizedDebt=`3524458192871300` = base `3520937255615684` × 1.001 ✓

### withdraw-lend

**Command:** `--dry-run withdraw-lend --chain 1 --pool rsETH --amount 10000000000000000`

**Result: PASS**

- Single call `IonPool.withdraw` to IonPool — selector `0xf3fef3a3` ✓

---

## L4 — Live On-Chain

**Result: BLOCKED**

No test wallet with rsETH or wstETH on Ethereum mainnet is available. L4 requires funded wallet with:
- 0.01 rsETH (collateral for deposit/borrow tests)
- 0.01 wstETH (for lend/repay tests)
- ETH for gas

---

## Static Analysis

### 1. ABI Selector Verification (Python keccak)

All 6 mandatory selectors verified via `Crypto.Hash.keccak`:

| Function Signature | Expected | Computed | Result |
|--------------------|----------|----------|--------|
| `supply(address,uint256,bytes32[])` | `7ca5643d` | `7ca5643d` | PASS |
| `join(address,uint256)` | `3b4da69f` | `3b4da69f` | PASS |
| `depositCollateral(uint8,address,address,uint256,bytes32[])` | `918a2f42` | `918a2f42` | PASS |
| `borrow(uint8,address,address,uint256,bytes32[])` | `9306f2f8` | `9306f2f8` | PASS |
| `repay(uint8,address,address,uint256)` | `8459b437` | `8459b437` | PASS |
| `approve(address,uint256)` | `095ea7b3` | `095ea7b3` | PASS |

Additional selectors present in `calldata.rs` / `rpc.rs` (not required but verified correct in test_cases.md):
`withdraw(0xf3fef3a3)`, `withdrawCollateral(0x743f9c0c)`, `exit(0xef693bed)`, `getCurrentBorrowRate(0x6908d3df)`, `vault(0x9a3db79b)`, `rate(0x3c04b547)`, `totalSupply(0x18160ddd)`, `balanceOf(0x70a08231)`, `normalizedDebt(0x57fc90b2)`.

### 2. extract_tx_hash_or_err

**Result: PASS**

`src/onchainos.rs` L148 defines `pub fn extract_tx_hash_or_err(result: &Value) -> anyhow::Result<String>`. The function:
- Returns `Err(...)` on `ok != true` (propagates errors correctly)
- Falls back to `"pending"` string only when the hash field is missing/empty (non-breaking fallback, not a silent swallow of errors)
- No use of `unwrap_or("pending")` pattern that would swallow contract-call failures

### 3. resolve_wallet chain 1 gotcha

**Result: PASS**

`src/onchainos.rs` L52-93: `resolve_wallet` uses `onchainos wallet addresses` (not `wallet balance --output json`). The code explicitly comments: "For chain 1 (Ethereum), --output json is NOT supported on wallet balance." Primary lookup filters by `chainIndex == "1"`. Fallback uses `wallet status --output json` (which does not suffer the EOF bug). Correct.

### 4. CJK in SKILL.md description field

**Result: PASS**

No CJK characters found anywhere in `skills/ion-protocol/SKILL.md`. Description field is ASCII/Latin only (485 chars).

### 5. "Do NOT use for" section in SKILL.md

**Result: PASS**

Section present at line 21 of SKILL.md. Contents are well-specified:
- Aave/Compound/Morpho (different interfaces)
- Non-Ethereum chains
- Liquid staking
- Bridging
- Claiming reward tokens
- AMM/DEX liquidity provision

### 6. repay 0.1% buffer on normalizedAmount

**Result: PASS**

Two code paths both apply buffer:

- `--all` path (`repay.rs` L51): `nd + nd / 1000`
- `--amount` path (`repay.rs` L60-62): `nd = to_normalized(borrow_amount, rate); nd + nd / 1000`

Dry-run confirmed: input 5000000000000000 WAD → normalizedDebt 3524458192871300 (1.001× base). `repayAmountHuman: "~0.005005 wstETH"` confirms approve amount also includes buffer.

### 7. borrow empty bytes32[] proof

**Result: PASS**

`encode_borrow` in `calldata.rs` L127-143 appends:
- `encode_u256_raw(0xa0)` → offset to dynamic array = 160 (5 params × 32)
- `encode_u256_raw(0x00)` → array length = 0

Confirmed in dry-run borrow calldata: word[4]=`0x...00000000a0`, word[5]=`0x...0000000000`. Proof is correctly empty for all users (whitelist is open).

---

## Issues Found

### P2-001 — dry-run requires explicit --from (no onchainos in test env)

**Severity:** P2 (cosmetic/test-env only)
**Description:** Without `--from`, the write commands attempt to call `onchainos wallet addresses` which fails in CI/test environments without onchainos installed. The dry-run still requires a wallet to be resolved before generating calldata (wallet address is embedded in ABI encoding). This is by-design but means dry-run tests must always pass `--from`.
**Impact:** None in production (onchainos is available). Test documentation should note `--from` is required for dry-run in isolated environments.
**Recommendation:** Add note to SKILL.md or consider a synthetic fallback address for dry-run if wallet resolution fails.

### P2-002 — APY uses linear approximation (noted in response but not in SKILL.md)

**Severity:** P2 (documentation)
**Description:** `borrow_rate_to_apy_pct` in `rpc.rs` L238-256 uses a linear approximation (`per_sec_excess * SECONDS_PER_YEAR * 100`) rather than compound formula (`(1 + r)^N - 1`). For rsETH pool at ~32.7% APY, compound formula would yield ~38.5% vs linear ~32.7%. The response JSON includes a disclaimer note on each pool, but SKILL.md does not mention this.
**Impact:** APY figures understate true borrowing cost by ~5-6% at current rates. Not harmful to protocol safety but may mislead users comparing rates.
**Recommendation:** Either switch to compound APY or clarify in SKILL.md that APY is a linear approximation.

---

## ABI Selector Reference Table

| Contract | Function | Selector | Verified |
|----------|----------|----------|---------|
| ERC-20 | `approve(address,uint256)` | `0x095ea7b3` | Python keccak + dry-run |
| IonPool | `supply(address,uint256,bytes32[])` | `0x7ca5643d` | Python keccak + dry-run |
| IonPool | `withdraw(address,uint256)` | `0xf3fef3a3` | dry-run |
| GemJoin | `join(address,uint256)` | `0x3b4da69f` | Python keccak + dry-run |
| GemJoin | `exit(address,uint256)` | `0xef693bed` | test_cases.md |
| IonPool | `depositCollateral(uint8,address,address,uint256,bytes32[])` | `0x918a2f42` | Python keccak + dry-run |
| IonPool | `withdrawCollateral(uint8,address,address,uint256)` | `0x743f9c0c` | test_cases.md |
| IonPool | `borrow(uint8,address,address,uint256,bytes32[])` | `0x9306f2f8` | Python keccak + dry-run |
| IonPool | `repay(uint8,address,address,uint256)` | `0x8459b437` | Python keccak + dry-run |
| IonPool | `getCurrentBorrowRate(uint8)` | `0x6908d3df` | test_cases.md |
| IonPool | `vault(uint8,address)` | `0x9a3db79b` | test_cases.md |
| IonPool | `rate(uint8)` | `0x3c04b547` | test_cases.md |
| IonPool | `totalSupply()` | `0x18160ddd` | test_cases.md |
| IonPool | `balanceOf(address)` | `0x70a08231` | test_cases.md |
| IonPool | `normalizedDebt(uint8,address)` | `0x57fc90b2` | test_cases.md |
