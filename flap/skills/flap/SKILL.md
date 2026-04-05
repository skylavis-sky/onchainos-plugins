---
name: flap
description: "Interact with Flap Protocol bonding curves on BSC (chain 56): create standard or tax tokens, buy tokens with BNB, sell tokens for BNB, and query bonding curve state. Trigger phrases: create token on Flap, buy Flap token, sell Flap token, check Flap bonding curve, launch meme token BSC, Flap launchpad, flap.sh token. Note: DEX-graduated tokens (status=DEX) must be traded via DEX, not through this plugin."
license: MIT
metadata:
  author: skylavis-sky
  version: "0.1.0"
---

## Architecture

- **Read ops** (`get-token-info`) use direct `eth_call` to BSC RPC; no onchainos needed
- **Write ops** (`create-token`, `buy`, `sell`) run `--dry-run` first, then after explicit user confirmation, submit via `onchainos wallet contract-call --chain 56 --force`
- **Sell** requires an ERC-20 `approve` to Portal before `swapExactInput` (done automatically); after user confirmation, executes via `onchainos wallet contract-call --chain 56 --force`
- **CREATE2 salt grinding**: `create-token` iterates ~65,536 salts to find a vanity address suffix (8888 standard, 7777 tax). Use `--skip-salt-grind` to bypass.

## Chain

BSC Mainnet (chain 56). Portal proxy: `0xe2cE6ab80874Fa9Fa2aAE65D277Dd6B8e65C9De0`

## Execution Flow for Write Operations

**IMPORTANT: Always ask for explicit user confirmation before executing any on-chain transaction.**

1. Run with `--dry-run` first to preview the operation and confirm parameters
2. **Ask the user to confirm** before executing on-chain (e.g. "Do you want to proceed with this transaction?")
3. Only after the user confirms: submit via `onchainos wallet contract-call --chain 56 --force`
4. Report transaction hash and BSCScan link

## Security Warnings

- **Sell tax risk**: Tokens with `sellTaxRate > 500 bps (5%)` display a warning. Tax rates up to 10000 bps (100%) are possible on TOKEN_TAXED_V3 tokens, which is a de-facto rug mechanism.
- **DEX graduation**: When `status == DEX (4)`, the bonding curve is closed. Use `onchainos dex swap` for graduated tokens.
- **Slippage**: Default slippage is 5% (500 bps). For volatile tokens, increase with `--slippage-bps`.

---

## Operations

### get-token-info — Query bonding curve state

Fetches token status, price, reserve, circulating supply, and tax rates from Portal.

```bash
flap get-token-info --token 0xAbCd...
flap get-token-info --token 0xAbCd... --rpc-url https://bsc-dataseed.binance.org/
```

