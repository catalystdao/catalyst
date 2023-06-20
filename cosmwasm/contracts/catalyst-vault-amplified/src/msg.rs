use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint128, Binary, Uint64};
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
pub enum AmplifiedExecuteExtension {

    SetAmplification {
        target_timestamp: Uint64,
        target_amplification: Uint64
    },

}

pub type AmplifiedExecuteMsg = ExecuteMsg<AmplifiedExecuteExtension>;


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


    // Amplified vault specific queries
    #[returns(TargetAmplificationResponse)]
    TargetAmplification {},
    #[returns(AmplificationUpdateFinishTimestampResponse)]
    AmplificationUpdateFinishTimestamp {},
    #[returns(Balance0Response)]
    Balance0 {},


    // CW20 Implementation
    #[returns(BalanceResponse)]
    Balance { address: String },
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },

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
