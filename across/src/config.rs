/// Across API base URL
pub const ACROSS_API_BASE: &str = "https://app.across.to/api";

/// SpokePool addresses (fallback; prefer API-returned spokePoolAddress)
pub fn get_spoke_pool(chain_id: u64) -> &'static str {
    match chain_id {
        1     => "0x5c7BCd6E7De5423a257D81B442095A1a6ced35C5",  // Ethereum
        10    => "0x6f26Bf09B1C792e3228e5467807a900A503c0281",  // Optimism
        137   => "0x9295ee1d8C5b022Be115A2AD3c30C72E34e7F096",  // Polygon
        8453  => "0x09aea4b2242abC8bb4BB78D537A67a245A7bEC64",  // Base
        42161 => "0xe35e9842fceaCA96570B734083f4a58e8F7C5f2A",  // Arbitrum
        _     => "",
    }
}

/// ETH native token placeholder address (EVM convention)
pub const ETH_ADDRESS: &str = "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE";

/// Status polling: max retries and interval (seconds)
pub const STATUS_MAX_RETRIES: u32 = 12;
pub const STATUS_POLL_INTERVAL_SECS: u64 = 5;

/// approve -> deposit wait time (seconds)
pub const APPROVE_DELAY_SECS: u64 = 3;
