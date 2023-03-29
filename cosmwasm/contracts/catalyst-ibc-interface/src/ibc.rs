use cosmwasm_std::{
    entry_point, DepsMut, Env, StdResult, IbcChannelOpenMsg, IbcChannelConnectMsg, IbcBasicResponse, IbcChannelCloseMsg, 
    IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg, IbcChannel
};

use crate::{ContractError, state::{IbcChannelInfo, OPEN_CHANNELS}};


// NOTE: Large parts of this IBC section are based on the cw20-ics20 example repository.


// IBC Interface constants 
pub const CATALYST_V1_CHANNEL_VERSION: &str = "catalyst-v1";




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

    // TODO overhaul the following
    // To recover from a lost channel, a new channel has to be established (permissionless) and the Catalyst pools
    // that relied on the closed channel have to be set up with new 'pool connections' employing the new channel.
    
    // Remove the channel info from the list of open channels
    let ibc_channel: IbcChannel = msg.into();
    OPEN_CHANNELS.remove(
        deps.storage,
        &ibc_channel.endpoint.channel_id.clone()
    );

    Ok(IbcBasicResponse::default())
}



fn validate_ibc_channel_config(
    channel: &IbcChannel,
    counterparty_version: Option<&str>,
) -> Result<(), ContractError> {

    // Check the channel version on the local side
    if channel.version != CATALYST_V1_CHANNEL_VERSION {
        return Err(
            ContractError::InvalidIbcChannelVersion { version: channel.version.clone() }
        );
    }

    // Check the channel version of the remote side. Note this value is only set in OpenTry and OpenAck,
    // and will occur in either 'ibc_channel_open' or 'ibc_channel_connect'. This check assumes that
    // at some point the 'counterparty_version' will be specified. (Code taken from cw20-ics20)
    // TODO do we want to add an extra check to make sure that the counterparty_version is always checked at some point?
    if let Some(version) = counterparty_version {
        if version != CATALYST_V1_CHANNEL_VERSION {
            return Err(
                ContractError::InvalidIbcChannelVersion { version: version.to_string() }
            );
        }
    }

    //TODO channel ordering type not enforced. Do we want to enforce an unordered channel (like cw20-ics20)

    Ok(())
}




// Channel communication ********************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    unimplemented!();
}