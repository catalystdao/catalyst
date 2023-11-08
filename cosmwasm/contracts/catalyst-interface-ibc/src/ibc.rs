#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{DepsMut, Env, IbcChannelOpenMsg, IbcChannelConnectMsg, IbcBasicResponse, IbcChannelCloseMsg, IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg, IbcChannel, CosmosMsg, to_binary, SubMsg, WasmMsg, ReplyOn, StdError};

use catalyst_interface_common::{ContractError, error::Never, state::{handle_message_reception, handle_message_response, ack_fail, ack_success, SET_ACK_REPLY_ID}, msg::ExecuteMsg, bindings::{CustomMsg, InterfaceResponse}};

use crate::state::{IbcChannelInfo, OPEN_CHANNELS, WRAPPED_MESSAGES_REPLY_ID};


// NOTE: Large parts of this IBC section are based on the cw20-ics20 example repository.




// Constants
// ************************************************************************************************

pub const CATALYST_V1_CHANNEL_VERSION: &str = "catalyst-v1";




// Channel management
// ************************************************************************************************

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
            StdError::generic_err(&format!(
                "Only IBC channel version 'catalyst-v1' is supported, got {}.",
                channel.version.clone()
            )).into()
        );
    }

    // Check the channel version of the remote side. Note this value is only set in OpenTry and OpenAck,
    // and will occur in either 'ibc_channel_open' or 'ibc_channel_connect'. This check assumes that
    // at some point the 'counterparty_version' will be specified. (Code taken from cw20-ics20)
    if let Some(version) = counterparty_version {
        if version != CATALYST_V1_CHANNEL_VERSION {
            return Err(
                StdError::generic_err(&format!(
                    "Only IBC channel version 'catalyst-v1' is supported, got {}.",
                    version.to_string()
                )).into()
            );
        }
    }

    // NOTE: The channel order type is not checked, as the Catalyst protocol makes no requirement on
    // ordered/unordered channels.

    Ok(())
}




// Channel communication
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    mut deps: DepsMut,
    env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse<CustomMsg>, Never> {

    // Invoke the receive function (either 'ReceiveAsset' or 'ReceiveLiquidity') of the destination
    // vault.

    let common_response = handle_message_reception(
        &mut deps,
        &env,
        msg.packet.dest.channel_id,
        msg.packet.data
    );

    // Adapt the common interface handle response to the IBC interface requirements
    let ibc_response = common_response.and_then(
        |common_response| {
            encode_ibc_receive_response(&env, common_response)
        }
    );

    // This function should never error, rather it should send a failure message within the
    // returned ack.
    ibc_response.or_else(|_| {
        Ok(IbcReceiveResponse::new()
            .set_ack(ack_fail())
        )
    })

}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketAckMsg,
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {

    let handle_response = handle_message_response(
        msg.original_packet.dest.channel_id,
        msg.original_packet.data,
        Some(msg.acknowledgement.data)
    )?;

    encode_ibc_basic_response(handle_response)
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    msg: IbcPacketTimeoutMsg,
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {

    let handle_response = handle_message_response(
        msg.packet.dest.channel_id,
        msg.packet.data,
        None
    )?;

    encode_ibc_basic_response(handle_response)

}

pub fn encode_ibc_receive_response(
    env: &Env,
    native_response: InterfaceResponse
) -> Result<IbcReceiveResponse<CustomMsg>, ContractError> {

    // ! No IBC request handled by the interface should error, rather a fail-ack response should
    // ! always be returned. The 'native' interface response to cross-chain messages must thus be
    // ! adapted with extra logic to make sure this is the case. In practice this translates to
    // ! having at most only **ONE** submessage per incoming cross-chain message, to prevent having
    // ! some messages updating the chain state whilst other failing. If more than one message is
    // ! required, they need to be wrapped within another message. This 'wrapping' message will
    // ! only pass if all of its messages pass, and will fail if any of its messages fails.

    let sub_msgs = native_response.messages;

    match sub_msgs.len() {
        0 => {
            // If no messages have to be excuted, return a success-ack
            // NOTE: this case should never be reached, as a message is always generated.
            Ok(IbcReceiveResponse::new()
                .set_ack(ack_success())
                .add_attributes(native_response.attributes)
                .add_events(native_response.events)
            )
        },
        1 => {
            
            if sub_msgs[0].reply_on == ReplyOn::Always && sub_msgs[0].id == SET_ACK_REPLY_ID {
                Ok(
                    IbcReceiveResponse::new()
                        .add_submessages(sub_msgs)
                        .add_attributes(native_response.attributes)
                        .add_events(native_response.events)
                )
            }
            else {
                // ! This case is only reached if calldata is to be executed, so the message
                // ! **must** be wrapped in case it fails.
                let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: env.contract.address.to_string(),
                    msg: to_binary(&ExecuteMsg::WrapSubMsgs { sub_msgs })?,
                    funds: vec![]
                });
    
                let sub_msg = SubMsg::reply_always(
                    cosmos_msg,
                    WRAPPED_MESSAGES_REPLY_ID
                );
    
                Ok(IbcReceiveResponse::new()
                    .add_submessage(sub_msg)
                    .add_attributes(native_response.attributes)
                    .add_events(native_response.events)
                )
            }
        },
        _ => {

            // ! Wrap the submessages within a single message in case any of the messages fails.
            let cosmos_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: env.contract.address.to_string(),
                msg: to_binary(&ExecuteMsg::WrapSubMsgs { sub_msgs })?,
                funds: vec![]
            });

            let sub_msg = SubMsg::reply_always(
                cosmos_msg,
                WRAPPED_MESSAGES_REPLY_ID
            );

            Ok(
                IbcReceiveResponse::new()
                    .add_submessage(sub_msg)
                    .add_attributes(native_response.attributes)
                    .add_events(native_response.events)
            )
        }
    }
}



pub fn encode_ibc_basic_response(
    native_response: InterfaceResponse
) -> Result<IbcBasicResponse<CustomMsg>, ContractError> {
    
    Ok(
        IbcBasicResponse::new()
            .add_events(native_response.events)
            .add_attributes(native_response.attributes)
            .add_submessages(native_response.messages)
    )

}

