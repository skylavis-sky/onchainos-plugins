# Gearbox V3 Plugin Test Cases

## L1 — Binary Smoke Test

```bash
gearbox-v3 --version
gearbox-v3 --help
```

Expected: version string, help text listing all subcommands.

---

## L2 — Read Operations (no wallet required)

### get-pools

```bash
gearbox-v3 get-pools --chain 42161
```

Expected output:
- `ok: true`
- `creditManagers` array with 6 entries (Trade USDC/USDC.e/WETH Tier 1/2)
- Each entry includes `minDebt`, `maxDebt`, `creditFacade`, `creditManager`
- Trade USDC Tier 2 shows minDebt "1000.00 USDC"

---

## L3 — Dry-Run Write Operations (no wallet, no broadcast)

### open-account dry-run

```bash
gearbox-v3 open-account --dry-run \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --token USDC \
  --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --collateral 1000 \
  --borrow 2000
```

Expected:
- `ok: true`, `dryRun: true`
- `steps` array with 2 entries: approve + openCreditAccount
- `steps[0].inputData` starts with `0x095ea7b3` (approve selector)
- `steps[1].inputData` starts with `0x92beab1d` (openCreditAccount selector)
- `borrowAmount: 2000`, `collateralAmount: 1000`, `totalPosition: 3000`

### add-collateral dry-run

```bash
gearbox-v3 add-collateral --dry-run \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --account 0x0000000000000000000000000000000000000001 \
  --token USDC \
  --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --amount 500
```

Expected:
- `ok: true`, `dryRun: true`
- `steps[1].inputData` starts with `0xebe4107c` (multicall selector)

### close-account dry-run

```bash
gearbox-v3 close-account --dry-run \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --account 0x0000000000000000000000000000000000000001 \
  --underlying 0xaf88d065e77c8cC2239327C5EDb3A432268e5831
```

Expected:
- `ok: true`, `dryRun: true`
- `steps[0].inputData` starts with `0x36b2ced3` (closeCreditAccount selector)

### withdraw dry-run

```bash
gearbox-v3 withdraw --dry-run \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --account 0x0000000000000000000000000000000000000001 \
  --token USDC \
  --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --amount 200
```

Expected:
- `ok: true`, `dryRun: true`
- `steps[0].inputData` starts with `0xebe4107c` (multicall selector)

---

## L4 — Live Write Operations (requires funded wallet)

### Prerequisites
- Wallet funded with at least 1000 USDC on Arbitrum (chain 42161)
- Active onchainos session (`onchainos wallet status`)

### get-account (live)

```bash
gearbox-v3 get-account --chain 42161
```

Expected: `ok: true`, shows credit account count (0 if no open accounts).

### open-account (live, minimum viable)

```bash
gearbox-v3 open-account \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --token USDC \
  --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --collateral 500 \
  --borrow 1000
```

Expected:
- `ok: true`
- `approveTxHash` is a valid 0x hash
- `openAccountTxHash` is a valid 0x hash

Note: borrow=1000 satisfies minDebt=1000 USDC for Trade USDC Tier 2.

### Borrow below minDebt (should error)

```bash
gearbox-v3 open-account \
  --chain 42161 \
  --facade 0x3974888520a637ce73bdcb2ee28a396f4b303876 \
  --manager 0xb780dd9cec259a0bbf7b32587802f33730353e86 \
  --token USDC \
  --token-addr 0xaf88d065e77c8cC2239327C5EDb3A432268e5831 \
  --collateral 100 \
  --borrow 100
```

Expected: error message "below minimum debt" with current minDebt value.
