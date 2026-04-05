// CIAN Yield Layer API client (legacy — kept for reference, not used by active commands)
// The CIAN frontend apps do not expose a public REST API; all read ops use on-chain calls via rpc.rs.

use anyhow::Result;
use serde_json::Value;

#[allow(dead_code)]
pub async fn list_vaults(_chain: &str) -> Result<Value> {
    anyhow::bail!("CIAN does not expose a public REST API; use list-vaults (on-chain reads)")
}

#[allow(dead_code)]
pub async fn get_vault_config(_chain: &str, _vault: &str) -> Result<Value> {
    Ok(serde_json::json!({}))
}

#[allow(dead_code)]
pub async fn get_user_position(_chain: &str, _vault: &str, _user: &str) -> Result<Value> {
    anyhow::bail!("CIAN does not expose a public REST API; use get-positions (on-chain reads)")
}
