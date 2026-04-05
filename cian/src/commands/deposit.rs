use anyhow::Result;
use crate::abi::{encode_approve, encode_optional_deposit, to_raw};
use crate::config::{chain_display_name, MAX_UINT256, ZERO_ADDRESS, VAULTS};
use crate::onchainos::{resolve_wallet, wallet_contract_call, extract_tx_hash, is_ok};

/// Deposit into a CIAN vault: approve token then call optionalDeposit.
///
/// Flow:
///   1. Fetch vault config to confirm capacity
///   2. ERC20 approve(vault, MAX_UINT256) on token contract
///   3. Wait 3s (nonce safety)
///   4. optionalDeposit(_token, _assets, _receiver, _referral=0x0)
pub async fn run(
    chain_id: u64,
    vault_addr: &str,
    token_addr: &str,
    amount: f64,
    decimals: u32,
    dry_run: bool,
) -> Result<()> {
    if amount <= 0.0 {
        anyhow::bail!("Amount must be greater than 0");
    }

    let wallet = resolve_wallet(chain_id).unwrap_or_else(|_| {
        if dry_run {
            "0x0000000000000000000000000000000000000000".to_string()
        } else {
            return "".to_string()
        }
    });
    if !dry_run && wallet.is_empty() {
        anyhow::bail!("Could not resolve wallet address for chain {}. Ensure onchainos supports this chain.", chain_id);
    }
    let raw_amount = to_raw(amount, decimals);
    if raw_amount == 0 {
        anyhow::bail!("Amount is too small (rounds to 0 at {} decimals)", decimals);
    }

    println!("=== CIAN Yield Layer Deposit ===");
    println!("Chain:      {} ({})", chain_id, chain_display_name(chain_id));
    println!("Wallet:     {}", wallet);
    println!("Vault:      {}", vault_addr);
    println!("Token:      {}", token_addr);
    println!("Amount:     {} ({} raw, {} decimals)", amount, raw_amount, decimals);
    if dry_run {
        println!("[DRY-RUN] No transactions will be broadcast.");
    }
    println!();

    // Show vault metadata from registry
    if let Some(meta) = VAULTS.iter().find(|v| {
        v.chain_id == chain_id && v.address.to_lowercase() == vault_addr.to_lowercase()
    }) {
        println!("Vault Name: {}", meta.name);
        println!("Strategy:   {}", meta.strategy);
        println!();
    }

    // Step 1: Approve token -> vault (MAX_UINT256)
    let approve_data = encode_approve(vault_addr, MAX_UINT256);
    println!("Step 1: Approve token -> vault");
    println!("  to:         {}", token_addr);
    println!("  input-data: {}", approve_data);

    let approve_result = wallet_contract_call(chain_id, token_addr, &approve_data, dry_run).await?;

    if !is_ok(&approve_result) && !dry_run {
        anyhow::bail!(
            "Approve failed: {}",
            approve_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let approve_tx = extract_tx_hash(&approve_result);
    println!("  approve tx: {}", approve_tx);
    println!();

    // Wait 3 seconds between approve and deposit (nonce safety)
    if !dry_run {
        println!("Waiting 3 seconds before deposit (nonce safety)...");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }

    // Step 2: optionalDeposit(_token, _assets, _receiver, _referral=0x0)
    let deposit_data = encode_optional_deposit(token_addr, raw_amount, &wallet, ZERO_ADDRESS);
    println!("Step 2: optionalDeposit()");
    println!("  to:         {}", vault_addr);
    println!("  input-data: {}", deposit_data);

    let deposit_result = wallet_contract_call(chain_id, vault_addr, &deposit_data, dry_run).await?;

    if !is_ok(&deposit_result) && !dry_run {
        anyhow::bail!(
            "optionalDeposit failed: {}",
            deposit_result["error"].as_str().unwrap_or("unknown error")
        );
    }
    let deposit_tx = extract_tx_hash(&deposit_result);
    println!("  deposit tx: {}", deposit_tx);
    println!();

    println!("=== Deposit Summary ===");
    println!("Deposited:   {} tokens ({} raw)", amount, raw_amount);
    println!("Vault:       {}", vault_addr);
    println!("Approve tx:  {}", approve_tx);
    println!("Deposit tx:  {}", deposit_tx);
    if dry_run {
        println!("[DRY-RUN] Transactions were simulated, not broadcast.");
    } else {
        println!("yl-tokens will appear in your wallet after on-chain confirmation.");
        println!("Use 'cian get-positions' to check your position.");
    }

    Ok(())
}
