use anyhow::Context;
use serde_json::{json, Value};

use crate::abi::{encode_set_approval_for_all, encode_unwind_leveraged_position, format_18};
use crate::config::{CD_POSITION, CHAIN_ID, LEVERAGE_ENGINE, POSITION_TOKEN, RPC_URL};
use crate::onchainos;
use crate::rpc;

/// Close a leveraged position on Archimedes Finance via LeverageEngine.unwindLeveragedPosition.
///
/// Flow:
/// 1. [Read] Verify wallet owns the NFT (PositionToken.ownerOf)
/// 2. [Read] Fetch position total value (CDPosition.getOUSDTotalIncludeInterest)
/// 3. [Read] Check if LeverageEngine is already approved for NFT operations
/// 4. [Write] PositionToken.setApprovalForAll(LeverageEngine, true) — if not already approved
/// 5. Wait 3 seconds
/// 6. [Write] LeverageEngine.unwindLeveragedPosition(tokenId, minReturnedOUSD)
///
/// dry_run: return simulated response without any on-chain execution.
pub async fn run(
    token_id: u128,
    min_return: Option<f64>,
    from: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<Value> {
    let from_addr = if let Some(addr) = from {
        addr.to_string()
    } else {
        onchainos::resolve_wallet().context("Could not resolve wallet address")?
    };

    // ── Step 1: Verify ownership (skipped in dry-run) ────────────────────────
    if !dry_run {
        let owner = rpc::owner_of(POSITION_TOKEN, token_id, RPC_URL)
            .await
            .context("Failed to fetch NFT owner")?;

        if owner.to_lowercase() != from_addr.to_lowercase() {
            anyhow::bail!(
                "Wallet {} does not own PositionToken #{} (owner: {})",
                from_addr,
                token_id,
                owner
            );
        }
    }

    // ── Step 2: Get position total value (best-effort; 0 in dry-run for dummy IDs) ──
    let total_ousd = rpc::get_ousd_total_include_interest(CD_POSITION, token_id, RPC_URL)
        .await
        .unwrap_or(0);

    let lvusd_borrowed = rpc::get_lvusd_borrowed(CD_POSITION, token_id, RPC_URL)
        .await
        .unwrap_or(0);

    // Compute minReturnedOUSD: use provided value or 95% of total (5% slippage buffer)
    let min_returned_ousd = if let Some(min_r) = min_return {
        // User-provided amount in OUSD (18 decimals)
        (min_r * 1e18) as u128
    } else if total_ousd > 0 {
        total_ousd * 95 / 100
    } else {
        0
    };

    // ── Step 3: Check setApprovalForAll ───────────────────────────────────────
    let already_approved =
        rpc::is_approved_for_all(POSITION_TOKEN, &from_addr, LEVERAGE_ENGINE, RPC_URL)
            .await
            .unwrap_or(false);

    if dry_run {
        let approval_calldata =
            encode_set_approval_for_all(LEVERAGE_ENGINE, true).context("Failed to encode setApprovalForAll")?;
        let unwind_calldata =
            encode_unwind_leveraged_position(token_id, min_returned_ousd).context("Failed to encode unwind calldata")?;

        let approval_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {} --force",
            CHAIN_ID, POSITION_TOKEN, approval_calldata, from_addr
        );
        let unwind_cmd = format!(
            "onchainos wallet contract-call --chain {} --to {} --input-data {} --from {} --force",
            CHAIN_ID, LEVERAGE_ENGINE, unwind_calldata, from_addr
        );

        return Ok(json!({
            "ok": true,
            "dryRun": true,
            "tokenId": token_id.to_string(),
            "wallet": from_addr,
            "ousdTotalWithInterest": format_18(total_ousd),
            "lvUSDBorrowed": format_18(lvusd_borrowed),
            "minReturnedOUSD": format_18(min_returned_ousd),
            "alreadyApproved": already_approved,
            "steps": [
                {
                    "step": 1,
                    "action": "setApprovalForAll",
                    "skippable": already_approved,
                    "simulatedCommand": approval_cmd
                },
                {
                    "step": 2,
                    "action": "unwindLeveragedPosition",
                    "simulatedCommand": unwind_cmd
                }
            ]
        }));
    }

    // ── Step 4: setApprovalForAll (if needed) ─────────────────────────────────
    let approval_tx = if !already_approved {
        let approval_calldata = encode_set_approval_for_all(LEVERAGE_ENGINE, true)
            .context("Failed to encode setApprovalForAll")?;
        let approval_result = onchainos::wallet_contract_call(
            CHAIN_ID,
            POSITION_TOKEN,
            &approval_calldata,
            Some(&from_addr),
            false,
        )
        .context("PositionToken.setApprovalForAll() failed")?;
        let tx = onchainos::extract_tx_hash(&approval_result);

        // Wait 3 seconds between approval and unwind to avoid nonce collision
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        Some(tx)
    } else {
        None
    };

    // ── Step 5: unwindLeveragedPosition ───────────────────────────────────────
    let unwind_calldata = encode_unwind_leveraged_position(token_id, min_returned_ousd)
        .context("Failed to encode unwindLeveragedPosition calldata")?;

    let unwind_result = onchainos::wallet_contract_call(
        CHAIN_ID,
        LEVERAGE_ENGINE,
        &unwind_calldata,
        Some(&from_addr),
        false,
    )
    .context("LeverageEngine.unwindLeveragedPosition() failed")?;
    let unwind_tx = onchainos::extract_tx_hash(&unwind_result);

    let mut response = json!({
        "ok": true,
        "dryRun": false,
        "tokenId": token_id.to_string(),
        "wallet": from_addr,
        "ousdTotalWithInterest": format_18(total_ousd),
        "lvUSDBorrowed": format_18(lvusd_borrowed),
        "minReturnedOUSD": format_18(min_returned_ousd),
        "unwindTxHash": unwind_tx,
    });

    if let Some(tx) = approval_tx {
        response["setApprovalTxHash"] = json!(tx);
    } else {
        response["setApprovalTxHash"] = json!(null);
        response["setApprovalSkipped"] = json!(true);
    }

    Ok(response)
}
