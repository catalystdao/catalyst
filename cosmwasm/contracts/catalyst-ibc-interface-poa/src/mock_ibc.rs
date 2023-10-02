#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    DepsMut, Env, IbcReceiveResponse, Reply, Response, SubMsgResult, IbcPacket, IbcEndpoint, Binary, Timestamp, MessageInfo
};
use catalyst_ibc_interface::{ContractError, ibc::{on_packet_receive, ack_fail, on_packet_success, on_packet_failure, RECEIVE_REPLY_ID, ACK_SUCCESS, ack_success}};
use catalyst_vault_common::bindings::VaultResponse;

use crate::state::is_owner;


fn build_mock_ibc_packet(data: Binary, channel_id: String) -> IbcPacket {
    IbcPacket::new(
        data,
        IbcEndpoint { port_id: "".to_string(), channel_id: "".to_string() },
        IbcEndpoint { port_id: "".to_string(), channel_id },
        0u64,
        Timestamp::from_seconds(1800000000).into()
    )
}

// NOTE: this function is based on 'ibc_packet_receive' of the catalyst-ibc-interface
pub fn execute_ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    data: Binary,
    channel_id: String
) -> Result<VaultResponse, ContractError> {

    if !is_owner(deps.as_ref(), info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let mock_ibc_packet = build_mock_ibc_packet(data, channel_id);

    // Invoke the receive function (either 'ReceiveAsset' or 'ReceiveLiquidity') of the destination vault.
    // This function should never error, rather it should send a failure message within the returned ack.
    let ibc_response: Result<IbcReceiveResponse<_>, ContractError> = on_packet_receive(deps, env, mock_ibc_packet)
        .or_else(|_| {
            Ok(IbcReceiveResponse::new()
                .set_ack(ack_fail())
            )
        });

    Ok(
        Response::new()
        .add_submessages(ibc_response?.messages)
    )
}


// NOTE: this function is based on 'reply' of the catalyst-ibc-interface
// If the vault invocation errors (i.e. the submessage created within 'on_packet_receive'), return a custom fail ack.
// NOTE: this 'reply' code is needed, as the Catalyst protocol is not compatible with the default 'failed-ack' that is 
// generated by CosmWasm. 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    _deps: DepsMut,
    _env: Env,
    reply: Reply
) -> Result<VaultResponse, ContractError> {
    match reply.id {
        RECEIVE_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(
                Response::new()
                    .add_attribute("action", "ibc_receive")
                    .add_attribute("ack", ack_success().to_string())
                ),
            SubMsgResult::Err(_) => Ok(
                Response::new()
                    .add_attribute("action", "ibc_receive")
                    .add_attribute("ack", ack_fail().to_string())
                )
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}


// NOTE: this function is based on 'ibc_packet_ack' of the catalyst-ibc-interface
pub fn execute_ibc_packet_ack(
    deps: DepsMut,
    info: MessageInfo,
    data: Binary,
    response: Binary,
    channel_id: String
) -> Result<VaultResponse, ContractError> {

    if !is_owner(deps.as_ref(), info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let mock_ibc_packet = build_mock_ibc_packet(data, channel_id);

    let ack = response.0.get(0);

    let response;
    if ack == Some(&ACK_SUCCESS) {
        // Handle the 'success' case.
        response = on_packet_success(mock_ibc_packet)?;
    }
    else {
        // Handle every other case as a 'failure'.
        response = on_packet_failure(mock_ibc_packet)?;
    }

    Ok(
        Response::new()
            .add_submessages(response.messages)
            .add_attribute("action", "ibc_ack")
    )
}


pub fn execute_ibc_packet_timeout(
    deps: DepsMut,
    info: MessageInfo,
    data: Binary,
    channel_id: String
) -> Result<VaultResponse, ContractError> {

    if !is_owner(deps.as_ref(), info.sender)? {
        return Err(ContractError::Unauthorized {});
    }

    let response = on_packet_failure(
        build_mock_ibc_packet(data, channel_id)
    )?;

    Ok(
        Response::new()
            .add_submessages(response.messages)
            .add_attribute("action", "ibc_timeout")
    )
}



