use cosmwasm_std::{Uint128, Addr, Uint64, Binary};
use cw_multi_test::{ContractWrapper, App, Executor, AppResponse};
use catalyst_vault_common::msg::{InstantiateMsg, ExecuteMsg};
use std::marker::PhantomData;
use vault_assets::asset::AssetTrait;
use crate::{misc::get_response_attribute, definitions::{SETUP_MASTER, FACTORY_OWNER}, env::CustomTestEnv, asset::CustomTestAsset};


pub const DEFAULT_TEST_VAULT_FEE : Uint64 = Uint64::new(70000000000000000u64);   // 7%
pub const DEFAULT_TEST_GOV_FEE  : Uint64 = Uint64::new(50000000000000000u64);   // 5%



// Contracts storage

pub fn vault_factory_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        catalyst_factory::contract::execute,
        catalyst_factory::contract::instantiate,
        catalyst_factory::contract::query,
    ).with_reply(catalyst_factory::contract::reply);
    
    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

pub fn interface_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        mock_catalyst_ibc_interface::contract::execute,
        mock_catalyst_ibc_interface::contract::instantiate,
        mock_catalyst_ibc_interface::contract::query,
    );

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

pub fn calldata_target_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        mock_calldata_target::contract::execute,
        mock_calldata_target::contract::instantiate,
        mock_calldata_target::contract::query,
    );

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}



// Contracts instantiation

pub fn mock_instantiate_factory(
    app: &mut App,
    default_governance_fee_share: Option<Uint64>
) -> Addr {

    let factory_contract_code = vault_factory_contract_storage(app);

    app.instantiate_contract(
        factory_contract_code,
        Addr::unchecked(FACTORY_OWNER),
        &catalyst_factory::msg::InstantiateMsg {
            default_governance_fee_share: default_governance_fee_share.unwrap_or(DEFAULT_TEST_GOV_FEE)
        },
        &[],
        "factory",
        None
    ).unwrap()
}

pub fn mock_instantiate_interface(
    app: &mut App
) -> Addr {

    let contract_code_storage = interface_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &catalyst_ibc_interface::msg::InstantiateMsg {},
        &[],
        "interface",
        None
    ).unwrap()
}

pub fn mock_instantiate_calldata_target(
    app: &mut App
) -> Addr {

    let contract_code_storage = calldata_target_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &mock_calldata_target::msg::InstantiateMsg {},
        &[],
        "calldata-target",
        None
    ).unwrap()
}



// Factory helpers

pub fn mock_factory_deploy_vault<A: AssetTrait, T: CustomTestAsset<A>>(
    env: &mut impl CustomTestEnv<A, T>,
    assets: Vec<T>,
    assets_balances: Vec<Uint128>,
    weights: Vec<Uint128>,
    amplification: Uint64,
    vault_code_id: u64,
    chain_interface: Option<Addr>,
    factory: Option<Addr>
) -> Addr {

    // Deploy factory if not provided
    let factory = factory.unwrap_or(
        mock_instantiate_factory(env.get_app(), None)
    );

    let vault_assets: Vec<A> = assets.iter()
        .map(|asset| asset.into_vault_asset())
        .collect();

    // Deploy the new vault
    let response = env.execute_contract(
        Addr::unchecked(SETUP_MASTER),
        factory,
        &catalyst_factory::msg::ExecuteMsg::DeployVault {
            vault_code_id,
            assets: vault_assets,
            assets_balances: assets_balances.clone(),
            weights,
            amplification,
            vault_fee: DEFAULT_TEST_VAULT_FEE,
            name: "TestVault".to_string(),
            symbol: "TP".to_string(),
            chain_interface: chain_interface.map(|value| value.to_string())
        },
        assets,
        assets_balances
    ).unwrap();

    let deploy_vault_event = response.events.iter()
        .find(|event| {
            event.ty == "wasm-deploy-vault".to_string()
        })
        .unwrap().clone();

    let vault = get_response_attribute::<String>(deploy_vault_event, "vault_address").unwrap();

    Addr::unchecked(vault)
}



#[derive(Clone)]
pub struct InitializeSwapCurvesMockConfig<A: AssetTrait, T: CustomTestAsset<A>> {
    pub assets: Vec<T>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<Uint128>,
    pub amp: Uint64,
    pub depositor: String,
    pub asset_trait: PhantomData<A>
}

