use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Addr};

#[cw_serde]
pub struct InstantiateMsg {
    pub gov_contract: String,
    pub default_timeout: u64
}

#[cw_serde]
pub enum ExecuteMsg {

    CrossChainSwap {
        chain_id: String,
        target_pool: [u8; 32],
        target_user: [u8; 32],
        target_asset_index: u8,
        units_x64: [u8; 32],
        min_out: [u8; 32],
        approx: bool,
        source_amount: Uint128,
        source_asset: Addr,
        calldata: Vec<u8>
    },

}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {}
