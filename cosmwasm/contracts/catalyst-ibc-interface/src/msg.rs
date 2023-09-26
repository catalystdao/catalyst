use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Uint128, Binary};
use catalyst_types::U256;

use crate::state::IbcChannelInfo;

#[cw_serde]
pub struct InstantiateMsg {
}


#[cw_serde]
pub enum ExecuteMsg {

    /// Initiate a 'send_asset' cross-chain call.
    /// 
    /// # Arguments: 
    /// * `channel_id` - The target chain identifier.
    /// * `to_vault` - The target vault on the target chain (Catalyst encoded).
    /// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
    /// * `to_asset_index` - The destination asset index.
    /// * `u` - The outgoing 'units'.
    /// * `min_out` - The mininum `to_asset` output amount to get on the target vault.
    /// * `from_amount` - The `from_asset` amount sold to the vault.
    /// * `from_asset` - The source asset.
    /// * `underwrite_incentive_x16` - The share of the swap return that is offered to an underwriter as incentive.
    /// * `block_number` - The block number at which the transaction has been committed.
    /// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
    /// 
    SendCrossChainAsset {
        channel_id: String,
        to_vault: Binary,
        to_account: Binary,
        to_asset_index: u8,
        u: U256,
        min_out: U256,
        from_amount: Uint128,
        from_asset: String,
        underwrite_incentive_x16: u16,
        block_number: u32,
        calldata: Binary
    },


    /// Initiate a 'send_liquidity' cross-chain call.
    /// 
    /// # Arguments: 
    /// * `channel_id` - The target chain identifier.
    /// * `to_vault` - The target vault on the target chain (Catalyst encoded).
    /// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
    /// * `u` - The outgoing 'units'.
    /// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
    /// * `min_reference_asset` - The mininum reference asset value on the target vault.
    /// * `from_amount` - The `from_asset` amount sold to the vault.
    /// * `block_number` - The block number at which the transaction has been committed.
    /// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
    /// 
    SendCrossChainLiquidity {
        channel_id: String,
        to_vault: Binary,
        to_account: Binary,
        u: U256,
        min_vault_tokens: U256,
        min_reference_asset: U256,
        from_amount: Uint128,
        block_number: u32,
        calldata: Binary
    },


    //TODO-UNDERWRITE documentation
    SetMaxUnderwriteDuration {
        new_max_underwrite_duration: Uint64
    },


    //TODO-UNDERWRITE documentation
    Underwrite {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },


    //TODO-UNDERWRITE documentation
    UnderwriteAndCheckConnection {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },


    //TODO-UNDERWRITE documentation
    ExpireUnderwrite {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },



    // Ownership msgs

    /// Transfer the ownership of the interface.
    /// * `new_owner` - The new owner of the contract. Must be a valid address.
    TransferOwnership {
        new_owner: String
    }

}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

    // Get the port id bound by the interface.
    #[returns(PortResponse)]
    Port {},

    // Get a list of the channels that are used by the interface.
    #[returns(ListChannelsResponse)]
    ListChannels {},

    //TODO-UNDERWRITE documentation
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


#[cw_serde]
pub struct UnderwriteIdentifierResponse {
    //TODO-UNDERWRITE documentation
    pub identifier: Binary
}