**Parameters:**
- `--token` (required): Token contract address (0x-prefixed)
- `--rpc-url` (optional): BSC RPC URL (default: https://bsc-rpc.publicnode.com)

**Output fields:**
- `status`: Tradable / DEX (graduated) / Staged / Invalid
- `status_code`: 1=Tradable, 4=DEX, 5=Staged, 0=Invalid
- `price_wei_per_token`: Current bonding curve price
- `circulating_supply`: Tokens in circulation
- `reserve_bnb`: BNB in bonding curve reserve
- `buy_tax_rate_bps`, `sell_tax_rate_bps`: Tax rates in basis points
- `bonding_progress_pct`: Progress toward graduation (16 BNB threshold on BSC)
- `dex_pool`: DEX pool address (once graduated)
- `warning`: Sell tax warning or DEX graduation notice

---

### create-token — Launch a new token on Flap

Creates a standard ERC-20 token or a tax token (V3) on the Flap bonding curve.

**Always run `--dry-run` first and ask user to confirm before executing on-chain.**

```bash
# Step 1: Preview (always do this first)
flap create-token \
  --name "Moon Hamster" --symbol "MHAMS" \
  --meta "QmXxx..." \
  --dry-run

# Step 2: Ask user "Do you want to create this token on BSC? This will cost gas."
# Step 3: Execute only after user confirms
flap create-token \
  --name "Moon Hamster" --symbol "MHAMS" \
  --meta "QmXxx..."
```

```bash
# Preview tax token (1% buy / 2% sell tax)
flap create-token \
  --name "Tax Cat" --symbol "TCAT" \
  --meta "QmXxx..." \
  --buy-tax-bps 100 --sell-tax-bps 200 \
  --beneficiary 0xYourAddress... \
  --dry-run

# Ask user confirmation, then execute after approval
flap create-token \
  --name "Tax Cat" --symbol "TCAT" \
  --meta "QmXxx..." \
  --buy-tax-bps 100 --sell-tax-bps 200 \
  --beneficiary 0xYourAddress...
```

**Parameters:**
- `--name` (required): Token name
- `--symbol` (required): Token ticker symbol
- `--meta` (optional): IPFS CID or metadata string (upload via https://funcs.flap.sh/api/upload)
- `--buy-tax-bps` (optional): Buy tax in bps (default: 0; >0 creates TOKEN_TAXED_V3)
- `--sell-tax-bps` (optional): Sell tax in bps (default: 0)
- `--beneficiary` (optional): Tax recipient address (required if tax > 0)
- `--tax-duration` (optional): Tax duration in seconds (default: 0 = permanent)
- `--anti-farmer-duration` (optional): Anti-farmer lock in seconds (default: 0 = disabled)
- `--initial-buy-wei` (optional): BNB wei for initial buy after creation (default: 0)
- `--skip-salt-grind` (optional): Skip vanity address grinding (token will not have 8888/7777 suffix)
- `--dex-id` (optional): DEX for graduation: 0=PancakeSwap V3 (default), 1=PancakeSwap V2
- `--dry-run` (optional): Preview without broadcasting

**Output fields:**
- `predicted_token_address`: CREATE2-predicted token contract address
- `salt_hex`: The salt used for CREATE2
- `salt_iterations`: Number of iterations to find vanity suffix
- `token_version`: 1=standard, 6=tax V3
- `tx_hash`: Transaction hash after creation

**Limitation:** The plugin does not upload token metadata/images. Upload the image and metadata JSON to `https://funcs.flap.sh/api/upload` separately and pass the returned IPFS CID as `--meta`.

---

### buy — Buy tokens from bonding curve

Purchases tokens using BNB. Quotes first, then calls `swapExactInput` with BNB value.

**Always run `--dry-run` first and ask user to confirm before executing on-chain.**

```bash
# Step 1: Preview
flap buy --token 0xAbCd... --bnb-amount 100000000000000000 --dry-run

# Step 2: Ask user "Do you want to buy with 0.1 BNB? This will execute on BSC."
# Step 3: Execute only after user confirms
flap buy --token 0xAbCd... --bnb-amount 100000000000000000
```

```bash
# With custom slippage (2%)
flap buy --token 0xAbCd... --bnb-amount 100000000000000000 --slippage-bps 200
```

**Parameters:**
- `--token` (required): Token contract address (0x-prefixed)
- `--bnb-amount` (required): BNB amount in wei (e.g. `100000000000000000` = 0.1 BNB)
- `--slippage-bps` (optional): Slippage tolerance in bps (default: 500 = 5%)
- `--rpc-url` (optional): BSC RPC URL
- `--dry-run` (optional): Preview without broadcasting

**Output fields:**
- `expected_tokens_out`: Quoted token output (before slippage)
- `min_tokens_out`: Minimum tokens enforced on-chain (after slippage)
- `tx_hash`, `bscscan_tx_url`

**Note:** If token status is DEX (4), buy is refused and user is directed to DEX.

---

### sell — Sell tokens back to bonding curve

Sells tokens for BNB. Automatically approves Portal before selling.

**Always run `--dry-run` first and ask user to confirm before executing on-chain.**

```bash
# Step 1: Preview
flap sell --token 0xAbCd... --token-amount 1000000000000000000 --dry-run

# Step 2: Ask user "Do you want to sell these tokens for BNB? This will execute on BSC."
# Step 3: Execute only after user confirms
flap sell --token 0xAbCd... --token-amount 1000000000000000000
```

**Parameters:**
- `--token` (required): Token contract address (0x-prefixed)
- `--token-amount` (required): Token units to sell (in smallest units, e.g. wei for 18-decimal tokens)
- `--slippage-bps` (optional): Slippage tolerance in bps (default: 500 = 5%)
- `--rpc-url` (optional): BSC RPC URL
- `--force` (optional): Skip sell tax warning
- `--dry-run` (optional): Preview without broadcasting

**Output fields:**
- `expected_bnb_out_wei`, `expected_bnb_out`: Quoted BNB output
- `min_bnb_out_wei`: Minimum BNB enforced on-chain
- `approve_tx_hash`: ERC-20 approval transaction hash
- `sell_tx_hash`, `bscscan_sell_tx_url`
- `warning`: Sell tax warning if applicable

**Note:** Sell executes two transactions after user confirmation: (1) ERC-20 approve, (2) swapExactInput. Both submitted via `onchainos wallet contract-call --chain 56 --force`. A 3-second delay is added between them. Confirm with the user before starting.

---

## Configuration Defaults

| Parameter | Default | Description |
|-----------|---------|-------------|
| `slippage_bps` | 500 | 5% slippage tolerance |
| `rpc_url` | https://bsc-rpc.publicnode.com | BSC RPC for eth_call reads |
| `portal_address` | 0xe2cE6ab80874Fa9Fa2aAE65D277Dd6B8e65C9De0 | Flap Portal proxy (BSC) |

## Known Limitations

1. **Metadata upload not implemented**: The plugin does not upload images or metadata to Flap's IPFS endpoint. Upload manually and pass the CID as `--meta`.
2. **Vanity salt grinding**: CREATE2 salt grinding (~65k iterations, typically less than 1 second) is synchronous and runs before the transaction is built.
3. **Token address post-creation**: The exact token address is predicted via CREATE2 before sending. Verify on BSCScan after the transaction.
4. **quoteExactInput may fail**: If the bonding curve has very low liquidity, the quote call may revert. In that case, `expected_tokens_out` will be 0 and `min_tokens_out` will be 0 (no slippage protection).
