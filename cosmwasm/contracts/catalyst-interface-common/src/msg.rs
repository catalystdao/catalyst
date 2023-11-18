use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Uint128, Binary, SubMsg, Empty};
use catalyst_types::{U256, Bytes32};
use generalised_incentives_common::state::IncentiveDescription;

#[cw_serde]
pub struct InstantiateMsg {
}


#[cw_serde]
pub enum ExecuteMsg<T=Empty> {

    /// Initiate a 'send_asset' cross-chain call.
    /// 
    /// **NOTE**: This call is **permissionless**. The recipient of the transaction must validate
    /// the sender of the transaction.
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
    /// * `incentive` - The relaying incentive.
    /// 
    SendCrossChainAsset {
        channel_id: Bytes32,
        to_vault: Binary,
        to_account: Binary,
        to_asset_index: u8,
        u: U256,
        min_out: U256,
        from_amount: Uint128,
        from_asset: String,
        underwrite_incentive_x16: u16,
        block_number: u32,
        calldata: Binary,
        incentive: IncentiveDescription
    },


    /// Initiate a 'send_liquidity' cross-chain call.
    /// 
    /// **NOTE**: This call is **permissionless**. The recipient of the transaction must validate
    /// the sender of the transaction.
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
    /// * `incentive` - The relaying incentive.
    /// 
    SendCrossChainLiquidity {
        channel_id: Bytes32,
        to_vault: Binary,
        to_account: Binary,
        u: U256,
        min_vault_tokens: U256,
        min_reference_asset: U256,
        from_amount: Uint128,
        block_number: u32,
        calldata: Binary,
        incentive: IncentiveDescription
    },


    /// Set the maximum underwriting duration (only applies to new underwrite orders).
    /// 
    /// NOTE: This function checks that the sender of the transaction is the current interface owner.
    /// 
    /// # Arguments:
    /// * `new_max_duration` - The new desired maximum underwriting duration.
    /// 
    SetMaxUnderwriteDuration {
        new_max_underwrite_duration: Uint64
    },


    /// Underwrite an asset swap.
    /// 
    /// **NOTE**: All the arguments passed to this function must **exactly match** those of the
    /// desired swap to be underwritten.
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
    Underwrite {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },


    /// Check the existance of a connection between the destination and the source vault, and
    /// perform an asset underwrite.
    /// 
    /// **NOTE**: All the arguments passed to this function must **exactly match** those of the
    /// desired swap to be underwritten.
    /// 
    /// # Arguments: 
    /// * `channel_id` - The incoming message channel identifier.
    /// * `from_vault` - The source vault on the source chain.
    /// * `to_vault` - The target vault.
    /// * `to_asset_ref` - The destination asset.
    /// * `u` - The underwritten units.
    /// * `min_out` - The mininum `to_asset_ref` output amount to get on the target vault.
    /// * `to_account` - The recipient of the swap.
    /// * `underwrite_incentive_x16` - The underwriting incentive.
    /// * `calldata` - The swap calldata.
    /// 
    UnderwriteAndCheckConnection {
        channel_id: Bytes32,
        from_vault: Binary,
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },


    /// Expire an underwrite and free the escrowed assets.
    /// 
    /// **NOTE**: The underwriter may expire the underwrite at any time. Any other account must wait
    /// until after the expiry time.
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
    ExpireUnderwrite {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },

    
    /// Wrap multiple submessages within a single submessage.
    /// 
    /// ! **IMPORTANT**: This method can only be invoked by the interface itself.
    /// 
    /// # Arguments:
    /// *sub_msgs* - The submessages to wrap into a single submessage.
    /// 
    WrapSubMsgs {
        sub_msgs: Vec<SubMsg<T>>
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
pub enum InterfaceCommonQueryMsg {

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

    //TODO add owner query?

}


#[cw_serde]
pub struct UnderwriteIdentifierResponse {
    // The underwrite identifier.
    pub identifier: Binary
}
