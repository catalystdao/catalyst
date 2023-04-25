use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;
use ethnum::U256;
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};

use crate::state::IbcChannelInfo;

// Implement JsonSchema for U256, see https://graham.cool/schemars/examples/5-remote_derive/
//TODO VERIFY THIS IS CORRECT AND SAFE!
//TODO move to common place
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(remote = "U256")]
pub struct U256Def([u128; 2]);


#[cw_serde]
pub struct InstantiateMsg {
}


#[cw_serde]
pub struct AssetSwapMetadata{
    pub from_amount: Uint128,
    pub from_asset: String,
    pub swap_hash: Vec<u8>,
    pub block_number: u32
}

#[cw_serde]
pub struct LiquiditySwapMetadata{
    pub from_amount: Uint128,
    pub swap_hash: Vec<u8>,
    pub block_number: u32
}

#[cw_serde]
pub enum ExecuteMsg {

    SendCrossChainAsset {
        channel_id: String,
        to_pool: Vec<u8>,
        to_account: Vec<u8>,
        to_asset_index: u8,
        #[serde(with = "U256Def")]
        u: U256,
        #[serde(with = "U256Def")]
        min_out: U256,
        metadata: AssetSwapMetadata,        //TODO do we want this?
        calldata: Vec<u8>
    },

    SendCrossChainLiquidity {
        channel_id: String,
        to_pool: Vec<u8>,
        to_account: Vec<u8>,
        #[serde(with = "U256Def")]
        u: U256,
        #[serde(with = "U256Def")]
        min_out: U256,
        metadata: LiquiditySwapMetadata,    //TODO do we want this?
        calldata: Vec<u8>
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
