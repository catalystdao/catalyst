use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Uint128, Addr};


#[cw_serde]
pub struct InstantiateMsg {
    pub default_governance_fee_share: Uint64
}


#[cw_serde]
pub enum ExecuteMsg {
    DeployVault {
        vault_code_id: u64,
        assets: Vec<String>,
        assets_balances: Vec<Uint128>,
        weights: Vec<Uint64>,
        amplification: Uint64,
        pool_fee: Uint64,
        name: String,
        symbol: String,
        chain_interface: Option<String>
    },

    SetDefaultGovernanceFeeShare {
        fee: Uint64
    },


    // Ownership msgs

    TransferOwnership {
        new_owner: String
    }


}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(OwnerResponse)]
    Owner {},
    #[returns(DefaultGovernanceFeeShareResponse)]
    DefaultGovernanceFeeShare {}
}

#[cw_serde]
pub struct OwnerResponse {
    pub owner: Option<Addr>
}

#[cw_serde]
pub struct DefaultGovernanceFeeShareResponse {
    pub fee: Uint64
}
