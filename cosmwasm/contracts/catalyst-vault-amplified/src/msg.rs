use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Uint128, Binary};
use catalyst_types::{U256, I256, Bytes32};
pub use catalyst_vault_common::msg::InstantiateMsg;
use catalyst_vault_common::{msg::{
    ExecuteMsg, AssetEscrowResponse, AssetsResponse, CalcLocalSwapResponse, CalcReceiveAssetResponse, CalcSendAssetResponse, ChainInterfaceResponse, FeeAdministratorResponse, GetLimitCapacityResponse, GovernanceFeeShareResponse, LiquidityEscrowResponse, OnlyLocalResponse, VaultConnectionStateResponse, VaultFeeResponse, ReadyResponse, SetupMasterResponse, TotalEscrowedAssetResponse, TotalEscrowedLiquidityResponse, WeightResponse, FactoryResponse, FactoryOwnerResponse, TotalSupplyResponse, BalanceResponse, AssetResponse
}, bindings::Asset};

#[cfg(feature="asset_native")]
use catalyst_vault_common::msg::VaultTokenDenomResponse;

#[cfg(feature="asset_cw20")]
use cw20::{AllowanceResponse, TokenInfoResponse};


// Extend Catalyst's base ExecuteMsg enum with custom messages
#[cw_serde]
pub enum AmplifiedExecuteExtension {

    #[cfg(feature="amplification_update")]
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
        channel_id: Bytes32,
        vault: Binary
    },

    #[returns(ReadyResponse)]
    Ready {},
    #[returns(OnlyLocalResponse)]
    OnlyLocal {},
    #[returns(AssetsResponse)]
    Assets {},
    #[returns(AssetResponse)]
    Asset{
        asset_ref: String
    },
    #[returns(AssetResponse)]
    AssetByIndex{
        asset_index: u8
    },
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
    #[returns(Balance0Response)]
    Balance0 {},
    #[returns(UnitTrackerResponse)]
    UnitTracker {},

    #[cfg(feature="amplification_update")]
    #[returns(TargetAmplificationResponse)]
    TargetAmplification {},
    #[cfg(feature="amplification_update")]
    #[returns(AmplificationUpdateFinishTimestampResponse)]
    AmplificationUpdateFinishTimestamp {},


    // Native Asset Implementation
    #[cfg(feature="asset_native")]
    #[returns(VaultTokenDenomResponse)]
    VaultTokenDenom {},


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
pub struct Balance0Response {
    pub balance_0: U256
}

#[cw_serde]
pub struct UnitTrackerResponse {
    pub amount: I256
}

#[cfg(feature="amplification_update")]
#[cw_serde]
pub struct TargetAmplificationResponse {
    pub target_amplification: Uint64
}

#[cfg(feature="amplification_update")]
#[cw_serde]
pub struct AmplificationUpdateFinishTimestampResponse {
    pub timestamp: Uint64
}
