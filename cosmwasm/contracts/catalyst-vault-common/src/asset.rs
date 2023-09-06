use cosmwasm_std::{CosmosMsg, Response, SubMsg, Empty};

// Re-export traits
pub use vault_assets::asset::{VaultAssetsTrait, AssetTrait};
pub use vault_token::vault_token::VaultTokenTrait;


//TODO rename asset.rs, as now vault tokens are also included in this file
#[cfg(all(not(feature="asset_native"), not(feature="asset_cw20")))]
compile_error!("An asset-type feature must be enabled (\"asset_native\" or \"asset_cw20\")");

#[cfg(all(feature="asset_native", feature="asset_cw20"))]
compile_error!("Multiple asset-type features cannot be enabled at the same time (\"asset_native\" and \"asset_cw20\")");


pub trait IntoCosmosCustomMsg<T> {
    fn into_cosmos_vault_msg(self) -> CosmosMsg<T>;
}

pub trait IntoVaultResponse {
    fn into_vault_response(self) -> VaultResponse;
}



#[cfg(feature="asset_native")]
pub use native_asset_vault_modules::{
    NativeAsset as Asset,
    NativeAssetMsg as AssetMsg,
    NativeVaultAssets as VaultAssets,
    NativeVaultToken as VaultToken,
    NativeVaultTokenMsg as VaultTokenMsg,
    NativeAssetCustomMsg as CustomMsg
};

#[cfg(feature="asset_cw20")]
pub use cw20_asset_vault_modules::{
    Cw20Asset as Asset,
    Cw20AssetMsg as AssetMsg,
    Cw20VaultAssets as VaultAssets,
    Cw20VaultToken as VaultToken,
    Cw20VaultTokenMsg as VaultTokenMsg,
    Cw20AssetCustomMsg as CustomMsg
};



pub mod native_asset_vault_modules {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::CosmosMsg;
    use cosmwasm_std::Empty;

    pub use vault_assets::asset::asset_native::{
        NativeAsset, NativeAssetMsg, NativeVaultAssets
    };

    pub use vault_token::vault_token::vault_token_native::{
        NativeVaultToken, NativeVaultTokenMsg
    };

    use super::IntoCosmosCustomMsg;


    #[cw_serde]
    pub enum NativeAssetCustomMsg {     // NOTE: This must match the allowed msgs of CosmosMsg::Custom
        Token(token_bindings::TokenMsg),
    }

    impl IntoCosmosCustomMsg<NativeAssetCustomMsg> for NativeAssetMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<NativeAssetCustomMsg> {
            match self {
                NativeAssetMsg::Bank(bank_msg) => CosmosMsg::Bank(bank_msg)
            }
        }
    }

    impl IntoCosmosCustomMsg<NativeAssetCustomMsg> for NativeVaultTokenMsg {

        fn into_cosmos_vault_msg(self) -> CosmosMsg<NativeAssetCustomMsg> {
            match self {
                NativeVaultTokenMsg::Token(token_msg) => {
                    CosmosMsg::Custom(NativeAssetCustomMsg::Token(token_msg))
                },
            }
        }
    }

    //TODO this shouldn't be needed
    impl IntoCosmosCustomMsg<NativeAssetCustomMsg> for CosmosMsg<Empty> {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<NativeAssetCustomMsg> {
            match self {
                CosmosMsg::Bank(bank_msg) => CosmosMsg::Bank(bank_msg),
                CosmosMsg::Wasm(wasm_msg) => CosmosMsg::Wasm(wasm_msg),
                CosmosMsg::Custom(_) => panic!("Unable to cast from CosmosMsg::Custom(Empty) to CosmosMsg::Custom(NativeAssetCustomMsg)"),
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



pub mod cw20_asset_vault_modules {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{CosmosMsg, Empty};
    

    pub use vault_assets::asset::asset_cw20::{
        Cw20Asset, Cw20AssetMsg, Cw20VaultAssets
    };

    pub use vault_token::vault_token::vault_token_cw20::{
        Cw20VaultToken, Cw20VaultTokenMsg
    };
    
    use super::IntoCosmosCustomMsg;


    #[cw_serde]
    pub enum Cw20AssetCustomMsg {
    }

    impl IntoCosmosCustomMsg<Cw20AssetCustomMsg> for Cw20AssetMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<Cw20AssetCustomMsg> {
            match self {
                Cw20AssetMsg::Wasm(wasm_msg) => CosmosMsg::<Cw20AssetCustomMsg>::Wasm(wasm_msg),
            }
        }
    }

    impl IntoCosmosCustomMsg<Cw20AssetCustomMsg> for Cw20VaultTokenMsg {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<Cw20AssetCustomMsg> {
            unreachable!("Unsupported empty message casting.")    // Code should never reach this point
        }
    }

    impl IntoCosmosCustomMsg<Cw20AssetCustomMsg> for CosmosMsg<Empty> {
        fn into_cosmos_vault_msg(self) -> CosmosMsg<Cw20AssetCustomMsg> {
            match self {
                CosmosMsg::Bank(bank_msg) => CosmosMsg::Bank(bank_msg),
                CosmosMsg::Wasm(wasm_msg) => CosmosMsg::Wasm(wasm_msg),
                CosmosMsg::Custom(_) => panic!("Unable to cast from CosmosMsg::Custom(Empty) to CosmosMsg::Custom(Cw20AssetCustomMsg)"),
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


pub type VaultResponse = cosmwasm_std::Response<CustomMsg>;

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
            }).collect::<Vec<SubMsg<CustomMsg>>>()
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