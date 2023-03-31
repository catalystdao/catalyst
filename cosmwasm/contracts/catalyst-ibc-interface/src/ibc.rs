use cosmwasm_std::{
    entry_point, DepsMut, Env, StdResult, IbcChannelOpenMsg, IbcChannelConnectMsg, IbcBasicResponse, IbcChannelCloseMsg, 
    IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg, IbcChannel, IbcPacket, Binary, Uint128, CosmosMsg, to_binary, SubMsg, Reply, Response, SubMsgResult
};
use ethnum::U256;

use swap_pool_common::msg::ExecuteMsg as SwapPoolExecuteMsg;

use crate::{ContractError, state::{IbcChannelInfo, OPEN_CHANNELS}, catalyst_ibc_payload::{CTX0_ASSET_SWAP, CTX1_LIQUIDITY_SWAP, FROM_POOL_LENGTH_POS, FROM_POOL_START, TO_POOL_LENGTH_POS, TO_POOL_START, CONTEXT_POS, TO_ACCOUNT_POS, TO_ACCOUNT_START, UNITS_START, UNITS_END, CTX0_TO_ASSET_INDEX_POS, CTX0_MIN_OUT_END, CTX0_MIN_OUT_START, CTX0_FROM_ASSET_POS, CTX0_SWAP_HASH_START, CTX0_SWAP_HASH_END, CTX0_DATA_START, CTX0_DATA_LENGTH_END, CTX0_DATA_LENGTH_START, CTX1_MIN_OUT_END, CTX1_SWAP_HASH_END, CTX1_DATA_LENGTH_END, CTX1_DATA_START, CTX1_MIN_OUT_START, CTX1_SWAP_HASH_START, CTX1_DATA_LENGTH_START, CTX0_FROM_AMOUNT_START, CTX0_FROM_AMOUNT_END, CTX0_FROM_ASSET_START, CTX0_BLOCK_NUMBER_END, CTX0_BLOCK_NUMBER_START, CTX1_FROM_AMOUNT_START, CTX1_FROM_AMOUNT_END, CTX1_BLOCK_NUMBER_START, CTX1_BLOCK_NUMBER_END}, error::Never};


// NOTE: Large parts of this IBC section are based on the cw20-ics20 example repository.


// IBC Interface constants 
pub const CATALYST_V1_CHANNEL_VERSION: &str = "catalyst-v1";

pub const RECEIVE_REPLY_ID: u64 = 0x100;

pub const ACK_SUCCESS: u8 = 0;
pub const ACK_FAIL: u8 = 1;



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

fn ack_success() -> Binary {
    Into::<Binary>::into(vec![ACK_SUCCESS])     //TODO overhaul ack success format
}

fn ack_fail(_err: String) -> Binary {
    Into::<Binary>::into(vec![ACK_FAIL])        //TODO overhaul ack fail format
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    deps: DepsMut,
    _env: Env,
    msg: IbcPacketReceiveMsg,
) -> Result<IbcReceiveResponse, Never> {

    on_packet_receive(deps, msg.packet).or_else(|err| {
        Ok(IbcReceiveResponse::new()            //TODO add attributes?
            .set_ack(ack_fail(err.to_string()))
        )
    })

}

