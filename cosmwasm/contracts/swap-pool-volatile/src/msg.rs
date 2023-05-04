
use cosmwasm_schema::QueryResponses;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use ethnum::U256;
pub use swap_pool_common::msg::{InstantiateMsg, ExecuteMsg};
use swap_pool_common::msg::{
    AssetEscrowResponse, AssetsResponse, CalcLocalSwapResponse, CalcReceiveAssetResponse, CalcSendAssetResponse,
    ChainInterfaceResponse, FeeAdministratorResponse, GetLimitCapacityResponse, GovernanceFeeShareResponse,
    LiquidityEscrowResponse, OnlyLocalResponse, PoolConnectionStateResponse, PoolFeeResponse, ReadyResponse,
    SetupMasterResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, WeightsResponse, FactoryResponse, FactoryOwnerResponse
};
use cw20::{AllowanceResponse, BalanceResponse, TokenInfoResponse};
use schemars::JsonSchema;
use serde::{Serialize, Deserialize};


// Implement JsonSchema for U256, see https://graham.cool/schemars/examples/5-remote_derive/
//TODO VERIFY THIS IS CORRECT AND SAFE!
//TODO move to common place
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(remote = "U256")]
pub struct U256Def([u128; 2]);


#[cw_serde]
pub enum VolatileExecuteExtension {

    SetWeights {
        weights: Vec<u64>,      //TODO EVM mismatch (name newWeights)
        target_timestamp: u64   //TODO EVM mismatch (targetTime)
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

    #[returns(PoolConnectionStateResponse)]
    PoolConnectionState {
        channel_id: String,
        pool: Vec<u8>
    },

    #[returns(ReadyResponse)]
    Ready {},
    #[returns(OnlyLocalResponse)]
    OnlyLocal {},
    #[returns(AssetsResponse)]
    Assets {},
    #[returns(WeightsResponse)]
    Weights {},

    #[returns(PoolFeeResponse)]
    PoolFee {},
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
        #[serde(with = "U256Def")]
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
        hash: Vec<u8>
    },
    #[returns(LiquidityEscrowResponse)]
    LiquidityEscrow {
        hash: Vec<u8>
    },


    // Volatile pool specific queries
    #[returns(TargetWeightsResponse)]
    TargetWeights {},
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
pub struct TargetWeightsResponse {
    pub target_weights: Vec<u64>
}

#[cw_serde]
pub struct WeightsUpdateFinishTimestampResponse {
    pub timestamp: u64
}