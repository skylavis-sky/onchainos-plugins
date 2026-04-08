# pump-fun Plugin Design

## §0 Plugin Meta

| Field | Value |
|-------|-------|
| plugin_name | pump-fun |
| dapp_name | pump.fun |
| target_chains | [501] (Solana mainnet) |
| target_protocols | Launchpad, Bonding Curve DEX |
| plugin_version | 0.1.0 |
| onchainos_broadcast | Yes |

---

## §1 接入可行性调研 (Feasibility Research)

| 检查项 | 结果 |
|--------|------|
| 有 Rust SDK？ | **Yes** — `pumpfun` crate v4.6.0 on crates.io: https://crates.io/crates/pumpfun ; source: https://github.com/nhuxhr/pumpfun-rs |
| SDK 支持哪些技术栈？ | Rust (primary, `pumpfun` crate); TypeScript (community); Python (community). Official Rust crate is the canonical choice. |
| 有 REST API？ | **No official API.** Third-party options: QuickNode Metis add-on (`/pump-fun/quote`, `/pump-fun/swap` — paid tier), Bitquery GraphQL (read-only analytics), Moralis Pump.fun API (read-only). For on-chain reads, query Solana RPC directly via the Rust SDK. |
| 有官方 Skill？ | **No** — pump.fun has no official MCP/plugin skill. |
| 开源社区有类似 Skill？ | **Yes (5+)**: `pumpfun-mcp-server` (https://github.com/noahgsolomon/pumpfun-mcp-server) — implements get-token-info, create, buy, sell; `PUMPFUN-MCP` (https://github.com/eskayML/PUMPFUN-MCP); `pumpfun-wallets-mcp` (https://github.com/kukapay/pumpfun-wallets-mcp) — wallet analytics; `pumpfun-dune-mcp` (https://github.com/yance-zhang/pumpfun-dune-mcp). |
| 支持哪些链？ | **Solana only** (chain ID 501). No EVM support. |
| 是否需要 onchainos 广播？ | **Yes** — All write operations (buy, sell, create token) are on-chain Solana transactions. The Rust SDK builds and signs transactions; the plugin extracts the serialized transaction bytes and submits via `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`. |

### 接入路径 (Integration Path)

**路径：SDK (Rust crate `pumpfun`) + 参考社区 Skill**

Community skills (especially `pumpfun-mcp-server`) confirm the operation set. The `pumpfun` Rust crate is the primary integration method:
- **Read ops**: call `PumpFun::get_bonding_curve_account()` → deserialize `BondingCurveAccount` → compute price via `get_buy_price()` / `get_sell_price()`.
- **Write ops**: use `get_buy_instructions()` / `get_sell_instructions()` to obtain `Vec<Instruction>` → build a `VersionedTransaction` → serialize to base64 → submit via `onchainos wallet contract-call --chain 501 --unsigned-tx <base64_tx>`.

**Key constraint:** Solana blockhash expires in ~60 seconds. The serialized transaction must be submitted to onchainos immediately after construction.

---

## §2 操作接口映射 (Operation Interface Mapping)

### 2a. 操作清单

| # | Operation | Type | Chain |
|---|-----------|------|-------|
| 1 | get-token-info | Read (off-chain + on-chain RPC) | Solana 501 |
| 2 | get-price | Read (on-chain RPC) | Solana 501 |
| 3 | buy | Write (on-chain tx) | Solana 501 |
| 4 | sell | Write (on-chain tx) | Solana 501 |
| 5 | create-token | Write (on-chain tx) | Solana 501 |

---

### 2b. 链下查询操作 (Read Operations)

#### Operation 1: get-token-info

Fetches on-chain bonding curve state for a token mint address.

**SDK Method:**
```rust
// Instantiate client
let pumpfun = PumpFun::new(Arc::new(payer_keypair), cluster);

// Query bonding curve account
let curve: BondingCurveAccount = pumpfun
    .get_bonding_curve_account(&mint_pubkey)
    .await?;
```

**Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| mint | Pubkey (base58 string) | Token mint address |

**BondingCurveAccount Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| virtual_token_reserves | u64 | Virtual token reserves for price calc |
| virtual_sol_reserves | u64 | Virtual SOL reserves for price calc |
| real_token_reserves | u64 | Actual token reserves in the curve |
| real_sol_reserves | u64 | Actual SOL in the curve |
| token_total_supply | u64 | Total token supply |
| complete | bool | True if bonding curve is complete (migrated to Raydium/PumpSwap) |
| creator | Pubkey | Token creator's address |

**Derived Output:**
- `get_market_cap_sol()` — current market cap in SOL
- `get_final_market_cap_sol(fee_basis_points)` — projected final market cap
- Token price in SOL = `virtual_sol_reserves / virtual_token_reserves`

---

#### Operation 2: get-price

Calculates the buy or sell price for a given amount.

**SDK Methods:**
```rust
// Get buy price: how many tokens for X lamports of SOL
let tokens_out: u64 = curve.get_buy_price(sol_amount_lamports)?;

// Get sell price: how many lamports SOL for X tokens
let sol_out: u64 = curve.get_sell_price(token_amount, fee_basis_points)?;

// Get current market cap in SOL
let market_cap: u64 = curve.get_market_cap_sol();
```

**Parameters:**

| Param | Type | Description |
|-------|------|-------------|
| mint | Pubkey | Token mint address |
| direction | "buy" \| "sell" | Trade direction |
| amount | u64 | SOL lamports (buy) or token units (sell) |
| fee_basis_points | u64 | Fee in basis points (default: 100 = 1%) |

**Response Fields:**

| Field | Type | Description |
|-------|------|-------------|
| price_sol | f64 | Current token price in SOL |
| amount_out | u64 | Tokens received (buy) or SOL received (sell) |
| amount_out_ui | f64 | Human-readable amount |
| market_cap_sol | u64 | Current market cap in SOL |
| bonding_complete | bool | Whether curve migration is complete |

---

### 2c. 链上写操作 (Write Operations)

#### Solana Write Operation Pattern

Solana has no calldata. All write operations follow this pattern:

```
1. Build instructions via SDK (get_buy_instructions / get_sell_instructions)
2. Fetch latest blockhash from Solana RPC
3. Assemble VersionedTransaction with instructions + blockhash
4. Serialize to base64 bytes
5. Submit: onchainos wallet contract-call --chain 501 --to <PROGRAM_ID> --unsigned-tx <base64_tx>
```

**Note:** There are no EVM function selectors. The Rust SDK handles all instruction encoding internally.

---

#### Operation 3: buy

Purchases tokens on a pump.fun bonding curve.

**SDK Methods:**
```rust
// Option A — full transaction (SDK signs and broadcasts internally, NOT for our use)
// pumpfun.buy(mint, sol_amount, track_volume, slippage_bps, priority_fee).await?;

// Option B — get instructions only (preferred: we build and submit via onchainos)
let instructions: Vec<Instruction> = pumpfun
    .get_buy_instructions(
        mint_pubkey,          // Pubkey: token mint
        amount_sol,           // u64: SOL in lamports
        track_volume,         // Option<bool>: track volume flag
        slippage_bps,         // Option<u64>: slippage in basis points (e.g. 100 = 1%)
    )
    .await?;
```

**Build and submit transaction:**
```rust
// 1. Resolve payer wallet address
let payer_pubkey = resolve_wallet_solana()?;   // from onchainos

// 2. Fetch latest blockhash
let blockhash = rpc_client.get_latest_blockhash().await?;

// 3. Build VersionedTransaction
let message = VersionedMessage::Legacy(Message::new(&instructions, Some(&payer_pubkey)));
let tx = VersionedTransaction::try_new(message, &[&payer_keypair])?;

// 4. Serialize to base64
let serialized = base64::encode(bincode::serialize(&tx)?);

// 5. Submit via onchainos (IMMEDIATELY — blockhash expires in ~60s)
onchainos wallet contract-call \
  --chain 501 \
  --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P \
  --unsigned-tx <serialized>
```

**onchainos command:**
```bash
onchainos wallet contract-call \
  --chain 501 \
  --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P \
  --unsigned-tx <base64_serialized_versioned_transaction>
```

**Input Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| mint | String (base58) | Yes | Token mint address |
| sol_amount | u64 | Yes | SOL amount in lamports (e.g. 100000000 = 0.1 SOL) |
| slippage_bps | u64 | No | Slippage tolerance in basis points (default: 100) |
| priority_fee_unit_limit | u32 | No | Compute unit limit (default: 200000) |
| priority_fee_unit_price | u64 | No | Micro-lamports per compute unit (default: 1000) |

**Program Address:** `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P` (pump.fun main program — Solana mainnet, hardcoded in crate constants)

**onchainos result:**
```json
{ "ok": true, "data": { "txHash": "<solana_signature>" } }
```
Extract: `result["data"]["txHash"]`

---

#### Operation 4: sell

Sells tokens back to the bonding curve for SOL.

**SDK Methods:**
```rust
// Get sell instructions (preferred path for onchainos submission)
let instructions: Vec<Instruction> = pumpfun
    .get_sell_instructions(
        mint_pubkey,          // Pubkey: token mint
        token_amount,         // Option<u64>: None = sell all tokens
        slippage_bps,         // Option<u64>: slippage in basis points
    )
    .await?;
```

**Build and submit transaction:** (same pattern as buy — assemble VersionedTransaction, serialize to base64, submit via onchainos)

**onchainos command:**
```bash
onchainos wallet contract-call \
  --chain 501 \
  --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P \
  --unsigned-tx <base64_serialized_versioned_transaction>
```

**Input Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| mint | String (base58) | Yes | Token mint address |
| token_amount | Option<u64> | No | Token units to sell; None = sell all |
| slippage_bps | u64 | No | Slippage tolerance in basis points (default: 100) |
| priority_fee_unit_limit | u32 | No | Compute unit limit (default: 200000) |
| priority_fee_unit_price | u64 | No | Micro-lamports per compute unit (default: 1000) |

---

#### Operation 5: create-token

Deploys a new token on pump.fun with bonding curve.

**SDK Methods:**
```rust
// Option A: Create only
let instructions = pumpfun.get_create_instructions(
    mint_keypair,
    metadata,              // CreateTokenMetadata
    priority_fee,          // Option<PriorityFee>
).await?;

// Option B: Create + initial buy in one tx (create_and_buy)
// pumpfun.create_and_buy(mint, metadata, sol_amount, track_volume, slippage_bps, priority_fee)
```

**CreateTokenMetadata fields:**
```rust
CreateTokenMetadata {
    name: String,
    symbol: String,
    description: String,
    file: String,          // local path to image file OR IPFS URI
    twitter: Option<String>,
    telegram: Option<String>,
    website: Option<String>,
    track_volume: Option<bool>,
}
```

**onchainos command:**
```bash
onchainos wallet contract-call \
  --chain 501 \
  --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P \
  --unsigned-tx <base64_serialized_versioned_transaction>
```

**Input Parameters:**

| Param | Type | Required | Description |
|-------|------|----------|-------------|
| name | String | Yes | Token name (e.g. "My Meme Coin") |
| symbol | String | Yes | Token symbol/ticker (e.g. "MMC") |
| description | String | Yes | Token description |
| image_path | String | Yes | Path or IPFS URI for token image |
| twitter | String | No | Twitter/X URL |
| telegram | String | No | Telegram URL |
| website | String | No | Website URL |
| initial_buy_sol | u64 | No | SOL in lamports to buy immediately after create (0 = no buy) |
| slippage_bps | u64 | No | Slippage for initial buy (default: 100) |

**Note:** The mint keypair must be generated fresh for each token creation. The public key becomes the token's mint address.

---

## §3 用户场景 (User Scenarios)

### Scenario 1: Buy a New Pump.fun Token

**User says:** "Buy 0.1 SOL worth of token PEPE at address `HzLGEj8X...` on pump.fun, max 2% slippage."

**Agent action sequence:**
1. [Read op] Call `get-price` for mint `HzLGEj8X...`:
   - `pumpfun.get_bonding_curve_account(&mint_pubkey)` → get `BondingCurveAccount`
   - Check `curve.complete == false` (bonding curve still active; if true, user must trade on PumpSwap/Raydium instead)
   - Call `curve.get_buy_price(100_000_000)` → calculate tokens out for 0.1 SOL
   - Display to user: "0.1 SOL → ~4,200,000 PEPE tokens at current price; market cap ~12.3 SOL"
2. [Read op] Call `resolve_wallet_solana()` via `onchainos wallet balance --chain 501 --output json` → get payer address
3. [Write op] Call `pumpfun.get_buy_instructions(mint, 100_000_000, None, Some(200))` → Vec<Instruction>
4. [Write op] Fetch latest Solana blockhash via RPC
5. [Write op] Assemble `VersionedTransaction`, serialize to base64
6. [Write op — onchainos] Submit immediately:
   ```
   onchainos wallet contract-call --chain 501 --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P --unsigned-tx <base64_tx>
   ```
7. Extract `result["data"]["txHash"]` → report tx signature to user

---

### Scenario 2: Sell Tokens for SOL

**User says:** "Sell all my PEPE tokens (`HzLGEj8X...`) on pump.fun."

**Agent action sequence:**
1. [Read op] Call `get-token-info` for mint `HzLGEj8X...`:
   - `pumpfun.get_bonding_curve_account(&mint_pubkey)` → check `complete` field
   - Confirm bonding curve is still active
2. [Read op] Check user's token balance via Solana RPC (`getTokenAccountsByOwner`) or derive ATA and fetch balance
3. [Read op] Call `curve.get_sell_price(token_balance, 100)` → estimated SOL out after fees
   - Display: "Selling ~4,200,000 PEPE → estimated ~0.098 SOL (after 1% fee)"
4. [Read op] Call `resolve_wallet_solana()` via onchainos
5. [Write op] Call `pumpfun.get_sell_instructions(mint, None, Some(100))` → `None` means sell all
6. [Write op] Fetch latest Solana blockhash via RPC
7. [Write op] Assemble `VersionedTransaction`, serialize to base64
8. [Write op — onchainos] Submit immediately:
   ```
   onchainos wallet contract-call --chain 501 --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P --unsigned-tx <base64_tx>
   ```
9. Extract `result["data"]["txHash"]` → report to user

---

### Scenario 3: Check Token Price and Bonding Progress

**User says:** "What's the current price of token `HzLGEj8X...` on pump.fun? How close is it to graduation?"

**Agent action sequence:**
1. [Read op] Call `pumpfun.get_bonding_curve_account(&mint_pubkey)`:
   - Retrieve `BondingCurveAccount`
2. [Compute] Calculate derived metrics:
   - Current price (SOL/token) = `virtual_sol_reserves / virtual_token_reserves` (as f64)
   - Current market cap = `curve.get_market_cap_sol()` (in SOL)
   - Final market cap (at graduation) = `curve.get_final_market_cap_sol(100)` (with 1% fee)
   - Graduation progress % = `(real_sol_reserves / 85_000_000_000) * 100` (approx. 85 SOL threshold)
   - `complete` field: whether already migrated to PumpSwap/Raydium
3. [Respond] Format and return:
   ```
   Token: HzLGEj8X...
   Price: 0.0000000238 SOL/token
   Market Cap: 23.8 SOL (~$X USD)
   Bonding Progress: 28% (23.8 / ~85 SOL threshold)
   Status: Active (not yet graduated)
   ```

---

### Scenario 4: Launch a New Token on pump.fun

**User says:** "Create a new token called 'Moon Cat' (symbol: MCAT) with description 'The cats are going to the moon' and upload image from /tmp/cat.png. Buy 0.5 SOL worth immediately."

**Agent action sequence:**
1. [Prepare] Generate fresh mint `Keypair::generate()` → store public key as the new token's mint address
2. [Prepare] Build `CreateTokenMetadata`:
   ```rust
   CreateTokenMetadata {
       name: "Moon Cat".to_string(),
       symbol: "MCAT".to_string(),
       description: "The cats are going to the moon".to_string(),
       file: "/tmp/cat.png".to_string(),
       twitter: None, telegram: None, website: None,
       track_volume: Some(true),
   }
   ```
3. [Read op] Call `resolve_wallet_solana()` via onchainos to get payer
4. [Write op] Call `pumpfun.get_create_and_buy_instructions(mint_keypair, metadata, 500_000_000, None, Some(100))` → Vec<Instruction> (500_000_000 lamports = 0.5 SOL)
5. [Write op] Fetch latest Solana blockhash via RPC
6. [Write op] Assemble `VersionedTransaction` with mint keypair as additional signer, serialize to base64
7. [Write op — onchainos] Submit immediately:
   ```
   onchainos wallet contract-call --chain 501 --to 6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P --unsigned-tx <base64_tx>
   ```
8. Extract tx signature → report to user: "Token MCAT created at mint address `<mint_pubkey>`, tx: `<signature>`"

---

## §4 外部 API 依赖 (External API Dependencies)

| # | Service | Purpose | Auth | Notes |
|---|---------|---------|------|-------|
| 1 | Solana RPC (mainnet-beta) | On-chain reads: `get_bonding_curve_account`, token balances, latest blockhash | None (public) or API key for private RPC | Default: `https://api.mainnet-beta.solana.com` — rate-limited. Use Helius or QuickNode for production. |
| 2 | Helius RPC | High-throughput Solana RPC with priority fee estimates | API key (`HELIUS_RPC_URL`) | Recommended for production; used by `pumpfun-mcp-server` community skill. |
| 3 | QuickNode Metis `/pump-fun/quote` | Get token price quotes with bonding curve data | QuickNode API key (paid add-on) | Optional — can replace direct on-chain BondingCurveAccount reads. Only available on paid Metis tier. |
| 4 | IPFS / Arweave (via SDK utils) | Token image and metadata upload for create-token | None (SDK handles via pump.fun metadata API) | The `pumpfun` crate's `utils` module handles metadata upload to pump.fun's IPFS-backed service. |
| 5 | onchainos CLI | Sign and broadcast Solana transactions | Wallet configured in onchainos | Must be installed and wallet pre-configured |

---

## §5 配置参数 (Configuration Parameters)

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `rpc_url` | String | `"https://api.mainnet-beta.solana.com"` | Solana RPC endpoint. For production use Helius or QuickNode. Set via `HELIUS_RPC_URL` env var or config. |
| `default_slippage_bps` | u64 | `100` | Default slippage tolerance in basis points (100 = 1%). |
| `default_priority_fee_unit_limit` | u32 | `200_000` | Compute unit limit for priority fee. |
| `default_priority_fee_unit_price` | u64 | `1_000` | Micro-lamports per compute unit for priority fee. |
| `fee_basis_points` | u64 | `100` | Fee applied on pump.fun trades (used in sell price calculation). Standard is 1%. |
| `dry_run` | bool | `false` | When `true`, skip onchainos broadcast and return simulated response. No on-chain transactions are submitted. |
| `track_volume` | bool | `true` | Whether to track volume in pump.fun analytics (passed to buy/create instructions). |
| `helius_api_key` | String | `""` | Optional Helius API key for higher-rate RPC access. |
| `quicknode_endpoint` | String | `""` | Optional QuickNode endpoint URL for Metis Pump.fun API (requires paid subscription). |

---

## §6 技术说明 (Technical Notes)

### Solana Transaction Flow (No Calldata)

Unlike EVM chains, Solana has no calldata concept. The integration flow is:

```
pumpfun Rust crate
  → get_buy_instructions() / get_sell_instructions()
  → Vec<Instruction> (Solana instruction objects)
  → Build VersionedTransaction with latest blockhash
  → bincode::serialize() → base64 encode
  → onchainos wallet contract-call --chain 501 --unsigned-tx <base64>
  → onchainos signs with wallet keypair + broadcasts
  → result["data"]["txHash"] = Solana tx signature
```

### Blockhash Expiry Warning

Solana blockhashes expire in approximately 60 seconds (~150 blocks). The plugin must:
1. Fetch blockhash immediately before building the transaction
2. Call onchainos immediately after serialization
3. Never cache or reuse serialized transactions

### Bonding Curve Graduation

When `BondingCurveAccount.complete == true`, the token has graduated from the bonding curve and is now trading on PumpSwap (pump.fun's AMM) or Raydium. In this state:
- `buy` / `sell` via the bonding curve program will fail
- The plugin should detect this and redirect the user to use a DEX swap instead (e.g., `onchainos dex swap execute --chain 501`)

### Program Address

The pump.fun main program address on Solana mainnet is hardcoded in the `pumpfun` crate constants:
- **Pump.fun Program:** `6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P`

This is a well-known, stable address used by all community SDKs and the official program.

### ATA Management

The `pumpfun` crate automatically manages Associated Token Accounts (ATAs):
- `create-ata` feature (default on): creates ATA for the buyer if it doesn't exist
- `close-ata` feature (default on): closes ATA after selling all tokens to recover rent (~0.002 SOL)
