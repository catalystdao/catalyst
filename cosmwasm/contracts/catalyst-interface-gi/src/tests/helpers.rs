use std::fmt::Debug;

use catalyst_interface_common::catalyst_payload::{CatalystV1SendAssetPayload, SendAssetVariablePayload, CatalystEncodedAddress, CatalystV1SendLiquidityPayload, SendLiquidityVariablePayload};
use catalyst_vault_common::bindings::CustomMsg;
use cosmwasm_std::{Uint128, Addr, Binary, Empty, coin};
use cw_multi_test::{ContractWrapper, Module, Executor, AppResponse};
use catalyst_types::{U256, Bytes32};
use generalised_incentives_common::bytes32::Bytes32 as GIBytes32;
use generalised_incentives_mock_on_receive::{contract::{execute as gi_execute, instantiate as gi_instantiate, query as gi_query, reply as gi_reply}, msg::InstantiateMsg as GIInstantiateMsg};

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use test_helpers::{math::u256_to_f64, contract::ExpectedReceiveAssetResult, definitions::{SETUP_MASTER, LOST_GAS, MESSAGING_AUTH, CHANNEL_ID, REMOTE_CHAIN_INTERFACE}, env::CustomApp};
use crate::{tests::TestApp, msg::{InstantiateMsg, ExecuteMsg}};


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
    app: &mut CustomApp<HandlerC, CustomMsg>,    // Cannot be generic on `ExecC`, as the factory contract is hardcoded with `CustomMsg`
    generalised_incentives: String
) -> Addr
where
    HandlerC: Module<ExecT = CustomMsg, QueryT = Empty>
{

    let contract_code_storage = interface_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &InstantiateMsg {
            owner: SETUP_MASTER.to_string(),
            generalised_incentives
        },
        &[],
        "interface",
        None
    ).unwrap()
}

pub fn gi_contract_storage<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>
) -> u64
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty>,
    ExecC: Clone + Debug + DeserializeOwned + JsonSchema + PartialEq + 'static
{

    // Create contract wrapper
    let contract = ContractWrapper::new_with_empty(
        gi_execute,
        gi_instantiate,
        gi_query,
    ).with_reply_empty(gi_reply);

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

pub fn mock_instantiate_gi<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    self_identifier: Bytes32
) -> Addr
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty>,
    ExecC: Clone + Debug + DeserializeOwned + JsonSchema + PartialEq + 'static
{

    let contract_code_storage = gi_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &GIInstantiateMsg {
            messaging_authority: MESSAGING_AUTH.to_string(),
            self_identifier: GIBytes32(self_identifier.0),
            message_cost: coin(0, ""),
            lost_gas_address: LOST_GAS.to_string()
        },
        &[],
        "gi",
        None
    ).unwrap()
}


pub fn connect_mock_remote_chain<HandlerC>(
    app: &mut CustomApp<HandlerC, CustomMsg>,
    interface: Addr
) -> AppResponse
where
    HandlerC: Module<ExecT = CustomMsg, QueryT = Empty>
{
    app.execute_contract(
        Addr::unchecked(SETUP_MASTER),
        interface,
        &ExecuteMsg::ConnectNewChain {
            channel_id: CHANNEL_ID,
            remote_interface: CatalystEncodedAddress::try_encode(REMOTE_CHAIN_INTERFACE.as_bytes()).unwrap().to_json_binary(),
            remote_gi: Binary(vec![0])
        },
        &[]
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

pub fn encode_mock_send_asset_packet(
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

pub fn encode_mock_send_liquidity_packet(
    from_vault: impl ToString,
    to_vault: impl ToString,
    to_account: impl ToString,
    u: U256,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    from_amount: U256,
    block_number: u32,
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
    
    let packet = CatalystV1SendLiquidityPayload {
        from_vault,
        to_vault,
        to_account,
        u,
        variable_payload: SendLiquidityVariablePayload {
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            block_number,
            calldata,
        },
    };

    packet.try_encode().unwrap()
}
