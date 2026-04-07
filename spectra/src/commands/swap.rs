use anyhow::Result;
use serde_json::Value;

use crate::config::{rpc_url, router_address, CMD_TRANSFER_FROM, CMD_CURVE_SWAP_SNG, KNOWN_BASE_POOLS};
use crate::onchainos::{
    decode_uint, encode_address, encode_uint256, erc20_approve, eth_call,
    extract_tx_hash_or_err, resolve_wallet, wallet_contract_call,
};

/// Swap PT for IBT (sell PT) OR IBT for PT (buy PT) via Curve StableSwap NG pool.
///
/// Direction:
///   sell_pt=true  → user provides PT,  receives IBT  (i=1 → j=0)
///   sell_pt=false → user provides IBT, receives PT   (i=0 → j=1)
pub async fn run(
    chain_id: u64,
    pt_address: &str,
    amount_in: &str,        // amount to sell, in wei
    min_amount_out: &str,   // min amount to receive, in wei (0 to auto-compute from slippage)
    sell_pt: bool,          // true = sell PT; false = buy PT (sell IBT)
    curve_pool: Option<&str>,
    from: Option<&str>,
    slippage: f64,
    dry_run: bool,
) -> Result<Value> {
    let rpc = rpc_url(chain_id);
    let router = router_address(chain_id);

    // Resolve wallet
    let wallet = if let Some(f) = from {
        f.to_string()
    } else {
        let w = resolve_wallet(chain_id).unwrap_or_default();
        if w.is_empty() {
            anyhow::bail!("Cannot resolve wallet. Pass --from or ensure onchainos is logged in.");
        }
        w
    };

    // Resolve Curve pool address
    let pool_addr = if let Some(p) = curve_pool {
        p.to_string()
    } else {
        let known = KNOWN_BASE_POOLS
            .iter()
            .find(|p| p.pt.to_lowercase() == pt_address.to_lowercase());
        match known {
            Some(k) => k.curve_pool.to_string(),
            None => anyhow::bail!(
                "No known Curve pool for PT {}. Pass --curve-pool explicitly.",
                pt_address
            ),
        }
    };

    // Resolve IBT address
    let ibt_addr = eth_call(rpc, pt_address, "0xc644fe94")
        .await
        .map(|h| crate::onchainos::decode_address(&h))
        .unwrap_or_default();

    // For weETH pool: coins(0)=IBT, coins(1)=PT
    // sell PT  → i=1 (PT), j=0 (IBT)
    // buy  PT  → i=0 (IBT), j=1 (PT)
    let (i_idx, j_idx, token_in_addr) = if sell_pt {
        (1u128, 0u128, pt_address.to_string())
    } else {
        (0u128, 1u128, ibt_addr.clone())
    };

    let amount_u128: u128 = amount_in
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid amount_in: {}", amount_in))?;

    // Estimate min_amount_out if not provided (0 means auto)
    let min_out_u128: u128 = if min_amount_out != "0" {
        min_amount_out
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid min_amount_out: {}", min_amount_out))?
    } else {
        // Use get_dy (Curve) to estimate; fallback to slippage * amount
        // get_dy(uint256 i, uint256 j, uint256 dx) on SNG pool
        // Selector for StableSwap NG: get_dy(uint256,uint256,uint256) = 0x556d6e9f
        // StableSwap NG v7 uses uint256 for i,j (unlike classic StableSwap which uses int128)
        // Try to call it; fall back to a simple slippage estimate
        let get_dy_calldata = format!(
            "0x556d6e9f{}{}{}",
            encode_uint256(i_idx),
            encode_uint256(j_idx),
            encode_uint256(amount_u128)
        );
        let est = eth_call(rpc, &pool_addr, &get_dy_calldata)
            .await
            .map(|h| decode_uint(&h))
            .unwrap_or(0);

        if est > 0 {
            (est as f64 * (1.0 - slippage)) as u128
        } else {
            // Conservative fallback: accept at least 90% of amount (10% slippage cap)
            (amount_u128 as f64 * (1.0 - slippage.max(0.01))).max(1.0) as u128
        }
    };

    // Build Router execute calldata
    // execute(bytes commands, bytes[] inputs) => 0x24856bc3
    //
    // Commands: [TRANSFER_FROM, CURVE_SWAP_SNG]
    //
    // Input 0 (TRANSFER_FROM): abi.encode(address token, uint256 value)
    // Input 1 (CURVE_SWAP_SNG): abi.encode(address pool, uint256 i, uint256 j, uint256 amountIn, uint256 minAmountOut, address recipient)

    let commands_bytes = vec![CMD_TRANSFER_FROM, CMD_CURVE_SWAP_SNG];
    let commands_hex = hex::encode(&commands_bytes);

    // ABI-encode Input 0: (address token, uint256 value)
    let input0 = format!(
        "{}{}",
        encode_address(&token_in_addr),
        encode_uint256(amount_u128)
    );

    // ABI-encode Input 1: (address pool, uint256 i, uint256 j, uint256 amountIn, uint256 minAmountOut, address recipient)
    // Note: amountIn here is the amount the Router has after TRANSFER_FROM, i.e. same as amount_u128
    let input1 = format!(
        "{}{}{}{}{}{}",
        encode_address(&pool_addr),
        encode_uint256(i_idx),
        encode_uint256(j_idx),
        encode_uint256(amount_u128),
        encode_uint256(min_out_u128),
        encode_address(&wallet)
    );

    // ABI-encode execute(bytes, bytes[]) — this is the complex part
    // Signature: execute(bytes commands, bytes[] inputs)
    // Encoding layout:
    //   [0x00] offset to commands (= 0x40)
    //   [0x20] offset to inputs array (= dynamic, after commands)
    //   commands: length + data (padded to 32 bytes)
    //   inputs[]: array length + offset + length + data for each element
    let calldata = build_execute_calldata(&commands_hex, &[&input0, &input1]);

    // Step 1: Approve token_in for Router
    let approve_result = erc20_approve(
        chain_id,
        &token_in_addr,
        router,
        Some(&wallet),
        dry_run,
    )
    .await?;
    let approve_hash = extract_tx_hash_or_err(&approve_result);

    // Step 2: Execute via Router
    let swap_result = wallet_contract_call(
        chain_id,
        router,
        &calldata,
        Some(&wallet),
        None,
        true,
        dry_run,
    )
    .await?;
    let tx_hash = extract_tx_hash_or_err(&swap_result);

    Ok(serde_json::json!({
        "ok": true,
        "operation": if sell_pt { "swap (sell PT -> IBT)" } else { "swap (buy PT, sell IBT)" },
        "chain_id": chain_id,
        "pt": pt_address,
        "ibt": ibt_addr,
        "curve_pool": pool_addr,
        "token_in": token_in_addr,
        "amount_in_raw": amount_in,
        "min_amount_out_raw": min_out_u128.to_string(),
        "slippage": slippage,
        "router": router,
        "wallet": wallet,
        "approve_tx": approve_hash,
        "tx_hash": tx_hash,
        "calldata": calldata,
        "dry_run": dry_run
    }))
}

