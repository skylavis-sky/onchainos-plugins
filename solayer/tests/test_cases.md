# Test Cases — Solayer Plugin

DApp: Solayer (Solana liquid restaking)
Chain: Solana mainnet (501)
Binary: `./target/release/solayer`

---

## L1 — Compilation + Lint

| # | Test | Command | Expected |
|---|------|---------|---------|
| L1-1 | Debug build | `cargo build` | Compiles successfully |
| L1-2 | Lint | `cargo clean && plugin-store lint .` | 0 errors |

---

## L2 — Read Tests (no wallet, no gas)

| # | Test | Command | Expected |
|---|------|---------|---------|
| L2-1 | Get staking rates | `solayer rates` | JSON with apy_percent, ssol_to_sol, tvl fields |
| L2-2 | Check positions | `solayer positions` | JSON with ssol_balance, sol_value, wallet address |

---

## L3 — Dry-run Tests

| # | Test | Command | Expected |
|---|------|---------|---------|
| L3-1 | Stake dry-run | `solayer --dry-run stake --amount 0.001` | dry_run:true, description mentions Jupiter routing |
| L3-2 | Unstake dry-run | `solayer --dry-run unstake --amount 0.001` | dry_run:true, message about UI, ui_url field |

---

## L4 — On-chain Tests (requires lock)

| # | Test | Command | Expected |
|---|------|---------|---------|
| L4-1 | Stake 0.001 SOL | `solayer stake --amount 0.001` | txHash returned, sSOL received in wallet |

**L4 excluded:**
- `unstake`: REST API not available; requires complex multi-instruction on-chain flow
