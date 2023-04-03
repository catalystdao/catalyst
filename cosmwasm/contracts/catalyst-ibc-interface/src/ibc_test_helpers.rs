#![cfg(test)]

use crate::{ibc::{ibc_channel_open, ibc_channel_connect, CATALYST_V1_CHANNEL_VERSION, ibc_channel_close}, state::IbcChannelInfo};
use cosmwasm_std::{testing::mock_env, IbcChannelOpenMsg, IbcChannelConnectMsg, IbcChannel, IbcEndpoint, IbcOrder, DepsMut, IbcChannelCloseMsg};

pub const TEST_LOCAL_PORT: &str = "ibc:wasmlocalport";
pub const TEST_REMOTE_PORT: &str = "ibc:wasmremoteport";
pub const TEST_CONNECTION_ID: &str = "ibc-connection";

pub fn mock_channel(channel_id: &str, order: IbcOrder, version: &str) -> IbcChannel {
    IbcChannel::new(
        IbcEndpoint {
            port_id: TEST_LOCAL_PORT.into(),
            channel_id: channel_id.into(),
        },
        IbcEndpoint {
            port_id: TEST_REMOTE_PORT.into(),
            channel_id: format!("{}-remote", channel_id),   // Make up an id for the remote endpoint
        },
        order,
        version,
        TEST_CONNECTION_ID,
    )
}

pub fn mock_channel_info(channel_id: &str) -> IbcChannelInfo {
    IbcChannelInfo {
        endpoint: IbcEndpoint {
            port_id: TEST_LOCAL_PORT.into(),
            channel_id: channel_id.into(),
        },
        counterparty_endpoint: IbcEndpoint {
            port_id: TEST_REMOTE_PORT.into(),
            channel_id: format!("{}-remote", channel_id),   // Make up an id for the remote endpoint
        },
        connection_id: TEST_CONNECTION_ID.to_string()
    }
}

pub fn open_channel(mut deps: DepsMut, channel_id: &str, order: Option<IbcOrder>, version: Option<&str>) {

    let channel_version = version.unwrap_or(CATALYST_V1_CHANNEL_VERSION);

    let channel = mock_channel(
        channel_id,
        order.unwrap_or(IbcOrder::Unordered),
        channel_version
    );

    ibc_channel_open(
        deps.branch(),
        mock_env(),
        IbcChannelOpenMsg::new_init(channel.clone())
    ).unwrap();

    ibc_channel_connect(
        deps,
        mock_env(), 
        IbcChannelConnectMsg::new_ack(channel, channel_version)
    ).unwrap();
}

pub fn close_channel(mut deps: DepsMut, channel_id: &str, order: Option<IbcOrder>, version: Option<&str>) {

    let channel_version = version.unwrap_or(CATALYST_V1_CHANNEL_VERSION);

    let channel = mock_channel(
        channel_id,
        order.unwrap_or(IbcOrder::Unordered),
        channel_version
    );

    ibc_channel_close(
        deps.branch(),
        mock_env(),
        IbcChannelCloseMsg::CloseInit { channel }
    ).unwrap();
}