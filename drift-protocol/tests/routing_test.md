# L0 Routing Tests — drift-protocol

Generated: 2026-04-19  
Source: skills/drift-protocol/SKILL.md

## Test Matrix

| ID | Input | Expected Route | Expected Outcome | Result |
|----|-------|---------------|------------------|--------|
| R01 | `drift-protocol --help` | top-level help | Lists all 6 subcommands | PASS |
| R02 | `drift-protocol get-balance --help` | get-balance help | Shows --chain option | PASS |
| R03 | `drift-protocol get-markets --help` | get-markets help | Shows --market, --depth options | PASS |
| R04 | `drift-protocol get-funding-rates --help` | get-funding-rates help | Shows --market option | PASS |
| R05 | `drift-protocol place-order --help` | place-order help | Shows --market, --side, --size, --price, --dry-run | PASS |
| R06 | `drift-protocol deposit --help` | deposit help | Shows --token, --amount, --dry-run | PASS |
| R07 | `drift-protocol cancel-order --help` | cancel-order help | Shows --order-id, --dry-run | PASS |
| R08 | `drift-protocol invalid-command` | clap error, exit 2 | "unrecognized subcommand" error | PASS |
| R09 | `drift-protocol place-order` (no args) | clap error, exit 2 | Missing required args error | PASS |
| R10 | `drift-protocol get-balance` | onchainos wallet balance --chain 501 | SOL/USDC/USDT balance JSON | PASS (live) |
| R11 | `drift-protocol get-markets` | DLOB API call | Paused error JSON (503) | PASS |
| R12 | `drift-protocol get-markets --market BTC-PERP` | DLOB API call with BTC-PERP | Paused error JSON (503) | PASS |
| R13 | `drift-protocol get-funding-rates` | data.api.drift.trade call | Paused error JSON | PASS |
| R14 | `drift-protocol place-order --market SOL-PERP --side buy --size 0.1 --price 85.0` | write stub | Paused write error JSON | PASS |
| R15 | `drift-protocol deposit --token USDT --amount 10.0` | write stub | Paused write error JSON | PASS |
| R16 | `drift-protocol cancel-order --order-id 123` | write stub | Paused write error JSON | PASS |

## Notes

- All 6 subcommands are correctly registered via clap `#[derive(Subcommand)]`
- Default values: get-balance chain=501, get-markets market=SOL-PERP depth=10, get-funding-rates market=SOL-PERP
- SKILL.md `deposit` example shows `--token USDT --amount 100.0` — implementation matches this signature
- SKILL.md `cancel-order` example shows `--order-id <ORDER_ID>` — implementation uses `Option<String>` (can be omitted)
- No routing gaps found; all SKILL.md-documented commands are implemented
