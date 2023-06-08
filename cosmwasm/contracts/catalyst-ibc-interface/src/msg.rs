use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Binary};
use catalyst_types::U256;

use crate::state::IbcChannelInfo;

#[cw_serde]
pub struct InstantiateMsg {
}


#[cw_serde]
pub enum ExecuteMsg {

    SendCrossChainAsset {
        channel_id: String,
        to_pool: Binary,
        to_account: Binary,
        to_asset_index: u8,
        u: U256,
        min_out: U256,
        from_amount: Uint128,
        from_asset: String,
        block_number: u32,
        calldata: Binary
    },

    SendCrossChainLiquidity {
        channel_id: String,
        to_pool: Binary,
        to_account: Binary,
        u: U256,
        min_pool_tokens: U256,      //TODO EVM mismatch
        min_reference_asset: U256,  //TODO EVM mismatch
        from_amount: Uint128,
        block_number: u32,
        calldata: Binary
    }

}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Get the port id bound by the interface
    #[returns(PortResponse)]
    Port {},
    // Get a list of the channels that are used by the interface
    #[returns(ListChannelsResponse)]
    ListChannels {}
}

#[cw_serde]
pub struct PortResponse {
    pub port_id: String,
}

#[cw_serde]
pub struct ListChannelsResponse {
    pub channels: Vec<IbcChannelInfo>,
}