fn on_packet_receive(
    deps: DepsMut,
    packet: IbcPacket
) -> Result<IbcReceiveResponse, ContractError> {

    let data = packet.data.to_vec();

    let context = data.get(CONTEXT_POS).ok_or(ContractError::PayloadDecodingError {})?;

    let mut offset: usize = 0;

    // Extract the common payload

    // From pool
    let from_pool_length: usize = *data.get(FROM_POOL_LENGTH_POS)
        .ok_or(ContractError::PayloadDecodingError {})? as usize;

    let from_pool = String::from_utf8(
        data.get(
            FROM_POOL_START ..
            FROM_POOL_START + from_pool_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
    ).map_err(|_| ContractError::PayloadDecodingError {})?;
    
    offset += from_pool_length;


    // To pool
    let to_pool_length: usize = *data.get(TO_POOL_LENGTH_POS + offset)
        .ok_or(ContractError::PayloadDecodingError {})? as usize;

    let to_pool = String::from_utf8(
        data.get(
            TO_POOL_START + offset ..
            TO_POOL_START + offset + to_pool_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
    ).map_err(|_| ContractError::PayloadDecodingError {})?;
    
    offset += to_pool_length;

    // Verify to pool
    deps.api.addr_validate(&to_pool)?;


    // To account
    let to_account_length: usize = *data.get(TO_ACCOUNT_POS + offset)
        .ok_or(ContractError::PayloadDecodingError {})? as usize;

    let to_account = String::from_utf8(
        data.get(
            TO_ACCOUNT_START + offset ..
            TO_ACCOUNT_START + offset + to_account_length
        ).ok_or(ContractError::PayloadDecodingError {})?.to_vec()
    ).map_err(|_| ContractError::PayloadDecodingError {})?;
    
    offset += to_account_length;

    // Verify to account
    deps.api.addr_validate(&to_account)?;


    // Units
    let u = U256::from_be_bytes(
        data.get(
            UNITS_START + offset .. UNITS_END + offset
        ).ok_or(
            ContractError::PayloadDecodingError {}
        )?.try_into().unwrap()                          // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart, this should never panic //TODO overhaul
    );


    // Extract the context dependent payload and build up contract invocation submessage
    let receive_asset_execute_msg: SwapPoolExecuteMsg<()> = match *context {
        CTX0_ASSET_SWAP => {

            // To asset index
            let to_asset_index = data.get(CTX0_TO_ASSET_INDEX_POS + offset)
                .ok_or(ContractError::PayloadDecodingError {})?.clone();

            // Min out
            let min_out = U256::from_be_bytes(
                data.get(
                    CTX0_MIN_OUT_START + offset .. CTX0_MIN_OUT_END + offset
                ).ok_or(
                    ContractError::PayloadDecodingError {}
                )?.try_into().unwrap()                          // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart, this should never panic //TODO overhaul
            );
            // For CosmWasm pools, min_out should be Uint128
            if min_out > U256::from(Uint128::MAX.u128()) {             //TODO overhaul - more efficient way to do this?
                return Err(ContractError::PayloadDecodingError {});
            }
            let min_out = Uint128::from(min_out.as_u128());

            // From asset length
            let from_asset_length: usize = *data.get(CTX0_FROM_ASSET_POS + offset)
                .ok_or(ContractError::PayloadDecodingError {})? as usize;
            offset += from_asset_length;

            // Swap hash
            let swap_hash = data.get(
                CTX0_SWAP_HASH_START + offset .. CTX0_SWAP_HASH_END + offset
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();

            // Calldata
            let calldata_length: usize = u16::from_be_bytes(
                data.get(
                    CTX0_DATA_LENGTH_START + offset .. CTX0_DATA_LENGTH_END + offset
                ).ok_or(
                    ContractError::PayloadDecodingError {}
                )?.try_into().unwrap()                          // If 'CTX0_DATA_LENGTH_START' and 'CTX0_DATA_LENGTH_END' are 2 bytes apart, this should never panic //TODO overhaul
            ) as usize;

            let calldata = data.get(
                CTX0_DATA_START + offset ..
                CTX0_DATA_START + offset + calldata_length
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();


            // Build execute message
            Ok::<SwapPoolExecuteMsg<()>, ContractError>(
                SwapPoolExecuteMsg::ReceiveAsset {
                    channel_id: packet.dest.channel_id,
                    from_pool,
                    to_asset_index,
                    to_account,
                    u,
                    min_out,
                    swap_hash,
                    calldata
                }
            )

        },
        CTX1_LIQUIDITY_SWAP => {

            // Min out
            let min_out = U256::from_be_bytes(
                data.get(
                    CTX1_MIN_OUT_START + offset .. CTX1_MIN_OUT_END + offset
                ).ok_or(
                    ContractError::PayloadDecodingError {}
                )?.try_into().unwrap()                          // If 'UNITS_START' and 'UNITS_END' are 32 bytes apart, this should never panic //TODO overhaul
            );
            // For CosmWasm pools, min_out should be Uint128
            if min_out > U256::from(Uint128::MAX.u128()) {             //TODO overhaul - more efficient way to do this?
                return Err(ContractError::PayloadDecodingError {});
            }
            let min_out = Uint128::from(min_out.as_u128());

            // Swap hash
            let swap_hash = data.get(
                CTX1_SWAP_HASH_START + offset .. CTX1_SWAP_HASH_END + offset
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();

            // Calldata
            let calldata_length: usize = u16::from_be_bytes(
                data.get(
                    CTX1_DATA_LENGTH_START + offset .. CTX1_DATA_LENGTH_END + offset
                ).ok_or(
                    ContractError::PayloadDecodingError {}
                )?.try_into().unwrap()                          // If 'CTX1_DATA_LENGTH_START' and 'CTX1_DATA_LENGTH_END' are 2 bytes apart, this should never panic //TODO overhaul
            ) as usize;

            let calldata = data.get(
                CTX1_DATA_START + offset ..
                CTX1_DATA_START + offset + calldata_length
            ).ok_or(ContractError::PayloadDecodingError {})?.to_vec();


            // Build execute message
            Ok::<SwapPoolExecuteMsg<()>, ContractError>(
                SwapPoolExecuteMsg::ReceiveLiquidity {
                    channel_id: packet.dest.channel_id,
                    from_pool,
                    to_account,
                    u,
                    min_out,
                    swap_hash,
                    calldata
                }
            )

        },
        _ => return Err(ContractError::PayloadDecodingError {})
    }?;

    let sub_msg = SubMsg::reply_always(
        cosmwasm_std::WasmMsg::Execute {
            contract_addr: to_pool,
            msg: to_binary(&receive_asset_execute_msg)?,
            funds: vec![]
        },
        RECEIVE_REPLY_ID
    );

    Ok(IbcReceiveResponse::new()        //TODO add attributes?
        .set_ack(ack_success())
        .add_submessage(sub_msg)
    )
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


// If, on IBC packet receive, the triggered submessage errors, return a custom fail ack.
// TODO overhaul: in theory the following is not needed, but is used to return a custom
// TODO           ack on submessage error. This is done by overriding the 'data' field
// TODO           of the response via '.set_data(ack_fail(err))'.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    _deps: DepsMut,
    _env: Env,
    reply: Reply
) -> Result<Response, ContractError> {
    match reply.id {
        RECEIVE_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => Ok(Response::new()),
            SubMsgResult::Err(err) => Ok(Response::new().set_data(ack_fail(err)))
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}