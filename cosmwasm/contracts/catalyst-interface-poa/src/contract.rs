use catalyst_interface_common::{state::{encode_send_cross_chain_asset, encode_send_cross_chain_liquidity, handle_message_reception, handle_message_response, handle_reply, set_max_underwriting_duration, underwrite, underwrite_and_check_connection, expire_underwrite, setup, update_owner, is_owner, query_underwrite_identifier}, msg::InterfaceCommonQueryMsg};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, Reply, Uint64, to_binary};
use cw2::set_contract_version;
use catalyst_types::{U256, Bytes32};
use catalyst_interface_common::{bindings::InterfaceResponse, ContractError};

use crate::msg::{ExecuteMsg, InstantiateMsg};

// Version information
const CONTRACT_NAME: &str = "catalyst-interface-poa";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const TRANSACTION_TIMEOUT: u64 = 2 * 60 * 60;   // 2 hours

pub const MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS: Uint64 = Uint64::new(24 * 60 * 60);       // 1 day at 1 block/s
pub const MIN_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(12 * 60 * 60);       // 12 hours at 1 block/s
pub const MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS: Uint64 = Uint64::new(15 * 24 * 60 * 60);  // 15 days at 1 block/s


// Instantiation
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<InterfaceResponse, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(
        deps,
        info,
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
            calldata
        } => execute_send_cross_chain_asset(
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
            calldata
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
            calldata
        } => execute_send_cross_chain_liquidity(
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
            calldata
        ),

        ExecuteMsg::PacketReceive {
            data,
            channel_id
        } => execute_packet_receive(
            deps,
            env,
            info,
            data,
            channel_id
        ),
    
        ExecuteMsg::PacketAck {
            data,
            response,
            channel_id
        } => execute_packet_ack(
            deps,
            info,
            data,
            response,
            channel_id
        ),
    
        ExecuteMsg::PacketTimeout {
            data,
            channel_id
        } => execute_packet_timeout(
            deps,
            info,
            data,
            channel_id
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

        // Ownership msgs
        ExecuteMsg::TransferOwnership {
            new_owner
        } => update_owner(deps, info, new_owner)

    }
}

fn execute_send_cross_chain_asset(
    env: Env,
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
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    let payload = encode_send_cross_chain_asset(
        info,
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

    Ok(Response::new()
        .add_attribute("action", "interface-send-cross-chain-asset")
        .add_attribute("channel_id", channel_id.to_base64())
        .add_attribute("data", payload.to_base64())
        .add_attribute("timeout", env.block.time.plus_seconds(TRANSACTION_TIMEOUT).seconds().to_string())
    )
}

fn execute_send_cross_chain_liquidity(
    env: Env,
    info: MessageInfo,
    channel_id: Bytes32,
    to_vault: Binary,
    to_account: Binary,
    u: U256,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    from_amount: Uint128,
    block_number: u32,
    calldata: Binary
) -> Result<InterfaceResponse, ContractError> {

    let payload = encode_send_cross_chain_liquidity(
        info,
        to_vault,
        to_account,
        u,
        min_vault_tokens,
        min_reference_asset,
        from_amount,
        block_number,
        calldata
    )?;

    Ok(Response::new()
        .add_attribute("action", "interface-send-cross-chain-liquidity")
        .add_attribute("channel_id", channel_id.to_base64())
        .add_attribute("data", payload.to_base64())
        .add_attribute("timeout", env.block.time.plus_seconds(TRANSACTION_TIMEOUT).seconds().to_string())
    )
}


pub fn execute_packet_receive(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    data: Binary,
    channel_id: Bytes32
) -> Result<InterfaceResponse, ContractError> {

    if !is_owner(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let response = handle_message_reception(
        &mut deps,
        &env,
        channel_id,
        data
    )?;

    Ok(
        response.add_attribute("action", "interface-receive")
    )
}

pub fn execute_packet_ack(
    deps: DepsMut,
    info: MessageInfo,
    data: Binary,
    response: Binary,
    channel_id: Bytes32
) -> Result<InterfaceResponse, ContractError> {

    if !is_owner(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let response = handle_message_response(
        channel_id,
        data,
        Some(response)
    )?;

    Ok(
        response.add_attribute("action", "interface-ack")
    )
}


pub fn execute_packet_timeout(
    deps: DepsMut,
    info: MessageInfo,
    data: Binary,
    channel_id: Bytes32
) -> Result<InterfaceResponse, ContractError> {

    if !is_owner(deps.as_ref(), &info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let response = handle_message_response(
        channel_id,
        data,
        None
    )?;

    Ok(
        response.add_attribute("action", "interface-timeout")
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
pub fn query(_deps: Deps, _env: Env, msg: InterfaceCommonQueryMsg) -> StdResult<Binary> {
    match msg {
        InterfaceCommonQueryMsg::UnderwriteIdentifier {
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        } => to_binary(&query_underwrite_identifier(
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        )?)
    }
}
