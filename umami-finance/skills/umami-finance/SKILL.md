# Umami Finance Skill

Umami Finance is an Arbitrum-native yield protocol offering auto-compounding GM Vaults built on GMX V2. Users deposit USDC, WETH, or WBTC to earn yield from GMX trading fees with delta-neutral hedging.

All write operations are submitted via `onchainos wallet contract-call` after asking the user to confirm.

## Architecture

- **Read ops** → direct `eth_call` to vault contracts (no gas)
- **Write ops** → after user confirmation, submits via `onchainos wallet contract-call --chain 42161`
- **Vaults**: ERC-4626 standard contracts on Arbitrum (chain 42161)

---

## Commands

### list-vaults — List all vaults

**Trigger phrases:** "show umami vaults", "list umami finance vaults", "what vaults does umami have", "umami vault APR"

```bash
umami-finance list-vaults [--chain 42161]
```

Returns all GM vaults with TVL, price per share, and underlying asset.

**Example output:**
```json
{
  "ok": true,
  "vaults": [
    {
      "name": "gmUSDC-eth",
      "asset_symbol": "USDC",
      "total_assets_human": "63003.346783 USDC",
      "price_per_share_human": "1.15400000 USDC/share"
    }
  ]
}
```

---

### vault-info — Get vault details

**Trigger phrases:** "umami vault info", "show me gmUSDC vault", "what is the TVL of umami WETH vault"

```bash
umami-finance vault-info --vault <gmUSDC-eth|gmUSDC-btc|gmWETH|gmWBTC> [--chain 42161]
```

**Parameters:**
- `--vault`: vault name (`gmUSDC-eth`, `gmUSDC-btc`, `gmWETH`, `gmWBTC`) or contract address

---

### positions — Show user's vault positions

**Trigger phrases:** "show my umami positions", "what's in my umami vaults", "my umami balance"

```bash
umami-finance positions [--from <wallet_address>] [--chain 42161]
```

Returns all vaults where user has a non-zero share balance, with estimated asset value.

---

### deposit — Deposit assets into a vault

**Trigger phrases:** "deposit into umami", "add USDC to umami vault", "invest in umami gmUSDC"

```bash
umami-finance deposit --vault <vault> --amount <amount> [--from <wallet>] [--chain 42161] [--dry-run]
```

**Parameters:**
- `--vault`: vault name (e.g., `gmUSDC-eth`)
- `--amount`: amount in human-readable units (e.g., `10.0` for 10 USDC)
- `--from`: sender address (optional)
- `--dry-run`: preview without broadcasting

**Steps (on-chain):**
1. Run `--dry-run` to preview shares received — **ask user to confirm** before proceeding
2. ERC-20 approve vault to spend asset (if allowance insufficient): `onchainos wallet contract-call --chain 42161 --to <ASSET_CONTRACT> --input-data 0x095ea7b3...`
3. Deposit: `onchainos wallet contract-call --chain 42161 --to <VAULT_CONTRACT> --input-data 0x6e553f65...`

---

### redeem — Redeem shares from a vault

**Trigger phrases:** "withdraw from umami", "redeem umami shares", "take out USDC from umami vault"

```bash
umami-finance redeem --vault <vault> [--shares <amount>] [--from <wallet>] [--chain 42161] [--dry-run]
```

**Parameters:**
- `--vault`: vault name (e.g., `gmWETH`)
- `--shares`: number of shares to redeem (optional, defaults to all shares)
- `--from`: wallet address (optional)
- `--dry-run`: preview without broadcasting

**Steps (on-chain):**
1. Run `--dry-run` to preview assets to receive — **ask user to confirm** before proceeding
2. Redeem: `onchainos wallet contract-call --chain 42161 --to <VAULT_CONTRACT> --input-data 0xba087652...`

---

## Supported Vaults (Arbitrum, chain 42161)

| Vault Name | Asset | Contract |
|-----------|-------|---------|
| `gmUSDC-eth` | USDC | `0x959f3807f0Aa7921E18c78B00B2819ba91E52FeF` |
| `gmUSDC-btc` | USDC | `0x5f851F67D24419982EcD7b7765deFD64fBb50a97` |
| `gmWETH` | WETH | `0x4bCA8D73561aaEee2D3a584b9F4665310de1dD69` |
| `gmWBTC` | WBTC | `0xcd8011AaB161A75058eAb24e0965BAb0b918aF29` |

## Notes

- Only Arbitrum (chain 42161) is supported
- GM Vaults use ERC-4626-based vaults with custom deposit/redeem functions (includes slippage parameters)
- Vaults earn yield from GMX V2 trading fees with delta-neutral hedging
- Umami vaults use Chainlink Data Streams for price validation — deposits require keeper coordination
- Read operations (list-vaults, vault-info, positions) work without any restrictions

## ⚠️ Protocol Status

Umami Finance GM Vaults are live on Arbitrum (read operations fully functional).
Write operations (deposit/redeem) require keeper coordination via the Umami Finance app.
Direct on-chain deposit/redeem calls may revert depending on keeper state.
