# Exactly Protocol Plugin — Test Cases

## L1: CLI Help / Structure

```bash
exactly-protocol --help
exactly-protocol get-markets --help
exactly-protocol deposit --help
exactly-protocol borrow --help
exactly-protocol repay --help
exactly-protocol withdraw --help
exactly-protocol enter-market --help
```

Expected: Each prints usage with correct flags.

---

## L2: Read-Only Tests (No Wallet Required)

### 2.1 Get Markets — Optimism
```bash
exactly-protocol get-markets --chain 10
```
Expected: JSON with `ok: true`, `marketCount >= 5`, markets include WETH, USDC, OP.

### 2.2 Get Markets — Ethereum
```bash
exactly-protocol get-markets --chain 1
```
Expected: JSON with `ok: true`, markets include WETH, USDC.

### 2.3 Get Position — Zero Address (no user data)
```bash
exactly-protocol get-position --chain 10 --from 0x0000000000000000000000000000000000000000
```
Expected: JSON with `ok: true`, `positionCount: 0` (zero address has no positions).

---

## L3: Dry-Run Tests (Wallet Required, No Broadcast)

### 3.1 Floating Deposit Dry Run
```bash
exactly-protocol deposit --market USDC --amount 100 --dry-run
```
Expected: JSON with `dryRun: true`, two steps (approve + deposit), calldata starts with `0x095ea7b3` and `0x6e553f65`.

### 3.2 Fixed Deposit Dry Run
```bash
exactly-protocol deposit --market USDC --amount 100 --maturity 1750896000 --dry-run
```
Expected: JSON with `dryRun: true`, calldata step 2 starts with `0x34f7d1f2`.

### 3.3 Floating Borrow Dry Run
```bash
exactly-protocol borrow --market USDC --amount 50 --dry-run
```
Expected: JSON with `dryRun: true`, calldata starts with `0xd5164184`, warning about enterMarket.

### 3.4 Fixed Borrow Dry Run
```bash
exactly-protocol borrow --market USDC --amount 50 --maturity 1750896000 --dry-run
```
Expected: calldata starts with `0x1a5b9e62`.

### 3.5 Enter Market Dry Run
```bash
exactly-protocol enter-market --market WETH --dry-run
```
Expected: calldata starts with `0x3fe5d425`, target is Auditor `0xaEb62e6F27BC103702E7BC879AE98bceA56f027E`.

### 3.6 Floating Repay Dry Run
```bash
exactly-protocol repay --market USDC --amount 50 --borrow-shares 49000000000000000 --dry-run
```
Expected: calldata step 2 starts with `0x7ad226dc`.

### 3.7 Fixed Repay Dry Run
```bash
exactly-protocol repay --market USDC --amount 50 --maturity 1750896000 --dry-run
```
Expected: calldata step 2 starts with `0x3c6f317f`.

### 3.8 Floating Withdraw Dry Run
```bash
exactly-protocol withdraw --market WETH --amount 0.5 --dry-run
```
Expected: calldata starts with `0xb460af94`.

### 3.9 Fixed Withdraw Dry Run
```bash
exactly-protocol withdraw --market WETH --amount 0.5 --maturity 1750896000 --dry-run
```
Expected: calldata starts with `0xa05a091a`.

### 3.10 Withdraw All Dry Run
```bash
exactly-protocol withdraw --market WETH --all --dry-run
```
Expected: calldata contains `ffffffff...` (uint256.max), warning about clearing borrows first.

---

## L4: Live Execution Tests (Requires Funded Wallet on Optimism)

Test order (prerequisite chain):
1. `get-position` - read baseline
2. `deposit --market USDC --amount 10` (floating) - approve + deposit
3. `enter-market --market USDC` - enable collateral
4. `get-position` - verify isCollateral=true for USDC
5. `borrow --market WETH --amount 0.001` (floating) - borrow
6. `repay --market WETH --amount 0.001 --borrow-shares <from-step-4>` - repay
7. `withdraw --market USDC --amount 10` - withdraw

### L4 Fixed-Rate Sub-flow (requires knowing a valid maturity)
1. `get-markets` - get valid maturity timestamp
2. `deposit --market USDC --amount 10 --maturity <ts>` - fixed deposit
3. `borrow --market WETH --amount 0.001 --maturity <ts>` - fixed borrow  
4. `repay --market WETH --amount <positionAssets> --maturity <ts>` - fixed repay
5. `withdraw --market USDC --amount 10 --maturity <ts>` (at/after maturity) - fixed withdraw

### Known Pitfalls to Test
- Verify `repay` with fixed maturity does NOT overflow (uses 0.1% buffer on maxAssets)
- Verify `borrow` correctly warns about enterMarket requirement
- Verify `withdraw --all` shows warning about zero-debt requirement

---

## Error Cases

```bash
# Invalid chain
exactly-protocol get-markets --chain 999
# Expected: error "Unsupported chain ID: 999"

# Invalid market
exactly-protocol deposit --market INVALID --amount 100 --dry-run
# Expected: error "Unknown market 'INVALID' on chain 10"

# Missing amount and --all
exactly-protocol withdraw --market WETH --dry-run
# Expected: error "Must specify either --amount or --all"
```
