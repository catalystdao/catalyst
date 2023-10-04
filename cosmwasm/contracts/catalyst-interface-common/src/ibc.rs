#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, IbcChannelOpenMsg, IbcChannelConnectMsg, IbcBasicResponse, IbcChannelCloseMsg, IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg, IbcChannel, IbcPacket, Binary, CosmosMsg, to_binary, SubMsg, Reply, Response, SubMsgResult, Uint128, WasmMsg, ReplyOn};

use catalyst_vault_common::{msg::ExecuteMsg as VaultExecuteMsg, bindings::{VaultResponse, CustomMsg}};

use crate::{ContractError, state::{IbcChannelInfo, OPEN_CHANNELS, UNDERWRITE_REPLY_ID, handle_underwrite_reply, handle_receive_asset, handle_receive_liquidity, RECEIVE_ASSET_REPLY_ID, RECEIVE_LIQUIDITY_REPLY_ID, handle_calldata_on_reply}, catalyst_ibc_payload::CatalystV1Packet, error::Never, msg::ExecuteMsg};


// NOTE: Large parts of this IBC section are based on the cw20-ics20 example repository.


// IBC Interface constants 
pub const CATALYST_V1_CHANNEL_VERSION: &str = "catalyst-v1";

// Ack Codes
pub const ACK_SUCCESS: u8 = 0x00;
pub const ACK_FAIL: u8 = 0x01;

// Reply IDs    //TODO reorganize 'Reply IDs' location
pub const RECEIVE_REPLY_ID: u64 = 0x100;



// Channel management ***********************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    msg: IbcChannelOpenMsg
) -> Result<(), ContractError> {

    // Enforce the desired IBC protocol configuration
    validate_ibc_channel_config(msg.channel(), msg.counterparty_version())
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelConnectMsg,
) -> Result<IbcBasicResponse, ContractError> {

    // Enforce the desired IBC protocol configuration
    validate_ibc_channel_config(msg.channel(), msg.counterparty_version())?;

    // Save the channel info
    let ibc_channel: IbcChannel = msg.into();
    OPEN_CHANNELS.save(
        deps.storage,
        &ibc_channel.endpoint.channel_id.clone(),
        &IbcChannelInfo {
            endpoint: ibc_channel.endpoint,
            counterparty_endpoint: ibc_channel.counterparty_endpoint,
            connection_id: ibc_channel.connection_id,
        }
    )?;

    Ok(IbcBasicResponse::default())
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    deps: DepsMut,
    _env: Env,
    msg: IbcChannelCloseMsg,
) -> Result<IbcBasicResponse, ContractError> {

    // To recover from a lost channel, a new channel has to be established (permissionless) and the Catalyst vaults
    // that relied on the closed channel have to be set up with new 'vault connection' employing the new channel.
    // !NOTE: This may not be possible depending on the vault configuration.
    
    // Remove the channel info from the list of open channels
    let ibc_channel: IbcChannel = msg.into();
    OPEN_CHANNELS.remove(
        deps.storage,
        &ibc_channel.endpoint.channel_id.clone()
    );

    Ok(IbcBasicResponse::default())
}


/// Validate an IBC channel configuration.
/// 
/// # Arguments:
/// * `channel` - The local channel configuration.
/// * `counterparty_version` - The counterparty's channel version.
/// 
fn validate_ibc_channel_config(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {

    // Check the channel version on the local side
    if channel.version != CATALYST_V1_CHANNEL_VERSION {
        return Err(
            ContractError::InvalidIbcChannelVersion {
                version: channel.version.clone()
            }
        );
    }

    // Check the channel version of the remote side. Note this value is only set in OpenTry and OpenAck,
    // and will occur in either 'ibc_channel_open' or 'ibc_channel_connect'. This check assumes that
    // at some point the 'counterparty_version' will be specified. (Code taken from cw20-ics20)
    if let Some(version) = counterparty_version {
        if version != CATALYST_V1_CHANNEL_VERSION {
            return Err(
                ContractError::InvalidIbcChannelVersion {
                    version: version.to_string()
                }
            );
        }
    }

    // NOTE: The channel order type is not checked, as the Catalyst protocol makes no requirement on
    // ordered/unordered channels.

    Ok(())
}




// Channel communication ********************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse<CustomMsg>, Never> {

    // Invoke the receive function (either 'ReceiveAsset' or 'ReceiveLiquidity') of the destination vault.
    // This function should never error, rather it should send a failure message within the returned ack.
    on_packet_receive(deps, env, msg.packet)
        .or_else(|_| {
            Ok(IbcReceiveResponse::new()
                .set_ack(ack_fail())
            )
        })

}


