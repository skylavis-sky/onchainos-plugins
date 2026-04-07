# Sanctum Validator LSTs — Test Cases

## Smoke Tests (no wallet required)

### T1: list-lsts
```bash
./target/release/sanctum-validator-lst list-lsts
```
Expected: JSON with `ok: true`, `data.lsts` array containing jitoSOL, bSOL, etc. APY/TVL may be "N/A" if Extra API is down.

### T2: list-lsts --all
```bash
./target/release/sanctum-validator-lst list-lsts --all
```
Expected: Same as T1 but also includes INF and wSOL entries.

### T3: get-quote jitoSOL → bSOL
```bash
./target/release/sanctum-validator-lst get-quote --from jitoSOL --to bSOL --amount 0.1
```
Expected: `ok: true`, `data.in_amount_ui ≈ "0.100000000"`, `data.out_amount_ui` a similar value, `data.swap_src` is "SPool" or similar.

### T4: get-quote (small amount)
```bash
./target/release/sanctum-validator-lst get-quote --from jitoSOL --to mSOL --amount 0.005
```
Expected: `ok: true` — mSOL is routable via swap even though it cannot be staked directly.

### T5: swap-lst --dry-run
```bash
./target/release/sanctum-validator-lst swap-lst --from jitoSOL --to bSOL --amount 0.005 --dry-run
```
Expected: `ok: true`, `dry_run: true`, quote data present.

### T6: stake jitoSOL --dry-run
```bash
./target/release/sanctum-validator-lst stake --lst jitoSOL --amount 0.002 --dry-run
```
Expected: `ok: true`, `dry_run: true`, preview with stake pool info.

### T7: stake mSOL (should fail with helpful error)
```bash
./target/release/sanctum-validator-lst stake --lst mSOL --amount 0.002
```
Expected: `ok: false`, error contains "marinade" and "custom program".

### T8: get-position (requires wallet)
```bash
./target/release/sanctum-validator-lst get-position
```
Expected: Either `ok: true` with holdings array, or error if onchainos not logged in.

### T9: Unknown LST
```bash
./target/release/sanctum-validator-lst stake --lst unknownSOL --amount 0.001
```
Expected: `ok: false`, error mentions "list-lsts".

### T10: Help output
```bash
./target/release/sanctum-validator-lst --help
./target/release/sanctum-validator-lst stake --help
```
Expected: CLI help text showing all subcommands and arguments.

## Integration Tests (requires onchainos login)

### I1: Full swap flow (devnet or small amount)
```bash
./target/release/sanctum-validator-lst swap-lst --from jitoSOL --to bSOL --amount 0.0001
```
Expected: `ok: true`, `data.txHash` is a valid Solana transaction hash, `data.solscan` link.

### I2: Stake jitoSOL
```bash
./target/release/sanctum-validator-lst stake --lst jitoSOL --amount 0.001
```
Expected: `ok: true`, `data.txHash`, note about epoch delay.

### I3: get-position after stake
Wait ~2-3 days after I2, then:
```bash
./target/release/sanctum-validator-lst get-position
```
Expected: jitoSOL appears in holdings with positive balance.
