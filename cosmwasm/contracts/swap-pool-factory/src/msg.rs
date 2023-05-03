use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;


#[cw_serde]
pub struct InstantiateMsg {
    pub default_governance_fee: u64
}


#[cw_serde]
pub enum ExecuteMsg {
    DeployVault {
        vault_template_id: u64,
        assets: Vec<String>,
        assets_balances: Vec<Uint128>,
        weights: Vec<u64>,
        amplification: u64,
        pool_fee: u64,
        name: String,
        symbol: String,
        chain_interface: Option<String>
    }
}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

}