//TODO move `reply` to 'state.rs'
// If the vault invocation errors (i.e. the submessage created within 'on_packet_receive'), return a custom 'fail' ack.
// NOTE: this 'reply' code is needed, as the Catalyst protocol is not compatible with the default 'failed-ack' that is 
// generated by CosmWasm. 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<VaultResponse, ContractError> {
    match reply.id {
        RECEIVE_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => {
                // Set the custom 'success-ack' for successful executions.
                Ok(Response::new().set_data(ack_success()))
            },
            SubMsgResult::Err(_) => {
                // Set the custom 'failed-ack' for unsuccessful executions.
                Ok(Response::new().set_data(ack_fail()))
            }
        },
        RECEIVE_ASSET_REPLY_ID | RECEIVE_LIQUIDITY_REPLY_ID => match reply.result {
            SubMsgResult::Ok(response) => {
                handle_calldata_on_reply(deps, response)
            },
            SubMsgResult::Err(_) => {
                unreachable!(
                    "Receive asset/liquidity reply should never be an error (ReplyOn::Success set)."
                )
            }
        },
        UNDERWRITE_REPLY_ID => match reply.result {
            SubMsgResult::Ok(response) => {
                handle_underwrite_reply(deps, env, response)
                
            },
            SubMsgResult::Err(_) => {
                unreachable!(
                    "Underwrite reply should never be an error (ReplyOn::Success set)."
                )
            }
        }
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {

    // NOTE: Only the first byte of the 'ack' response is checked. This allows future 'ack' implementations to
    // extend the 'ack' format.
    let ack = msg.acknowledgement.data.0.get(0);

    if ack == Some(&ACK_SUCCESS) {
        // Handle the 'success' case.
        on_packet_success(msg.original_packet)
    }
    else {
        // Handle every other case as a 'failure'.
        on_packet_failure(msg.original_packet)
    }
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {

    on_packet_failure(msg.packet)

}


/// Generate a 'success' ack data response.
pub fn ack_success() -> Binary {
    Into::<Binary>::into(vec![ACK_SUCCESS])
}

/// Generate a 'fail' ack data response.
pub fn ack_fail() -> Binary {
    Into::<Binary>::into(vec![ACK_FAIL])
}


/// Handle the reception of a packet.
/// 
/// # Arguments:
/// * `packet` - The IBC packet.
/// 
pub fn on_packet_receive(
    mut deps: DepsMut,
    env: Env,
    packet: IbcPacket
) -> Result<IbcReceiveResponse<CustomMsg>, ContractError> {

    let catalyst_packet = CatalystV1Packet::try_decode(packet.data)?;

    // Match the payload type and build up the execute message
    let handle_response = match catalyst_packet {

        CatalystV1Packet::SendAsset(payload) => {

            // Convert min_out into Uint128
            let min_out: Uint128 = payload.variable_payload.min_out.try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?;

            handle_receive_asset(
                &mut deps,
                &env,
                packet.dest.channel_id,
                payload.to_vault.try_decode_as_string()?,
                payload.variable_payload.to_asset_index,
                payload.to_account.try_decode_as_string()?,
                payload.u,
                min_out,
                payload.variable_payload.underwrite_incentive_x16,
                payload.from_vault.to_binary(),
                payload.variable_payload.from_amount,
                payload.variable_payload.from_asset.to_binary(),
                payload.variable_payload.block_number,
                payload.variable_payload.calldata
            )
        },

        CatalystV1Packet::SendLiquidity(payload) => {

            // Convert the minimum outputs into Uint128
            let min_vault_tokens: Uint128 = payload.variable_payload.min_vault_tokens.try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?;
            let min_reference_asset: Uint128 = payload.variable_payload.min_reference_asset.try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?;

            handle_receive_liquidity(
                &mut deps,
                packet.dest.channel_id,
                payload.to_vault.try_decode_as_string()?,
                payload.to_account.try_decode_as_string()?,
                payload.u,
                min_vault_tokens,
                min_reference_asset,
                payload.from_vault.to_binary(),
                payload.variable_payload.from_amount,
                payload.variable_payload.block_number,
                payload.variable_payload.calldata
            )
        }
    }?;

    // Convert the 'handle' response into an IbcReceiveResponse
    let sub_msgs = handle_response.messages;

    match sub_msgs.len() {
        0 => {
            Ok(IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attributes(handle_response.attributes)
                .add_events(handle_response.events)
            )
        },
        1 => {
            //TODO this can be optimized further: wrapping of the requested message should not be required
            // If the requested submessage does not require extra handling on 'reply', set it to
            // trigger the `RECEIVE_REPLY_ID` logic.
            if sub_msgs[0].reply_on == ReplyOn::Never {

                let mut sub_msg = sub_msgs[0].clone();
                sub_msg.id = RECEIVE_REPLY_ID;      // Can override the `id` safely, as `reply_on` is set to `Never`
                sub_msg.reply_on = ReplyOn::Always;
    
                Ok(IbcReceiveResponse::new()    // 'ack' set on 'reply'
                    .add_submessage(sub_msg)
                    .add_attributes(handle_response.attributes)
                    .add_events(handle_response.events)
                )
            }
            // Otherwise, wrap it in another submessage to make sure its 'reply' logic does not
            // interfere with the `RECEIVE_REPLY_ID` logic.
            else {
                let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: to_binary(&ExecuteMsg::WrapSubMsgs { sub_msgs })?,
                    funds: vec![]
                });
    
                let sub_msg = SubMsg::reply_always(
                    cosmos_msg,
                    RECEIVE_REPLY_ID
                );
    
                Ok(IbcReceiveResponse::new()    // 'ack' set on 'reply'
                    .add_submessage(sub_msg)
                    .add_attributes(handle_response.attributes)
                    .add_events(handle_response.events)
                )
            }
        },
        _ => {

            // Wrap the submessages within a single message. This is required to be able to revert
            // the entire state of all the sub messages should one of the messages revert without
            // having to fail the entire transaction.
            let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::WrapSubMsgs { sub_msgs })?,
                funds: vec![]
            });

            let sub_msg = SubMsg::reply_always(
                cosmos_msg,
                RECEIVE_REPLY_ID
            );

            Ok(IbcReceiveResponse::new()    // 'ack' set on 'reply'
                .add_submessage(sub_msg)
                .add_attributes(handle_response.attributes)
                .add_events(handle_response.events)
            )
        }
    }
}



