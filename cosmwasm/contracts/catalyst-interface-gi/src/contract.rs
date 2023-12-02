use catalyst_interface_common::state::{encode_send_cross_chain_asset, encode_send_cross_chain_liquidity, handle_message_reception, handle_message_response, handle_reply, set_max_underwriting_duration, underwrite, underwrite_and_check_connection, expire_underwrite, setup, update_owner, query_underwrite_identifier};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, StdResult, Uint128, Reply, Uint64, to_json_binary, WasmMsg};
use cw2::set_contract_version;
use catalyst_types::{U256, Bytes32};
use catalyst_interface_common::{bindings::InterfaceResponse, ContractError};
use generalised_incentives_common::{msg::ExecuteMsg as GIExecuteMsg, state::IncentiveDescription, bytes32::Bytes32 as GIBytes32};

use crate::{msg::{ExecuteMsg, InstantiateMsg, QueryMsg}, state::{set_generalised_incentives, set_min_gas_for, query_estimate_additional_cost, get_generalised_incentives, connect_new_chain, get_remote_interface, check_route_description, only_generalised_incentives, set_min_ack_gas_price}};

// Version information
const CONTRACT_NAME: &str = "catalyst-interface-gi";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS: Uint64 = Uint64::new(24 * 60 * 60);       // 1 day at 1 block/s
pub const MIN_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(12 * 60 * 60);       // 12 hours at 1 block/s
pub const MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(15 * 24 * 60 * 60);  // 15 days at 1 block/s


// Instantiation
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<InterfaceResponse, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_generalised_incentives(&mut deps, msg.generalised_incentives)?;

    setup(
        deps,
        msg.owner,
        MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS,
        Some(MIN_UNDERWRITE_DURATION_ALLOWED_BLOCKS),
        Some(MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS)
    )
}




// Execution
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<InterfaceResponse, ContractError> {
    match msg {

        ExecuteMsg::SendCrossChainAsset {
            channel_id,
            to_vault,
            to_account,
            to_asset_index,
            u,
            min_out,
            from_amount,
            from_asset,
            underwrite_incentive_x16,
            block_number,
            calldata,
            incentive
        } => execute_send_cross_chain_asset(
            deps.as_ref(),
            env,
            info,
            channel_id,
            to_vault,
            to_account,
            to_asset_index,
            u,
            min_out,
            from_amount,
            from_asset,
            underwrite_incentive_x16,
            block_number,
            calldata,
            incentive
        ),

        ExecuteMsg::SendCrossChainLiquidity {
            channel_id,
            to_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            block_number,
            calldata,
            incentive
        } => execute_send_cross_chain_liquidity(
            deps.as_ref(),
            env,
            info,
            channel_id,
            to_vault,
            to_account,
            u,
            min_vault_tokens,
            min_reference_asset,
            from_amount,
            block_number,
            calldata,
            incentive
        ),

        ExecuteMsg::SetMaxUnderwriteDuration {
            new_max_underwrite_duration
        } => set_max_underwriting_duration(
            &mut deps,
            &info,
            new_max_underwrite_duration,
            Some(MIN_UNDERWRITE_DURATION_ALLOWED_BLOCKS),
            Some(MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS)
        ),

        ExecuteMsg::Underwrite {
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        } => underwrite(
            &mut deps,
            &env,
            &info,
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        ),

        ExecuteMsg::UnderwriteAndCheckConnection {
            channel_id,
            from_vault,
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        } => underwrite_and_check_connection(
            &mut deps,
            &env,
            &info,
            channel_id,
            from_vault,
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        ),

        ExecuteMsg::ExpireUnderwrite {
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        } => expire_underwrite(
            &mut deps,
            &env,
            &info,
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        ),

        ExecuteMsg::ConnectNewChain {
            channel_id,
            remote_interface,
            remote_gi
        } => connect_new_chain(
            deps,
            info,
            channel_id,
            remote_interface,
            remote_gi
        ),

        ExecuteMsg::SetMinGasFor {
            chain_identifier,
            min_gas
        } => set_min_gas_for(
            deps,
            info,
            chain_identifier,
            min_gas
        ),

        ExecuteMsg::SetMinAckGasPrice {
            min_gas_price
        } => set_min_ack_gas_price(
            deps,
            info,
            min_gas_price
        ),
        
        ExecuteMsg::ReceiveMessage {
            source_identifier,
            message_identifier: _,
            from_application,
            message
        } => execute_receive_message(
            deps,
            env,
            info,
            source_identifier,
            from_application,
            message
        ),

        ExecuteMsg::ReceiveAck {
            destination_identifier,
            message_identifier: _,
            acknowledgement
        } => execute_receive_ack(
            deps,
            info,
            destination_identifier,
            acknowledgement
        ),

        // Ownership msgs
        ExecuteMsg::TransferOwnership {
            new_owner
        } => update_owner(deps, info, new_owner)

    }
}

