use catalyst_interface_common::catalyst_payload::{CatalystV1SendAssetPayload, SendAssetVariablePayload, CatalystEncodedAddress};
use catalyst_vault_common::bindings::{CustomMsg, Asset};
use cosmwasm_std::{Uint128, Addr, Binary, Empty};
use cw_multi_test::{ContractWrapper, Module, Executor};
use catalyst_types::U256;

use test_helpers::{math::u256_to_f64, contract::{ExpectedReceiveAssetResult, ExpectedReceiveLiquidityResult, ExpectedReferenceAsset, mock_factory_deploy_vault, mock_set_vault_connection}, definitions::{SETUP_MASTER, CHANNEL_ID, UNDERWRITER}, env::{CustomApp, CustomTestEnv}, asset::{TestNativeAsset, CustomTestAsset}, misc::encode_payload_address};
use crate::tests::{TestApp, TestEnv, TestAsset};

use super::parameters::{TEST_VAULT_ASSET_COUNT, TEST_VAULT_BALANCES, TEST_VAULT_WEIGHTS, AMPLIFICATION};


// Contracts
pub fn vault_contract_storage(
    app: &mut TestApp
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        catalyst_vault_volatile::contract::execute,
        catalyst_vault_volatile::contract::instantiate,
        catalyst_vault_volatile::contract::query,
    );
    
    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

pub fn interface_contract_storage<HandlerC>(
    app: &mut CustomApp<HandlerC, CustomMsg>    // Cannot be generic on `ExecC`, as the factory contract is hardcoded with `CustomMsg`
) -> u64
where
    HandlerC: Module<ExecT = CustomMsg, QueryT = Empty>
{

    // Create contract wrapper
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    ).with_reply(crate::contract::reply);

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

pub fn mock_instantiate_interface<HandlerC>(
    app: &mut CustomApp<HandlerC, CustomMsg>    // Cannot be generic on `ExecC`, as the factory contract is hardcoded with `CustomMsg`
) -> Addr
where
    HandlerC: Module<ExecT = CustomMsg, QueryT = Empty>
{

    let contract_code_storage = interface_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &catalyst_interface_common::msg::InstantiateMsg {},
        &[],
        "interface",
        None
    ).unwrap()
}


pub fn compute_expected_receive_asset(
    u: U256,
    to_weight: Uint128,
    to_balance: Uint128
) -> ExpectedReceiveAssetResult {

    // Convert arguments into float
    let u = u256_to_f64(u) / 1e18;
    let to_weight = to_weight.u128() as f64;
    let to_balance = to_balance.u128() as f64;

    // Compute swap
    ExpectedReceiveAssetResult {
        to_amount: to_balance * (1. - (-u/to_weight).exp())
    }
    
}

pub fn encode_mock_packet(
    from_vault: impl ToString,
    to_vault: impl ToString,
    to_account: impl ToString,
    u: U256,
    to_asset_index: u8,
    min_out: U256,
    from_amount: U256,
    from_asset: impl ToString,
    block_number: u32,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Binary {

    let from_vault = CatalystEncodedAddress::try_encode(
        from_vault.to_string().as_bytes()
    ).unwrap();
    let to_vault = CatalystEncodedAddress::try_encode(
        to_vault.to_string().as_bytes()
    ).unwrap();
    let to_account = CatalystEncodedAddress::try_encode(
        to_account.to_string().as_bytes()
    ).unwrap();
    let from_asset = CatalystEncodedAddress::try_encode(
        from_asset.to_string().as_bytes()
    ).unwrap();
    
    let packet = CatalystV1SendAssetPayload {
        from_vault,
        to_vault,
        to_account,
        u,
        variable_payload: SendAssetVariablePayload {
            to_asset_index,
            min_out,
            from_amount,
            from_asset,
            block_number,
            underwrite_incentive_x16,
            calldata,
        },
    };

    packet.try_encode().unwrap()
}




pub struct MockTestState {
    pub interface: Addr,
    pub vault: Addr,
    pub from_vault: String,
    pub vault_assets: Vec<TestAsset>,
    pub vault_initial_balances: Vec<Uint128>,
    pub vault_weights: Vec<Uint128>
}

impl MockTestState {

    pub fn initialize(
        env: &mut TestEnv
    ) -> Self {

        // Instantiate and initialize vault
        let interface = mock_instantiate_interface(env.get_app());
        let vault_assets = env.get_assets()[..TEST_VAULT_ASSET_COUNT].to_vec();
        let vault_initial_balances = TEST_VAULT_BALANCES.to_vec();
        let vault_weights = TEST_VAULT_WEIGHTS.to_vec();
        let vault_code_id = vault_contract_storage(env.get_app());
        let vault = mock_factory_deploy_vault::<Asset, _, _>(
            env,
            vault_assets.clone(),
            vault_initial_balances.clone(),
            vault_weights.clone(),
            AMPLIFICATION,
            vault_code_id,
            Some(interface.clone()),
            None,
            None
        );

        // Connect vault with a mock vault
        let from_vault = "from_vault".to_string();
        mock_set_vault_connection(
            env.get_app(),
            vault.clone(),
            CHANNEL_ID.to_string(),
            encode_payload_address(from_vault.as_bytes()),
            true
        );

        Self {
            interface,
            vault,
            from_vault,
            vault_assets,
            vault_initial_balances,
            vault_weights,
        }
    }
}