/// Handle the ack/fail of a previously sent packet.
/// 
/// ! **NOTE-DEV**: This function should never error for a **valid** Catalyst packet.
/// 
/// # Arguments:
/// * `packet` - The IBC packet.
/// * `success` - Whether the cross chain transaction has been successful or not.
/// 
pub fn on_packet_response(
    packet: IbcPacket,
    success: bool
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {

    let catalyst_packet = CatalystV1Packet::try_decode(packet.data)?;
    
    // Build the SendAsset/SendLiquidity ack response message
    let receive_asset_execute_msg: cosmwasm_std::WasmMsg = match catalyst_packet {

        CatalystV1Packet::SendAsset(payload) => {

            // Convert 'from_amount' into Uint128
            let from_amount: Uint128 = payload.variable_payload.from_amount.try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?;

            // Build the message to execute the success/fail call.
            // NOTE: none of the fields are validated, these must be correctly handled by the vault.
            let msg = match success {
                true => VaultExecuteMsg::<()>::OnSendAssetSuccess {
                    channel_id: packet.dest.channel_id,
                    to_account: payload.to_account.to_binary(),
                    u: payload.u,
                    escrow_amount: from_amount,
                    asset_ref: payload.variable_payload.from_asset.try_decode_as_string()?,
                    block_number_mod: payload.variable_payload.block_number
                },
                false => VaultExecuteMsg::<()>::OnSendAssetFailure {
                    channel_id: packet.dest.channel_id,
                    to_account: payload.to_account.to_binary(),
                    u: payload.u,
                    escrow_amount: from_amount,
                    asset_ref: payload.variable_payload.from_asset.try_decode_as_string()?,
                    block_number_mod: payload.variable_payload.block_number
                },
            };

            Ok::<cosmwasm_std::WasmMsg, ContractError>(cosmwasm_std::WasmMsg::Execute {
                contract_addr: payload.from_vault.try_decode_as_string()?,    // No need to validate, 'Execute' will fail for an invalid address.
                msg: to_binary(&msg)?,
                funds: vec![]
            })

        },

        CatalystV1Packet::SendLiquidity(payload) => {

            // Convert 'from_amount' into Uint128
            let from_amount: Uint128 = payload.variable_payload.from_amount.try_into()
                .map_err(|_| ContractError::PayloadDecodingError {})?;

            // Build the message to execute the success/fail call.
            // NOTE: none of the fields are validated, these must be correctly handled by the vault.
            let msg = match success {
                true => VaultExecuteMsg::<()>::OnSendLiquiditySuccess {
                    channel_id: packet.dest.channel_id,
                    to_account: payload.to_account.to_binary(),
                    u: payload.u,
                    escrow_amount: from_amount,
                    block_number_mod: payload.variable_payload.block_number
                },
                false => VaultExecuteMsg::<()>::OnSendLiquidityFailure {
                    channel_id: packet.dest.channel_id,
                    to_account: payload.to_account.to_binary(),
                    u: payload.u,
                    escrow_amount: from_amount,
                    block_number_mod: payload.variable_payload.block_number
                },
            };

            Ok::<cosmwasm_std::WasmMsg, ContractError>(cosmwasm_std::WasmMsg::Execute {
                contract_addr: payload.from_vault.try_decode_as_string()?,    // No need to validate, 'Execute' will fail for an invalid address.
                msg: to_binary(&msg)?,
                funds: vec![]
            })

        }
    }?;

    // Build the response messsage.
    let response_msg = CosmosMsg::Wasm(receive_asset_execute_msg);

    Ok(IbcBasicResponse::new()
        .add_message(response_msg)
    )
}


/// Wrapper around `on_packet_response` to specifically handle the 'success' case.
/// 
/// # Arguments:
/// * `packet` - The IBC packet.
/// 
pub fn on_packet_success(
    packet: IbcPacket
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {
    on_packet_response(packet, true)
}


/// Wrapper around `on_packet_response` to specifically handle the 'failure' case.
/// 
/// # Arguments:
/// * `packet` - The IBC packet.
/// 
pub fn on_packet_failure(
    packet: IbcPacket
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {
    on_packet_response(packet, false)
}
