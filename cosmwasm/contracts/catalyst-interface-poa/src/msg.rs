use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Binary, Uint64};
use catalyst_types::{U256, Bytes32};


#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String
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
        from_asset_ref: String,
        underwrite_incentive_x16: u16,
        block_number: u32,
        calldata: Binary
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
        calldata: Binary
    },

    PacketReceive {
        data: Binary,
        channel_id: Bytes32
    },

    PacketAck {
        data: Binary,
        result: Option<u8>,
        channel_id: Bytes32
    },

    PacketTimeout {
        data: Binary,
        channel_id: Bytes32
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


    // Ownership msgs

    TransferOwnership {
        new_owner: String
    }

}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
}