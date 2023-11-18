use catalyst_interface_common::msg::UnderwriteIdentifierResponse;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Binary, Uint64};
use catalyst_types::{U256, Bytes32};
use generalised_incentives_common::{msg::EstimateAddtionalCostResponse, state::IncentiveDescription};


#[cw_serde]
pub struct InstantiateMsg {
    pub generalised_incentives: String
}


#[cw_serde]
pub enum ExecuteMsg {

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
        incentive: IncentiveDescription,
    },

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

    
    SetMaxUnderwriteDuration {
        new_max_underwrite_duration: Uint64
    },

    Underwrite {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },

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

    ExpireUnderwrite {
        to_vault: String,
        to_asset_ref: String,
        u: U256,
        min_out: Uint128,
        to_account: String,
        underwrite_incentive_x16: u16,
        calldata: Binary
    },


    ConnectNewChain {
        channel_id: Bytes32,
        remote_interface: Binary,
        remote_gi: Binary
    },

    SetMinGasFor {
        chain_identifier: Bytes32,
        min_gas: Uint64
    },

    SetMinAckGasPrice {
        min_gas_price: Uint128
    },


    // The following message definitions are taken from the GeneralisedIncentives repo
    ReceiveMessage {
        source_identifier: Bytes32,
        message_identifier: Bytes32,
        from_application: Binary,
        message: Binary
    },
    
    ReceiveAck {
        destination_identifier: Bytes32,
        message_identifier: Bytes32,
        acknowledgement: Binary
    },


    // Ownership msgs
    TransferOwnership {
        new_owner: String
    }

}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

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

    #[returns(EstimateAddtionalCostResponse)]
    EstimateAdditionalCost {}

}