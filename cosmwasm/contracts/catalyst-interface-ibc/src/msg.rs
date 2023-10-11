use catalyst_interface_common::msg::UnderwriteIdentifierResponse;
use catalyst_types::U256;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Binary};

use crate::state::IbcChannelInfo;


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

    // Get the port id bound by the interface.
    #[returns(PortResponse)]
    Port {},

    // Get a list of the channels that are used by the interface.
    #[returns(ListChannelsResponse)]
    ListChannels {},

    // Get the underwriting identifier of the provided underwrite parameters.
    /// 
    /// # Arguments:
    /// * `to_vault` - The target vault.
    /// * `to_asset_ref` - The destination asset.
    /// * `u` - The underwritten units.
    /// * `min_out` - The mininum `to_asset_ref` output amount to get on the target vault.
    /// * `to_account` - The recipient of the swap.
    /// * `underwrite_incentive_x16` - The underwriting incentive.
    /// * `calldata` - The swap calldata.
    /// 
    #[returns(UnderwriteIdentifierResponse)]
    UnderwriteIdentifier {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },

}

#[cw_serde]
pub struct PortResponse {
    // The port id used by the interface.
    pub port_id: String
}

#[cw_serde]
pub struct ListChannelsResponse {
    // List of the channels used by the interface.
    pub channels: Vec<IbcChannelInfo>
}