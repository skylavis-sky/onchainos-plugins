use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::{encode_approve, encode_zap_in, human_to_minimal};
use crate::config::{ARCH_TOKEN, CHAIN_ID, COORDINATOR, RPC_URL, ZAPPER};
use crate::onchainos;
use crate::rpc;

/// Open a leveraged position on Archimedes Finance via Zapper.zapIn.
///
/// Flow:
/// 1. [Read] Check available leverage (Coordinator.getAvailableLeverage)
/// 2. [Read] Preview required ARCH and expected OUSD (Zapper.previewZapInAmount)
/// 3. [Write] ERC-20 approve stablecoin → Zapper
/// 4. [Write] (If use_arch) ERC-20 approve ARCH → Zapper
/// 5. [Write] Zapper.zapIn(...)
///
/// dry_run: return simulated response without any on-chain execution.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    amount: f64,
    token: &str,
    cycles: u64,
    use_arch: bool,
    max_slippage_bps: u16,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let (stable_addr, decimals) =
        crate::config::resolve_stablecoin(token).context("Unsupported stablecoin")?;

    let stable_amount_raw = human_to_minimal(amount, decimals);

    let from_addr = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet().context("Could not resolve wallet address")?
    };

    // ── Step 1: Check protocol liquidity ─────────────────────────────────────
    let available_leverage = rpc::get_available_leverage(COORDINATOR, RPC_URL)
        .await
        .context("Failed to fetch available leverage")?;

    if available_leverage == 0 {
        anyhow::bail!(
            "Protocol has no available lvUSD leverage. \
             Current available: 0. Try again later or use fewer cycles."
        );
    }

    // ── Step 2: Preview zap amounts ───────────────────────────────────────────
    let (preview_ousd, preview_arch) =
        rpc::preview_zap_in_amount(ZAPPER, stable_amount_raw, cycles, stable_addr, use_arch, RPC_URL)
            .await
            .unwrap_or((0, 0)); // Non-fatal — use 0 as min amounts

    // Use 5% slippage on previewed amounts as min (or 0 to be permissive)
    let arch_min_amount = if preview_arch > 0 {
        preview_arch * 95 / 100
    } else {
        0
    };
    let ousd_min_amount = if preview_ousd > 0 {
        preview_ousd * 95 / 100
    } else {
        0
    };

    if dry_run {
        let approve_calldata = encode_approve(ZAPPER, stable_amount_raw)
            .context("Failed to encode approve calldata")?;
        let zap_calldata = encode_zap_in(
            stable_amount_raw,
            cycles,
            arch_min_amount,
            ousd_min_amount,
            max_slippage_bps,
            stable_addr,
            use_arch,
        )
        .context("Failed to encode zapIn calldata")?;

        let approve_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {} --force",
            CHAIN_ID, stable_addr, approve_calldata, from_addr
        );
        let zap_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {} --force",
            CHAIN_ID, ZAPPER, zap_calldata, from_addr
        );

        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "token": token,
            "tokenAddress": stable_addr,
            "amount": amount,
            "amountRaw": stable_amount_raw.to_string(),
            "cycles": cycles,
            "useUserArch": use_arch,
            "maxSlippageBps": max_slippage_bps,
            "previewOUSDOut": preview_ousd.to_string(),
            "previewARCHNeeded": preview_arch.to_string(),
            "availableLvUSD": available_leverage.to_string(),
            "steps": [
                {
                    "step": 1,
                    "action": "approve stablecoin",
                    "simulatedCommand": approve_cmd
                },
                {
                    "step": 2,
                    "action": "zapIn",
                    "simulatedCommand": zap_cmd
                }
            ]
        }));
    }

    // ── Step 3: Approve stablecoin → Zapper ───────────────────────────────────
    let approve_calldata =
        encode_approve(ZAPPER, stable_amount_raw).context("Failed to encode approve calldata")?;

    let approve_result = onchainos::wallet_contract_call(
        CHAIN_ID,
        stable_addr,
        &approve_calldata,
        Some(&from_addr),
        false,
    )
    .context("ERC-20 approve (stablecoin → Zapper) failed")?;
    let approve_tx = onchainos::extract_tx_hash(&approve_result);

    // ── Step 4: (Optional) Approve ARCH → Zapper ──────────────────────────────
    let arch_approve_tx = if use_arch && preview_arch > 0 {
        let arch_calldata = encode_approve(ZAPPER, preview_arch)
            .context("Failed to encode ARCH approve calldata")?;
        let arch_result = onchainos::wallet_contract_call(
            CHAIN_ID,
            ARCH_TOKEN,
            &arch_calldata,
            Some(&from_addr),
            false,
        )
        .context("ERC-20 approve (ARCH → Zapper) failed")?;
        Some(onchainos::extract_tx_hash(&arch_result))
    } else {
        None
    };

    // Wait 3 seconds between approve and zapIn to avoid nonce collision
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    // ── Step 5: zapIn ──────────────────────────────────────────────────────────
    let zap_calldata = encode_zap_in(
        stable_amount_raw,
        cycles,
        arch_min_amount,
        ousd_min_amount,
        max_slippage_bps,
        stable_addr,
        use_arch,
    )
    .context("Failed to encode zapIn calldata")?;

    let zap_result = onchainos::wallet_contract_call(
        CHAIN_ID,
        ZAPPER,
        &zap_calldata,
        Some(&from_addr),
        false,
    )
    .context("Zapper.zapIn() failed")?;
    let zap_tx = onchainos::extract_tx_hash(&zap_result);

    let mut response = json!({
        "ok": true,
        "dryRun": false,
        "token": token,
        "tokenAddress": stable_addr,
        "amount": amount,
        "amountRaw": stable_amount_raw.to_string(),
        "cycles": cycles,
        "useUserArch": use_arch,
        "maxSlippageBps": max_slippage_bps,
        "previewOUSDOut": preview_ousd.to_string(),
        "previewARCHNeeded": preview_arch.to_string(),
        "availableLvUSD": available_leverage.to_string(),
        "approveTxHash": approve_tx,
        "zapInTxHash": zap_tx,
        "note": "Check transaction receipt for minted PositionToken NFT ID (Transfer event)"
    });

    if let Some(arch_tx) = arch_approve_tx {
        response["archApproveTxHash"] = json!(arch_tx);
    }

    Ok(response)
}
