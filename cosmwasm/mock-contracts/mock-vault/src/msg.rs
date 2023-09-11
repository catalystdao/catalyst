use cosmwasm_schema::QueryResponses;
use cosmwasm_schema::cw_serde;
pub use catalyst_vault_common::msg::{InstantiateMsg, ExecuteMsg};
use catalyst_vault_common::msg::{
    AssetsResponse, CalcSendAssetResponse,
    ChainInterfaceResponse, FeeAdministratorResponse, GovernanceFeeShareResponse,
    OnlyLocalResponse, VaultFeeResponse, ReadyResponse,
    SetupMasterResponse, WeightResponse, FactoryResponse, FactoryOwnerResponse
};
use cw20::{AllowanceResponse, TokenInfoResponse};



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

    #[returns(VaultFeeResponse)]
    VaultFee {},
    #[returns(GovernanceFeeShareResponse)]
    GovernanceFeeShare {},
    #[returns(FeeAdministratorResponse)]
    FeeAdministrator {},
    #[returns(CalcSendAssetResponse)]


    // CW20 Implementation
    #[returns(BalanceResponse)]
    Balance { address: String },
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },

}
