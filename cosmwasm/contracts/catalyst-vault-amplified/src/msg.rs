use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Uint128, Binary};
use catalyst_types::{U256, I256};
pub use catalyst_vault_common::msg::InstantiateMsg;
use catalyst_vault_common::{msg::{
    ExecuteMsg, AssetEscrowResponse, AssetsResponse, CalcLocalSwapResponse, CalcReceiveAssetResponse, CalcSendAssetResponse, ChainInterfaceResponse, FeeAdministratorResponse, GetLimitCapacityResponse, GovernanceFeeShareResponse, LiquidityEscrowResponse, OnlyLocalResponse, VaultConnectionStateResponse, VaultFeeResponse, ReadyResponse, SetupMasterResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, WeightResponse, FactoryResponse, FactoryOwnerResponse, TotalSupplyResponse, BalanceResponse
}, bindings::Asset};

#[cfg(feature="asset_cw20")]
use cw20::{AllowanceResponse, TokenInfoResponse};


// Extend Catalyst's base ExecuteMsg enum with custom messages
#[cw_serde]
pub enum AmplifiedExecuteExtension {

    SetAmplification {
        target_timestamp: Uint64,
        target_amplification: Uint64
    },

    UpdateMaxLimitCapacity {
    }

}

pub type AmplifiedExecuteMsg = ExecuteMsg<AmplifiedExecuteExtension, Asset>;


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {


    // Catalyst Base Queries
    #[returns(ChainInterfaceResponse)]
    ChainInterface {},
    #[returns(SetupMasterResponse)]
    SetupMaster {},
    #[returns(FactoryResponse)]
    Factory {},
    #[returns(FactoryOwnerResponse)]
    FactoryOwner {},

    #[returns(VaultConnectionStateResponse)]
    VaultConnectionState {
        channel_id: String,
        vault: Binary
    },

    #[returns(ReadyResponse)]
    Ready {},
    #[returns(OnlyLocalResponse)]
    OnlyLocal {},
    #[returns(AssetsResponse)]
    Assets {},
    #[returns(WeightResponse)]
    Weight {
        asset_ref: String
    },

    #[returns(TotalSupplyResponse)]
    TotalSupply {},
    #[returns(BalanceResponse)]
    Balance {
        address: String
    },

    #[returns(VaultFeeResponse)]
    VaultFee {},
    #[returns(GovernanceFeeShareResponse)]
    GovernanceFeeShare {},
    #[returns(FeeAdministratorResponse)]
    FeeAdministrator {},

    #[returns(CalcSendAssetResponse)]
    CalcSendAsset {
        from_asset_ref: String,
        amount: Uint128
    },
    #[returns(CalcReceiveAssetResponse)]
    CalcReceiveAsset {
        to_asset_ref: String,
        u: U256
    },
    #[returns(CalcLocalSwapResponse)]
    CalcLocalSwap {
        from_asset_ref: String,
        to_asset_ref: String,
        amount: Uint128
    },

    #[returns(GetLimitCapacityResponse)]
    GetLimitCapacity {},

    #[returns(TotalEscrowedAssetResponse)]
    TotalEscrowedAsset {
        asset_ref: String
    },
    #[returns(TotalEscrowedLiquidityResponse)]
    TotalEscrowedLiquidity {},
    #[returns(AssetEscrowResponse)]
    AssetEscrow {
        hash: Binary
    },
    #[returns(LiquidityEscrowResponse)]
    LiquidityEscrow {
        hash: Binary
    },


    // Amplified vault specific queries
    #[returns(AmplificationResponse)]
    Amplification {},
    #[returns(TargetAmplificationResponse)]
    TargetAmplification {},
    #[returns(AmplificationUpdateFinishTimestampResponse)]
    AmplificationUpdateFinishTimestamp {},
    #[returns(Balance0Response)]
    Balance0 {},
    #[returns(UnitTrackerResponse)]
    UnitTracker {},


    // CW20 Implementation
    #[cfg(feature="asset_cw20")]
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[cfg(feature="asset_cw20")]
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },

}


#[cw_serde]
pub struct AmplificationResponse {
    pub amplification: Uint64
}

#[cw_serde]
pub struct TargetAmplificationResponse {
    pub target_amplification: Uint64
}

#[cw_serde]
pub struct AmplificationUpdateFinishTimestampResponse {
    pub timestamp: Uint64
}

#[cw_serde]
pub struct Balance0Response {
    pub balance_0: U256
}

#[cw_serde]
pub struct UnitTrackerResponse {
    pub amount: I256
}
