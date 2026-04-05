use crate::config;
use crate::onchainos;

/// Query current apxETH exchange rate and vault statistics.
/// All data fetched on-chain — no external REST API.
pub async fn run() -> anyhow::Result<()> {
    // 1. convertToAssets(1e18) — how much pxETH is 1 apxETH worth
    let one_ether_hex = format!("{:064x}", 1_000_000_000_000_000_000u128);
    let convert_calldata = format!("0x{}{}", config::SEL_CONVERT_TO_ASSETS, one_ether_hex);
    let rate_raw = onchainos::eth_call(config::CHAIN_ID, config::APXETH_VAULT, &convert_calldata)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let apxeth_per_pxeth = rate_raw as f64 / 1e18;

    // 2. totalAssets() — total pxETH in vault
    let total_assets_call = format!("0x{}", config::SEL_TOTAL_ASSETS);
    let total_assets_wei = onchainos::eth_call(config::CHAIN_ID, config::APXETH_VAULT, &total_assets_call)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let total_assets_eth = total_assets_wei as f64 / 1e18;

    // 3. totalSupply() on apxETH — total shares
    let total_supply_call = format!("0x{}", config::SEL_TOTAL_SUPPLY);
    let total_supply_raw = onchainos::eth_call(config::CHAIN_ID, config::APXETH_VAULT, &total_supply_call)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let total_supply = total_supply_raw as f64 / 1e18;

    // 4. pxETH totalSupply — total pxETH minted
    let pxeth_supply_raw = onchainos::eth_call(config::CHAIN_ID, config::PXETH_TOKEN, &total_supply_call)
        .map(|r| onchainos::decode_uint256(&r))
        .unwrap_or(0);
    let pxeth_supply = pxeth_supply_raw as f64 / 1e18;

    // 5. Check PirexEth paused state
    let paused_call = format!("0x{}", config::SEL_PAUSED);
    let is_paused = onchainos::eth_call(config::CHAIN_ID, config::PIREXETH, &paused_call)
        .map(|r| onchainos::decode_bool(&r))
        .unwrap_or(false);

    // Approximate APR: apxETH accumulates yield over time via harvest()
    // Rate > 1.0 means accumulated yield
    // Without time-series data, we show the current rate as a proxy
    let rate_display = if apxeth_per_pxeth > 0.0 {
        format!("{:.8}", apxeth_per_pxeth)
    } else {
        "unavailable".to_string()
    };

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "data": {
                "apxeth_per_pxeth": rate_display,
                "description": "1 apxETH can be redeemed for this many pxETH",
                "total_assets_pxeth": format!("{:.4}", total_assets_eth),
                "total_apxeth_supply": format!("{:.4}", total_supply),
                "total_pxeth_supply": format!("{:.4}", pxeth_supply),
                "pirexeth_deposit_paused": is_paused,
                "protocol_status": if is_paused { "ETH deposits paused; apxETH vault active" } else { "all operations active" },
                "chain": "ethereum",
                "contracts": {
                    "PirexEth": config::PIREXETH,
                    "pxETH": config::PXETH_TOKEN,
                    "apxETH": config::APXETH_VAULT
                }
            }
        })
    );
    Ok(())
}