/// Build ABI-encoded calldata for Router.execute(bytes commands, bytes[] inputs)
/// Selector: 0x24856bc3
fn build_execute_calldata(commands_hex: &str, inputs: &[&str]) -> String {
    // commands_hex: hex-encoded bytes (no 0x prefix)
    // inputs: each entry is a hex-encoded tightly-packed ABI encoding (no 0x)

    let commands_bytes = hex::decode(commands_hex).unwrap_or_default();
    let cmd_len = commands_bytes.len();

    // Layout of execute(bytes commands, bytes[] inputs):
    // offset[0] = pointer to commands bytes = 0x40 (after two 32-byte pointers)
    // offset[1] = pointer to inputs[] = 0x40 + ceil_32(4+cmd_len) ... computed below

    // commands encoding: 32 bytes length + data padded to 32 bytes
    let cmd_len_padded = ((cmd_len + 31) / 32) * 32;
    let commands_slot_size = 32 + cmd_len_padded; // length word + padded data

    // inputs array encoding:
    // 32 bytes: array length (N)
    // N * 32 bytes: offsets (relative to start of array content)
    // each element: 32 bytes length + padded data

    let n = inputs.len();

    // Compute each input's byte-length (raw bytes, not padded yet)
    let input_bytes: Vec<Vec<u8>> = inputs
        .iter()
        .map(|s| hex::decode(s).unwrap_or_default())
        .collect();

    // Pointer to commands = 0x40 (fixed: two slots for the two top-level offsets)
    let ptr_commands: usize = 0x40;

    // Pointer to inputs[] = ptr_commands + commands_slot_size
    let ptr_inputs: usize = ptr_commands + commands_slot_size;

    // Start building the full ABI payload (without selector)
    let mut payload: Vec<u8> = Vec::new();

    // Top-level offset[0]: pointer to commands
    push_u256(&mut payload, ptr_commands as u128);
    // Top-level offset[1]: pointer to inputs[]
    push_u256(&mut payload, ptr_inputs as u128);

    // Write commands bytes
    push_u256(&mut payload, cmd_len as u128); // length
    let mut cmd_padded = commands_bytes.clone();
    pad_to_32(&mut cmd_padded);
    payload.extend_from_slice(&cmd_padded);

    // Write inputs[] array
    // First: array length
    push_u256(&mut payload, n as u128);

    // Then: N offset words (each offset is relative to start of array, i.e. after the length word)
    // Start of first element = N * 32 (after the N offset slots)
    let mut elem_offsets: Vec<usize> = Vec::new();
    let mut running_offset: usize = n * 32; // relative to the array data start
    for ib in &input_bytes {
        elem_offsets.push(running_offset);
        running_offset += 32 + (((ib.len() + 31) / 32) * 32);
    }
    for off in &elem_offsets {
        push_u256(&mut payload, *off as u128);
    }

    // Then: each element as (length, padded data)
    for ib in &input_bytes {
        push_u256(&mut payload, ib.len() as u128);
        let mut padded = ib.clone();
        pad_to_32(&mut padded);
        payload.extend_from_slice(&padded);
    }

    format!("0x24856bc3{}", hex::encode(&payload))
}

fn push_u256(buf: &mut Vec<u8>, val: u128) {
    let mut bytes = [0u8; 32];
    let val_bytes = val.to_be_bytes();
    bytes[16..32].copy_from_slice(&val_bytes);
    buf.extend_from_slice(&bytes);
}

fn pad_to_32(buf: &mut Vec<u8>) {
    let remainder = buf.len() % 32;
    if remainder != 0 {
        buf.resize(buf.len() + (32 - remainder), 0);
    }
}