impl<A: AssetTrait, T: CustomTestAsset<A>> InitializeSwapCurvesMockConfig<A, T> {

    pub fn transfer_vault_assets(
        &self,
        app: &mut App,
        vault: String,
        depositor: Addr
    ) {
        self.assets
            .iter()
            .zip(&self.assets_balances)
            .filter(|(_, amount)| *amount != Uint128::zero())
            .for_each(|(asset, amount)| {

                asset.transfer(
                    app,
                    *amount,
                    depositor.clone(),
                    vault.to_string()
                );

            });
    }

    pub fn into_execute_msg(self) -> ExecuteMsg<(), A> {
        Into::into(self)
    }

}

impl<A: AssetTrait, T: CustomTestAsset<A>> Into<ExecuteMsg<(), A>> for InitializeSwapCurvesMockConfig<A, T> {
    fn into(self) -> ExecuteMsg<(), A> {

        let vault_assets = self.assets.iter()
            .map(|test_asset| test_asset.into_vault_asset())
            .collect::<Vec<A>>();

        ExecuteMsg::<(), A>::InitializeSwapCurves {
            assets: vault_assets,
            weights: self.weights,
            amp: self.amp,
            depositor: self.depositor
        }

    }
}



// Vault management helpers

pub fn mock_instantiate_vault_msg(
    chain_interface: Option<String>
) -> InstantiateMsg {
    InstantiateMsg {
        name: "TestVault".to_string(),
        symbol: "TP".to_string(),
        chain_interface,
        vault_fee: DEFAULT_TEST_VAULT_FEE,
        governance_fee_share: DEFAULT_TEST_GOV_FEE,
        fee_administrator: FACTORY_OWNER.to_string(),   // The 'fee_administrator' is set to the 'factory_owner' as this is the default when vaults are deployed via the factory
        setup_master: SETUP_MASTER.to_string()
    }
}

pub fn mock_instantiate_vault(
    app: &mut App,
    vault_code_id: u64,
    chain_interface: Option<Addr>
) -> Addr {

    let instantiate_msg = mock_instantiate_vault_msg(
        chain_interface.map(|addr| addr.to_string())
    );

    app.instantiate_contract(
        vault_code_id,
        Addr::unchecked(SETUP_MASTER),
        &instantiate_msg,
        &[],
        "vault",
        None
    ).unwrap()
}


pub fn mock_finish_vault_setup(
    app: &mut App,
    vault_contract: Addr
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(SETUP_MASTER),
        vault_contract,
        &ExecuteMsg::<()>::FinishSetup {},
        &[]
    ).unwrap()
}


pub fn mock_set_vault_connection(
    app: &mut App,
    vault_contract: Addr,
    channel_id: String,
    to_vault: Binary,
    state: bool
) -> AppResponse {
    app.execute_contract::<ExecuteMsg::<()>>(
        Addr::unchecked(FACTORY_OWNER),
        vault_contract,
        &ExecuteMsg::<()>::SetConnection {
            channel_id,
            to_vault,
            state
        },
        &[]
    ).unwrap()
}


pub fn mock_set_vault_fee(
    app: &mut App,
    vault_contract: Addr,
    fee: Uint64
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(FACTORY_OWNER),
        vault_contract,
        &ExecuteMsg::<()>::SetVaultFee { fee },
        &[]
    ).unwrap()
}


pub fn mock_set_governance_fee_share(
    app: &mut App,
    vault_contract: Addr,
    fee: Uint64
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(FACTORY_OWNER),
        vault_contract,
        &ExecuteMsg::<()>::SetGovernanceFeeShare { fee },
        &[]
    ).unwrap()
}


// Swap types

pub struct ExpectedLocalSwapResult {
    pub u: f64,
    pub to_amount: f64,
    pub vault_fee: f64,
    pub governance_fee: f64
}

pub struct ExpectedSendAssetResult {
    pub u: f64,
    pub vault_fee: f64,
    pub governance_fee: f64
}

pub struct ExpectedReceiveAssetResult {
    pub to_amount: f64,
}

pub struct ExpectedSendLiquidityResult {
    pub u: f64
}

pub struct ExpectedReceiveLiquidityResult {
    pub to_amount: f64
}

pub struct ExpectedReferenceAsset {
    pub amount: f64
}
