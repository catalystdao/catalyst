use catalyst_vault_common::bindings::Asset;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Uint64, Uint128, Addr};


#[cw_serde]
pub struct InstantiateMsg {
    // The default value for the governance fee share (18 decimals).
    pub default_governance_fee_share: Uint64
}


#[cw_serde]
pub enum ExecuteMsg<A = Asset> {

    /// Deploy a new vault (permissionless).
    /// * `vault_code_id` - The code id of the *stored* contract with which to deploy the new vault.
    /// * `assets` - A list of the assets that are to be supported by the vault.
    /// * `assets_balances` - The asset balances that are going to be deposited on the vault.
    /// * `weights` - The weights applied to the assets.
    /// * `amplification` - The amplification value applied to the vault.
    /// * `vault_fee` - The vault fee (18 decimals).
    /// * `name` - The name of the vault token.
    /// * `symbol` - The symbol of the vault token.
    /// * `chain_interface` - The interface used for cross-chain swaps. It can be set to None to disable cross-chain swaps.
    DeployVault {
        vault_code_id: u64,
        assets: Vec<A>,
        assets_balances: Vec<Uint128>,
        weights: Vec<Uint128>,
        amplification: Uint64,
        vault_fee: Uint64,
        name: String,
        symbol: String,
        chain_interface: Option<String>
    },


    /// Modify the default governance fee share
    /// * `fee` - The new governance fee share (18 decimals).
    SetDefaultGovernanceFeeShare {
        fee: Uint64
    },


    // Ownership msgs

    /// Transfer the ownership of the factory.
    /// * `new_owner` - The new owner of the contract. Must be a valid address.
    TransferOwnership {
        new_owner: String
    }

}


#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {

    /// Query the factory owner.
    #[returns(OwnerResponse)]
    Owner {},

    /// Query the default governance fee share.
    #[returns(DefaultGovernanceFeeShareResponse)]
    DefaultGovernanceFeeShare {}

}

#[cw_serde]
pub struct OwnerResponse {
    // The contract owner.
    pub owner: Option<Addr>
}

#[cw_serde]
pub struct DefaultGovernanceFeeShareResponse {
    // The governance fee share (18 decimals).
    pub fee: Uint64
}
