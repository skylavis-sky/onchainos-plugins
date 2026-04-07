// Solana transaction building helpers — reused from jito plugin pattern
//
// Builds v0 versioned transactions for SPL Stake Pool DepositSol instruction.
// Key facts:
//   - v0 versioned message: prefix byte 0x80, trailing 0x00 (empty address table list)
//   - onchainos rejects legacy-format transactions
//   - Do NOT include CreateATA instruction in the same tx (fails in simulation)

use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};

// ──────────────────────── Message / Instruction types ────────────────────────

pub struct SolanaMessage {
    pub num_required_sigs: u8,
    pub num_readonly_signed: u8,
    pub num_readonly_unsigned: u8,
    pub account_keys: Vec<Vec<u8>>,
    pub recent_blockhash: Vec<u8>,
    pub instructions: Vec<SolanaInstruction>,
}

pub struct SolanaInstruction {
    pub program_id_index: u8,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

impl SolanaMessage {
    /// Serialize to v0 versioned message bytes.
    pub fn serialize(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // v0 versioned prefix
        buf.push(0x80);
        buf.push(self.num_required_sigs);
        buf.push(self.num_readonly_signed);
        buf.push(self.num_readonly_unsigned);

        encode_compact_u16(&mut buf, self.account_keys.len() as u16);
        for key in &self.account_keys {
            buf.extend_from_slice(key);
        }

        buf.extend_from_slice(&self.recent_blockhash);

        encode_compact_u16(&mut buf, self.instructions.len() as u16);
        for ix in &self.instructions {
            buf.push(ix.program_id_index);
            encode_compact_u16(&mut buf, ix.account_indices.len() as u16);
            buf.extend_from_slice(&ix.account_indices);
            encode_compact_u16(&mut buf, ix.data.len() as u16);
            buf.extend_from_slice(&ix.data);
        }

        // v0: empty address lookup table list
        buf.push(0x00);
        buf
    }
}

/// Solana compact-u16 encoding
pub fn encode_compact_u16(buf: &mut Vec<u8>, val: u16) {
    if val <= 0x7f {
        buf.push(val as u8);
    } else if val <= 0x3fff {
        buf.push((val & 0x7f) as u8 | 0x80);
        buf.push(((val >> 7) & 0x7f) as u8);
    } else {
        buf.push((val & 0x7f) as u8 | 0x80);
        buf.push(((val >> 7) & 0x7f) as u8 | 0x80);
        buf.push(((val >> 14) & 0x03) as u8);
    }
}

/// Build an unsigned v0 transaction (1 sig placeholder = 64 zero bytes).
pub fn build_unsigned_transaction(message_bytes: &[u8]) -> Vec<u8> {
    let mut tx = Vec::new();
    encode_compact_u16(&mut tx, 1); // 1 signature
    tx.extend_from_slice(&[0u8; 64]); // placeholder signature
    tx.extend_from_slice(message_bytes);
    tx
}

/// Encode transaction bytes to base64.
pub fn encode_transaction_base64(tx_bytes: &[u8]) -> String {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    STANDARD.encode(tx_bytes)
}

// ──────────────────────── PDA derivation ────────────────────────

fn bs58_decode(s: &str) -> Result<Vec<u8>> {
    bs58::decode(s)
        .into_vec()
        .map_err(|e| anyhow!("Invalid base58 address '{}': {}", s, e))
}

/// Solana find_program_address — iterates nonce 255..=0, returns first off-curve hash.
fn find_program_address(seeds: &[&[u8]], program_id: &[u8]) -> Result<Vec<u8>> {
    for nonce in (0u8..=255).rev() {
        let mut all_seeds: Vec<&[u8]> = seeds.to_vec();
        all_seeds.push(std::slice::from_ref(&nonce));
        let hash = create_program_address_hash(&all_seeds, program_id);
        if !is_on_ed25519_curve(&hash) {
            return Ok(hash.to_vec());
        }
    }
    Err(anyhow!("Could not find valid PDA for given seeds"))
}

fn create_program_address_hash(seeds: &[&[u8]], program_id: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for seed in seeds {
        hasher.update(seed);
    }
    hasher.update(program_id);
    hasher.update(b"ProgramDerivedAddress");
    hasher.finalize().into()
}

fn is_on_ed25519_curve(bytes: &[u8; 32]) -> bool {
    use curve25519_dalek::edwards::CompressedEdwardsY;
    CompressedEdwardsY(*bytes).decompress().is_some()
}

/// Derive withdraw authority PDA for an SPL stake pool.
/// PDA = find_program_address([pool_addr_bytes, b"withdraw"], STAKE_POOL_PROGRAM)
pub fn derive_withdraw_authority(pool_addr_b58: &str) -> Result<Vec<u8>> {
    let pool = bs58_decode(pool_addr_b58)?;
    let stake_pool_program = bs58_decode(crate::config::STAKE_POOL_PROGRAM)?;
    find_program_address(&[&pool, b"withdraw"], &stake_pool_program)
}

/// Derive Associated Token Account (ATA) address.
/// ATA PDA = find_program_address([owner, token_program, mint], ATA_PROGRAM)
pub fn derive_ata(owner_b58: &str, mint_b58: &str) -> Result<Vec<u8>> {
    let owner = bs58_decode(owner_b58)?;
    let mint = bs58_decode(mint_b58)?;
    let token_program = bs58_decode(crate::config::TOKEN_PROGRAM)?;
    let ata_program = bs58_decode(crate::config::ASSOCIATED_TOKEN_PROGRAM)?;
    find_program_address(&[&owner, &token_program, &mint], &ata_program)
}

// ──────────────────────── SPL Stake Pool DepositSol ────────────────────────

/// Parsed stake pool state (from on-chain account data, offsets per SPL Stake Pool v0.7).
pub struct StakePoolInfo {
    pub reserve_stake: Vec<u8>,       // bytes 130..162
    pub pool_mint: Vec<u8>,           // bytes 162..194
    pub manager_fee_account: Vec<u8>, // bytes 194..226
    pub total_lamports: u64,
    pub pool_token_supply: u64,
}

/// Parse SPL Stake Pool account data (must be >= 298 bytes).
pub fn parse_stake_pool(data: &[u8]) -> Result<StakePoolInfo> {
    if data.len() < 298 {
        return Err(anyhow!("Stake pool account data too short: {} bytes", data.len()));
    }
    let reserve_stake = data[130..162].to_vec();
    let pool_mint = data[162..194].to_vec();
    let manager_fee_account = data[194..226].to_vec();
    let total_lamports = u64::from_le_bytes(data[258..266].try_into().unwrap());
    let pool_token_supply = u64::from_le_bytes(data[266..274].try_into().unwrap());

    Ok(StakePoolInfo {
        reserve_stake,
        pool_mint,
        manager_fee_account,
        total_lamports,
        pool_token_supply,
    })
}

/// Build a DepositSol transaction for the given stake pool.
///
/// Account layout in message (following SPL Stake Pool v0.7 + jito reference):
///
/// Writable + signer: [0] wallet
/// Writable non-signer: [1] stake_pool, [2] reserve_stake, [3] user_token_account,
///                      [4] manager_fee_account, [5] pool_mint
/// Readonly non-signer: [6] withdraw_authority, [7] system_program,
///                      [8] token_program, [9] stake_pool_program
///
/// Returns base64-encoded unsigned v0 transaction.
pub fn build_deposit_sol_transaction(
    wallet_b58: &str,
    stake_pool_b58: &str,
    pool_info: &StakePoolInfo,
    user_token_account_bytes: &[u8],
    blockhash_bytes: &[u8],
    lamports: u64,
) -> Result<String> {
    let wallet_bytes = bs58_decode(wallet_b58)?;
    let stake_pool_bytes = bs58_decode(stake_pool_b58)?;
    let system_program_bytes = bs58_decode(crate::config::SYSTEM_PROGRAM)?;
    let token_program_bytes = bs58_decode(crate::config::TOKEN_PROGRAM)?;
    let stake_pool_program_bytes = bs58_decode(crate::config::STAKE_POOL_PROGRAM)?;
    let withdraw_authority_bytes = derive_withdraw_authority(stake_pool_b58)?;

    let account_keys: Vec<Vec<u8>> = vec![
        wallet_bytes,                         // 0: wallet (signer, writable)
        stake_pool_bytes,                     // 1: stake_pool (writable)
        pool_info.reserve_stake.clone(),      // 2: reserve_stake (writable)
        user_token_account_bytes.to_vec(),    // 3: user LST token account (writable)
        pool_info.manager_fee_account.clone(),// 4: manager_fee_account (writable)
        pool_info.pool_mint.clone(),          // 5: pool_mint (writable)
        withdraw_authority_bytes,             // 6: withdraw_authority (readonly)
        system_program_bytes,                 // 7: system_program (readonly)
        token_program_bytes,                  // 8: token_program (readonly)
        stake_pool_program_bytes,             // 9: stake_pool_program (readonly)
    ];

    // Header: 1 signer, 0 readonly-signed, 4 readonly-unsigned
    let num_required_sigs = 1u8;
    let num_readonly_signed = 0u8;
    let num_readonly_unsigned = 4u8; // indices 6,7,8,9

    // DepositSol instruction: discriminator=14, lamports as u64 LE
    let mut deposit_data = vec![14u8];
    deposit_data.extend_from_slice(&lamports.to_le_bytes());

    let deposit_sol_ix = SolanaInstruction {
        program_id_index: 9, // stake_pool_program
        account_indices: vec![
            1, // stake_pool
            6, // withdraw_authority
            2, // reserve_stake
            0, // wallet (from / signer)
            3, // user LST token account (dest)
            4, // manager_fee_account
            3, // referrer_fee = same as dest
            5, // pool_mint
            7, // system_program
            8, // token_program
        ],
        data: deposit_data,
    };

    let message = SolanaMessage {
        num_required_sigs,
        num_readonly_signed,
        num_readonly_unsigned,
        account_keys,
        recent_blockhash: blockhash_bytes.to_vec(),
        instructions: vec![deposit_sol_ix],
    };

    let msg_bytes = message.serialize();
    let tx_bytes = build_unsigned_transaction(&msg_bytes);
    Ok(encode_transaction_base64(&tx_bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_withdraw_authority_jito() {
        // Known-good PDA for Jito stake pool (verified on mainnet)
        let result = derive_withdraw_authority("Jito4APyf642JPZPx3hGc6WWJ8zPKtRbRs4P815Awbb").unwrap();
        let addr = bs58::encode(&result).into_string();
        assert_eq!(
            addr, "6iQKfEyhr3bZMotVkW6beNZz5CPAkiwvgV2CTje9pVSS",
            "Withdraw authority PDA mismatch: got {}", addr
        );
    }
}