fn execute_send_cross_chain_asset(
    deps: Deps,
    _env: Env,
    info: MessageInfo,
    channel_id: Bytes32,
    to_vault: Binary,
    to_account: Binary,
    to_asset_index: u8,
    u: U256,
    min_out: U256,
    from_amount: Uint128,
    from_asset: String,
    underwrite_incentive_x16: u16,
    block_number: u32,
    calldata: Binary,
    incentive: IncentiveDescription
) -> Result<InterfaceResponse, ContractError> {

    check_route_description(
        &deps,
        channel_id.clone(),
        to_account.clone(),
        to_vault.clone(),
        &incentive
    )?;

    let payload = encode_send_cross_chain_asset(
        info.clone(),
        to_vault,
        to_account,
        to_asset_index,
        u,
        min_out,
        from_amount,
        from_asset,
        underwrite_incentive_x16,
        block_number,
        calldata
    )?;

    let gi_message = WasmMsg::Execute {
        contract_addr: get_generalised_incentives(&deps)?.to_string(),
        msg: to_json_binary(&GIExecuteMsg::SubmitMessage {
            destination_identifier: GIBytes32(channel_id.0.clone()),
            destination_address: get_remote_interface(&deps, channel_id)?.to_json_binary(),
            message: payload,
            incentive
        })?,
        funds: info.funds
    };

    Ok(
        InterfaceResponse::new()
            .add_message(gi_message)
    )
}

fn execute_send_cross_chain_liquidity(
    deps: Deps,
    _env: Env,
    info: MessageInfo,
    channel_id: Bytes32,
    to_vault: Binary,
    to_account: Binary,
    u: U256,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    from_amount: Uint128,
    block_number: u32,
    calldata: Binary,
    incentive: IncentiveDescription
) -> Result<InterfaceResponse, ContractError> {

    check_route_description(
        &deps,
        channel_id.clone(),
        to_account.clone(),
        to_vault.clone(),
        &incentive
    )?;

    let payload = encode_send_cross_chain_liquidity(
        info.clone(),
        to_vault,
        to_account,
        u,
        min_vault_tokens,
        min_reference_asset,
        from_amount,
        block_number,
        calldata
    )?;

    let gi_message = WasmMsg::Execute {
        contract_addr: get_generalised_incentives(&deps)?.to_string(),
        msg: to_json_binary(&GIExecuteMsg::SubmitMessage {
            destination_identifier: GIBytes32(channel_id.0.clone()),
            destination_address: get_remote_interface(&deps, channel_id)?.to_json_binary(),
            message: payload,
            incentive
        })?,
        funds: info.funds
    };

    Ok(
        InterfaceResponse::new()
            .add_message(gi_message)
    )
}


pub fn execute_receive_message(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    channel_id: Bytes32,
    from_application: Binary,
    message: Binary
) -> Result<InterfaceResponse, ContractError> {

    only_generalised_incentives(&deps.as_ref(), &info)?;

    let expected_from_application = get_remote_interface(
        &deps.as_ref(),
        channel_id.clone()
    )?.to_json_binary();

    if from_application != expected_from_application {
        return Err(ContractError::InvalidSourceInterface {})
    }


    handle_message_reception(
        &mut deps,
        &env,
        channel_id,
        message
    )
}

pub fn execute_receive_ack(
    deps: DepsMut,
    info: MessageInfo,
    channel_id: Bytes32,
    acknowledgement: Binary
) -> Result<InterfaceResponse, ContractError> {

    only_generalised_incentives(&deps.as_ref(), &info)?;

    let result = acknowledgement.get(0).cloned();
    let original_payload = acknowledgement.get(1..)
        .ok_or(ContractError::PayloadDecodingError {})?
        .into();

    handle_message_response(
        channel_id,
        original_payload,
        result
    )
}




// Reply
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<InterfaceResponse, ContractError> {

    // Run the common reply handler before handling any custom replies
    let response = handle_reply(
        deps,
        env,
        reply.clone()
    )?;

    match response {
        Some(response) => Ok(response),
        None => Err(ContractError::UnknownReplyId { id: reply.id })
    }
}




// Query
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::UnderwriteIdentifier {
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        } => to_json_binary(&query_underwrite_identifier(
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        )?),

        QueryMsg::EstimateAdditionalCost {            
        } => to_json_binary(&query_estimate_additional_cost(&deps)?)
    }
}
