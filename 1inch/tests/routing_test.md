# Routing Test — 1inch Plugin

**Phase:** L0 — SKILL.md Routing Validation
**Date:** 2026-04-19
**Plugin version:** 0.1.0

---

## 1. Trigger Phrase Coverage

### English triggers (SKILL.md)
| Phrase | Maps to command | Correct? |
|--------|----------------|---------|
| "1inch", "swap on 1inch", "1inch quote" | get-quote / swap | Yes |
| "best swap rate", "swap via 1inch" | swap | Yes |
| "check allowance 1inch" | get-allowance | Yes |
| "approve token 1inch" | approve | Yes |

### Chinese triggers (SKILL.md)
| Phrase | Maps to command | Correct? |
|--------|----------------|---------|
| "1inch 换币", "1inch 报价" | get-quote / swap | Yes |
| "用1inch兑换" | swap | Yes |
| "1inch 查询额度" | get-allowance | Yes |
| "1inch 代币授权" | approve | Yes |

### Negative routing
| Phrase | Expected behaviour |
|--------|--------------------|
| "swap SOL on Solana" | MUST NOT trigger; user should be directed to jupiter |
| "Uniswap swap" | MUST NOT trigger |
| "pump.fun buy" | MUST NOT trigger |

**Assessment:** Trigger phrase coverage is complete and accurate. Negative routing is correctly documented in SKILL.md ("Do NOT use for: non-EVM chains").

---

## 2. Command Routing

| User intent | Expected binary subcommand | --dry-run required? | Confirmation gate? |
|-------------|--------------------------|--------------------|--------------------|
| Get quote for ETH→USDC | `get-quote --src ETH --dst USDC --amount X --chain 8453` | No | No (read-only) |
| Swap ETH→USDC | `swap --src ETH --dst USDC --amount X --chain 8453` | No | Yes — swap broadcast |
| Swap USDC→ETH (ERC-20 src) | `swap --src USDC --dst ETH --amount X --chain 8453` | No | Yes — approve + swap |
| Check USDC allowance | `get-allowance --token USDC --chain 8453` | No | No (read-only) |
| Approve USDC unlimited | `approve --token USDC --chain 8453` | No | Yes — approve broadcast |
| Preview swap calldata | `swap ... --dry-run` | Yes | No (dry-run) |
| Preview approve calldata | `approve ... --dry-run` | Yes | No (dry-run) |

**Assessment:** All user intents map to correct subcommands with correct flag usage.

---

## 3. Confirmation Gate Audit

Per SKILL.md the plugin MUST ask user confirmation before:
- Broadcasting ERC-20 approve via `wallet contract-call`
- Broadcasting swap via `wallet contract-call`

**Source code review (main.rs):**
- `cmd_swap` (L4 live path): Emits `[confirm]` message before approve (line 221) and before swap (line 288). PASS.
- `cmd_approve` (L4 live path): Emits `[confirm]` message before broadcast (line 458). PASS.
- Both commands use `--force` in `wallet_contract_call()` — required by onchainos. PASS.

**Dry-run guard placement:**
- `swap`: dry-run guard fires BEFORE `resolve_wallet()` — wallet not required for dry-run. PASS.
- `approve`: same pattern. PASS.
- `get-allowance`: calls `resolve_wallet()` unconditionally even for ERC-20 tokens. This is a minor issue — get-allowance is read-only and the wallet address is required to call the 1inch allowance API. Acceptable behaviour.

---

## 4. Chain Validation

| Chain ID | Accepted? | Expected |
|----------|----------|---------|
| 1 (Ethereum) | Yes | Pass |
| 42161 (Arbitrum) | Yes | Pass |
| 8453 (Base) | Yes | Pass |
| 56 (BSC) | Yes | Pass |
| 137 (Polygon) | Yes | Pass |
| 999 (unsupported) | No — clear error message | Pass |

---

## 5. Token Resolution

| Token | Chain | Resolves? |
|-------|-------|----------|
| ETH | 8453 | Yes → NATIVE_TOKEN sentinel |
| USDC | 8453 | Yes → 0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913 |
| WETH | 8453 | Yes → 0x4200000000000000000000000000000000000006 |
| WBTC | 8453 | No — not in token map (error: unknown token) |
| 0x<addr> | any | Yes — passed through as-is |
| UNKNOWN_TOKEN | 8453 | No — clear error message |

**Gap noted:** WBTC is absent from Base token map. Not a blocker — SKILL.md does not list cbBTC/WBTC for Base.

---

## 6. SKILL.md vs plugin.yaml Version Consistency

| Field | SKILL.md | plugin.yaml |
|-------|---------|------------|
| version | 0.1.0 | 0.1.0 |
| name | 1inch | 1inch |
| chains | 1, 42161, 8453, 56, 137 | (implied by api_calls) |

**Assessment:** Version is consistent. PASS.

---

## 7. Overall L0 Result

**PASS** — Routing, command structure, confirmation gates, chain validation, and SKILL.md documentation all correctly reflect the plugin implementation.
