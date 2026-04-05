use crate::config;
use crate::onchainos;
use clap::Args;

#[derive(Args)]
pub struct PositionsArgs {
    /// Wallet address to query (defaults to current logged-in wallet)
    #[arg(long)]
    pub address: Option<String>,

    /// Chain ID (only Ethereum mainnet supported)
    #[arg(long, default_value = "1")]
    pub chain: u64,
}

/// Query pxETH and apxETH balances for a wallet.
pub async fn run(args: PositionsArgs) -> anyhow::Result<()> {
    let wallet = if let Some(addr) = args.address {
        addr
    } else {
        onchainos::resolve_wallet(args.chain)?
    };

    if wallet.is_empty() {
        anyhow::bail!("Cannot resolve wallet address. Pass --address or ensure onchainos is logged in.");
    }

    let wallet_clean = wallet.trim_start_matches("0x");
    let wallet_padded = format!("{:0>64}", wallet_clean);

    // Query pxETH balance
    let pxeth_calldata = format!("0x{}{}", config::SEL_BALANCE_OF, wallet_padded);
    let pxeth_raw = onchainos::eth_call(config::CHAIN_ID, config::PXETH_TOKEN, &pxeth_calldata)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let pxeth_balance = pxeth_raw as f64 / 1e18;

    // Query apxETH balance
    let apxeth_calldata = format!("0x{}{}", config::SEL_BALANCE_OF, wallet_padded);
    let apxeth_raw = onchainos::eth_call(config::CHAIN_ID, config::APXETH_VAULT, &apxeth_calldata)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let apxeth_balance = apxeth_raw as f64 / 1e18;

    // Convert apxETH balance to pxETH value using convertToAssets
    let apxeth_pxeth_value = if apxeth_raw > 0 {
        let shares_hex = format!("{:064x}", apxeth_raw);
        let convert_calldata = format!("0x{}{}", config::SEL_CONVERT_TO_ASSETS, shares_hex);
        let pxeth_value_raw = onchainos::eth_call(config::CHAIN_ID, config::APXETH_VAULT, &convert_calldata)
            .map(|r| onchainos::decode_uint256(&r))
            .unwrap_or(0);
        pxeth_value_raw as f64 / 1e18
    } else {
        0.0
    };

    let total_pxeth_equivalent = pxeth_balance + apxeth_pxeth_value;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "wallet": wallet,
                "pxETH": {
                    "balance": format!("{:.8}", pxeth_balance),
                    "contract": config::PXETH_TOKEN
                },
                "apxETH": {
                    "balance": format!("{:.8}", apxeth_balance),
                    "pxeth_value": format!("{:.8}", apxeth_pxeth_value),
                    "contract": config::APXETH_VAULT
                },
                "total_pxeth_equivalent": format!("{:.8}", total_pxeth_equivalent),
                "chain": "ethereum"
            }
        })
    );
    Ok(())
}
