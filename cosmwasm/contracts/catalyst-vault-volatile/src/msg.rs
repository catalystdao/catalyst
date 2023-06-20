use cosmwasm_schema::QueryResponses;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Uint64, Uint128, Binary};
use catalyst_types::U256;
pub use catalyst_vault_common::msg::{InstantiateMsg, ExecuteMsg};
use catalyst_vault_common::msg::{
    AssetEscrowResponse, AssetsResponse, CalcLocalSwapResponse, CalcReceiveAssetResponse, CalcSendAssetResponse,
    ChainInterfaceResponse, FeeAdministratorResponse, GetLimitCapacityResponse, GovernanceFeeShareResponse,
    LiquidityEscrowResponse, OnlyLocalResponse, VaultConnectionStateResponse, VaultFeeResponse, ReadyResponse,
    SetupMasterResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, WeightResponse, FactoryResponse, FactoryOwnerResponse
};
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};


#[cw_serde]
pub enum VolatileExecuteExtension {

    SetWeights {
        new_weights: Vec<Uint64>,
        target_timestamp: Uint64   //TODO EVM mismatch (targetTime)
    },

}

pub type VolatileExecuteMsg = ExecuteMsg<VolatileExecuteExtension>;



#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {


    // Common Queries
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
        asset: String
    },

    #[returns(VaultFeeResponse)]
    VaultFee {},
    #[returns(GovernanceFeeShareResponse)]
    GovernanceFeeShare {},
    #[returns(FeeAdministratorResponse)]
    FeeAdministrator {},
    #[returns(CalcSendAssetResponse)]

    CalcSendAsset {
        from_asset: String,
        amount: Uint128
    },
    #[returns(CalcReceiveAssetResponse)]
    CalcReceiveAsset {
        to_asset: String,
        u: U256
    },
    #[returns(CalcLocalSwapResponse)]
    CalcLocalSwap {
        from_asset: String,
        to_asset: String,
        amount: Uint128
    },

    #[returns(GetLimitCapacityResponse)]
    GetLimitCapacity {},

    #[returns(TotalEscrowedAssetResponse)]
    TotalEscrowedAsset {
        asset: String
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


    // Volatile vault specific queries
    #[returns(TargetWeightResponse)]
    TargetWeight {
        asset: String
    },
    #[returns(WeightsUpdateFinishTimestampResponse)]
    WeightsUpdateFinishTimestamp {},


    // CW20 Implementation
    #[returns(BalanceResponse)]
    Balance { address: String },
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },

}


#[cw_serde]
pub struct TargetWeightResponse {
    pub target_weight: Uint64
}

#[cw_serde]
pub struct WeightsUpdateFinishTimestampResponse {
    pub timestamp: Uint64
}