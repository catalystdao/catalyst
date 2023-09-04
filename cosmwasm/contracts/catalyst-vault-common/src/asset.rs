//TODO rename asset.rs, as now vault tokens are also included in this file
#[cfg(all(not(feature="asset_native"), not(feature="asset_cw20")))]
compile_error!("An asset-type feature must be enabled (\"asset_native\" or \"asset_cw20\")");

#[cfg(all(feature="asset_native", feature="asset_cw20"))]
compile_error!("Multiple asset-type features cannot be enabled at the same time (\"asset_native\" and \"asset_cw20\")");


pub trait IntoCosmosVaultMsg {
    fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg>;
}

pub trait IntoVaultResponse {
    fn into_vault_response(self) -> VaultResponse;
}


use cosmwasm_std::{CosmosMsg, Response, SubMsg, Empty};

#[cfg(feature="asset_native")]
pub use native_asset_vault_modules::*;

#[cfg(feature="asset_native")]
pub mod native_asset_vault_modules {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::CosmosMsg;
    use cosmwasm_std::Empty;

    pub use vault_assets::asset::asset_native::{
        NativeAsset as Asset,
        NativeAssetMsg as AssetMsg,
        NativeVaultAssets as VaultAssets
    };

    pub use vault_token::vault_token::vault_token_native::{
        NativeVaultToken as VaultToken,
        NativeVaultTokenMsg as VaultTokenMsg
    };

    use super::IntoCosmosVaultMsg;

    #[cw_serde]
    pub enum VaultMsg {     // NOTE: This must match the allowed msgs of CosmosMsg::Custom
        Token(token_bindings::TokenMsg),
    }

    impl IntoCosmosVaultMsg for AssetMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg> {
            match self {
                AssetMsg::Bank(bank_msg) => CosmosMsg::Bank(bank_msg)
            }
        }
    }

    impl IntoCosmosVaultMsg for VaultTokenMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg> {
            match self {
                VaultTokenMsg::Token(token_msg) => CosmosMsg::Custom(VaultMsg::Token(token_msg)),
            }
        }
    }

    //TODO this shouldn't be needed
    impl IntoCosmosVaultMsg for CosmosMsg<Empty> {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg> {
            match self {
                CosmosMsg::Bank(bank_msg) => CosmosMsg::Bank(bank_msg),
                CosmosMsg::Wasm(wasm_msg) => CosmosMsg::Wasm(wasm_msg),
                CosmosMsg::Custom(_) => panic!("Unable to cast from CosmosMsg::Custom(Empty) to CosmosMsg::Custom(VaultMsg)"),
                CosmosMsg::Staking(staking_msg) => CosmosMsg::Staking(staking_msg),
                CosmosMsg::Distribution(distribution_msg) => CosmosMsg::Distribution(distribution_msg),
                CosmosMsg::Stargate { type_url, value } => CosmosMsg::Stargate { type_url, value },
                CosmosMsg::Ibc(ibc_msg) => CosmosMsg::Ibc(ibc_msg),
                CosmosMsg::Gov(gov_msg) => CosmosMsg::Gov(gov_msg),
                _ => unimplemented!(),
            }
        }
    }

}


#[cfg(feature="asset_cw20")]
pub use cw20_asset_vault_modules::*;

#[cfg(feature="asset_cw20")]
pub mod cw20_asset_vault_modules {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::CosmosMsg;
    use cosmwasm_std::Empty;
    

    pub use vault_assets::asset::asset_cw20::{
        Cw20Asset as Asset,
        Cw20AssetMsg as AssetMsg,
        Cw20VaultAssets as VaultAssets
    };

    pub use vault_token::vault_token::vault_token_cw20::{
        Cw20VaultToken as VaultToken,
        Cw20VaultTokenMsg as VaultTokenMsg
    };
    
    use super::IntoCosmosVaultMsg;

    #[cw_serde]
    pub enum VaultMsg {
    }

    impl IntoCosmosVaultMsg for AssetMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg> {
            match self {
                AssetMsg::Wasm(wasm_msg) => CosmosMsg::Wasm(wasm_msg),
            }
        }
    }

    impl IntoCosmosVaultMsg for VaultTokenMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg> {
            panic!("Unsupported empty message casting.")    // Code should never reach this point
        }
    }

    impl IntoCosmosVaultMsg for CosmosMsg<Empty> {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<VaultMsg> {
            match self {
                CosmosMsg::Bank(bank_msg) => CosmosMsg::Bank(bank_msg),
                CosmosMsg::Wasm(wasm_msg) => CosmosMsg::Wasm(wasm_msg),
                CosmosMsg::Custom(_) => panic!("Unable to cast from CosmosMsg::Custom(Empty) to CosmosMsg::Custom(VaultMsg)"),
                CosmosMsg::Staking(staking_msg) => CosmosMsg::Staking(staking_msg),
                CosmosMsg::Distribution(distribution_msg) => CosmosMsg::Distribution(distribution_msg),
                CosmosMsg::Stargate { type_url, value } => CosmosMsg::Stargate { type_url, value },
                CosmosMsg::Ibc(ibc_msg) => CosmosMsg::Ibc(ibc_msg),
                CosmosMsg::Gov(gov_msg) => CosmosMsg::Gov(gov_msg),
                _ => unimplemented!(),
            }
        }
    }

    
}


pub use vault_assets::asset::{VaultAssetsTrait, AssetTrait};
pub use vault_token::vault_token::VaultTokenTrait;

pub type VaultResponse = cosmwasm_std::Response<VaultMsg>;

impl IntoVaultResponse for Response<Empty> {
    fn into_vault_response(self) -> VaultResponse {

        let mut response = VaultResponse::new();

        response = response.add_submessages(
            self.messages.iter().map(|sub_msg| {
                SubMsg {
                    id: sub_msg.id,
                    msg: sub_msg.msg.clone().into_cosmos_vault_msg(),
                    gas_limit: sub_msg.gas_limit,
                    reply_on: sub_msg.reply_on.clone(),
                }
            }).collect::<Vec<SubMsg<VaultMsg>>>()
        );

        response = response.add_attributes(
            self.attributes
        );

        response = response.add_events(
            self.events
        );

        if let Some(data) = self.data {
            response = response.set_data(
                data
            );
        }

        response
    }
}