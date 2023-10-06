#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, to_binary, IbcQuery, PortIdResponse, Order, Uint128, Reply, SubMsgResult};
use cw2::set_contract_version;
use catalyst_types::U256;
use catalyst_vault_common::bindings::{VaultResponse, CustomMsg}; //TODO import from catalyst_interface_common
use catalyst_interface_common::{state::{encode_send_cross_chain_liquidity, encode_send_cross_chain_asset, underwrite, underwrite_and_check_connection, wrap_sub_msgs, query_underwrite_identifier, set_max_underwriting_duration, expire_underwrite, update_owner, ack_success, ack_fail, setup, handle_reply}, msg::{InstantiateMsg, ExecuteMsg}, ContractError};
use crate::{msg::{QueryMsg, PortResponse, ListChannelsResponse}, state::{OPEN_CHANNELS, TRANSACTION_TIMEOUT_SECONDS, WRAPPED_MESSAGES_REPLY_ID}};


// Version information
const CONTRACT_NAME: &str = "catalyst-interface-ibc";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");




// Instantiation
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<VaultResponse, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(deps, info)
}




// Execution
// ************************************************************************************************

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg<CustomMsg>,
) -> Result<VaultResponse, ContractError> {
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

        ExecuteMsg::SetMaxUnderwriteDuration {
            new_max_underwrite_duration
        } => set_max_underwriting_duration(
            &mut deps,
            &info,
            new_max_underwrite_duration
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

        ExecuteMsg::WrapSubMsgs {
            sub_msgs
        } => wrap_sub_msgs(
            &info,
            &env,
            sub_msgs
        ),



        // Ownership msgs
        ExecuteMsg::TransferOwnership {
            new_owner
        } => update_owner(deps, info, new_owner)

    }
}


/// Pack the arguments of a 'send_asset' transaction into a byte array following Catalyst's
/// payload definition, and initiate an IBC cross chain transaction.
/// 
/// **NOTE**: This call is **permissionless**. The recipient of the transaction must validate
/// the sender of the transaction.
/// 
/// # Arguments: 
/// * `channel_id` - The target chain identifier.
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `to_asset_index` - The destination asset index.
/// * `u` - The outgoing 'units'.
/// * `min_out` - The mininum `to_asset` output amount to get on the target vault.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `from_asset` - The source asset.
/// * `underwrite_incentive_x16` - The share of the swap return that is offered to an underwriter as incentive.
/// * `block_number` - The block number at which the transaction has been committed.
/// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
/// 
fn execute_send_cross_chain_asset(
    env: Env,
    info: MessageInfo,
    channel_id: String,
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
) -> Result<VaultResponse, ContractError> {

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

    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: payload,
        timeout: env.block.time.plus_seconds(TRANSACTION_TIMEOUT_SECONDS).into()
    };

    Ok(Response::new()
        .add_message(ibc_msg)
    )
}


/// Pack the arguments of a 'send_liquidity' transaction into a byte array following Catalyst's
/// payload definition, and initiate an IBC cross chain transaction.
/// 
/// **NOTE**: This call is **permissionless**. The recipient of the transaction must validate
/// the sender of the transaction.
/// 
/// # Arguments: 
/// * `channel_id` - The target chain identifier.
/// * `to_vault` - The target vault on the target chain (Catalyst encoded).
/// * `to_account` - The recipient of the swap on the target chain (Catalyst encoded).
/// * `u` - The outgoing 'units'.
/// * `min_vault_tokens` - The mininum vault tokens output amount to get on the target vault.
/// * `min_reference_asset` - The mininum reference asset value on the target vault.
/// * `from_amount` - The `from_asset` amount sold to the vault.
/// * `block_number` - The block number at which the transaction has been committed.
/// * `calldata` - Arbitrary data to be executed on the target chain upon successful execution of the swap.
/// 
fn execute_send_cross_chain_liquidity(
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_vault: Binary,
    to_account: Binary,
    u: U256,
    min_vault_tokens: U256,
    min_reference_asset: U256,
    from_amount: Uint128,
    block_number: u32,
    calldata: Binary
) -> Result<VaultResponse, ContractError> {

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

    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: payload,
        timeout: env.block.time.plus_seconds(TRANSACTION_TIMEOUT_SECONDS).into()
    };

    Ok(Response::new()
        .add_message(ibc_msg)
    )
}




// Reply
// ************************************************************************************************

// If the vault invocation errors, return a custom 'fail' ack.
// NOTE: this 'reply' code is needed, as the Catalyst protocol is not compatible with the default
// 'failed-ack' that is generated by CosmWasm. 
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut,
    env: Env,
    reply: Reply
) -> Result<VaultResponse, ContractError> {

    // Run the common reply handler before handling any custom replies
    let common_response = handle_reply(
        deps,
        env,
        reply.clone()
    )?;

    if let Some(response) = common_response {
        return Ok(response);
    }

    //TODO move to 'common'?
    // Match ibc-specific replies
    match reply.id {
        
        WRAPPED_MESSAGES_REPLY_ID => match reply.result {
            SubMsgResult::Ok(_) => {
                // Set the custom 'success-ack' for successful executions.
                Ok(Response::new().set_data(ack_success()))
            },
            SubMsgResult::Err(_) => {
                // Set the custom 'failed-ack' for unsuccessful executions.
                Ok(Response::new().set_data(ack_fail()))
            }
        },
        _ => Err(ContractError::UnknownReplyId { id: reply.id }),
    }
}





// Query
// ************************************************************************************************

// The following 'query' code has been taken in part from the cw20-ics20 contract of the cw-plus 
// repository.

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Port {} => to_binary(&query_port(deps)?),
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?),
        QueryMsg::UnderwriteIdentifier {
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        } => to_binary(
            &query_underwrite_identifier(
                to_vault,
                to_asset_ref,
                u,
                min_out,
                to_account,
                underwrite_incentive_x16,
                calldata
            )?
        )
    }
}


fn query_port(deps: Deps) -> StdResult<PortResponse> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;
    Ok(PortResponse { port_id })
}


fn query_list(deps: Deps) -> StdResult<ListChannelsResponse> {
    let channels = OPEN_CHANNELS
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;
    Ok(ListChannelsResponse { channels })
}



// Tests ******************************************************************************************

// #[cfg(test)]
// mod catalyst_interface_common_tests {

//     use crate::{ibc_test_helpers::{open_channel, mock_channel_info, TEST_LOCAL_PORT, close_channel, TEST_REMOTE_PORT}, catalyst_ibc_payload::CatalystV1Packet, ibc::{ibc_packet_receive, RECEIVE_REPLY_ID, ibc_packet_ack, ibc_packet_timeout, reply}};

//     use super::*;
//     use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary, Uint128, SubMsg, IbcTimeout, IbcPacket, IbcEndpoint, IbcPacketReceiveMsg, IbcPacketAckMsg, IbcAcknowledgement, IbcPacketTimeoutMsg, Reply, SubMsgResponse, SubMsgResult};
//     use catalyst_types::u256;

//     pub const DEPLOYER_ADDR: &str = "deployer_addr";



//     // Helpers ******************************************************************************************************************

//     fn mock_ibc_packet(
//         channel_id: &str,
//         from_vault: &str,
//         send_msg: ExecuteMsg<CustomMsg>,
//         from_amount: Option<U256>    // Allow to override the send_msg from_amount to provide invalid configs
//     ) -> IbcPacket {
//         IbcPacket::new(
//             Binary::from(build_payload(from_vault.as_bytes(), send_msg, from_amount).unwrap()),
//             IbcEndpoint {
//                 port_id: TEST_REMOTE_PORT.to_string(),
//                 channel_id: format!("{}-remote", channel_id),
//             },
//             IbcEndpoint {
//                 port_id: TEST_LOCAL_PORT.to_string(),
//                 channel_id: channel_id.to_string(),
//             },
//             7,
//             mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT_SECONDS).into(),     // Note mock_env() always returns the same block time
//         )
//     }
    

//     // Send asset helpers

//     fn mock_send_asset_msg(
//         channel_id: &str,
//         to_vault: Vec<u8>,
//         min_out: Option<U256>          // Allow to override the default value to provide invalid configs
//     ) -> ExecuteMsg<CustomMsg> {
//         ExecuteMsg::SendCrossChainAsset {
//             channel_id: channel_id.into(),
//             to_vault: CatalystEncodedAddress::try_encode(to_vault.as_ref()).unwrap().to_binary(),
//             to_account: CatalystEncodedAddress::try_encode(b"to_account").unwrap().to_binary(),
//             to_asset_index: 1u8,
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             min_out: min_out.unwrap_or(
//                 u256!("323476719582585693194107115743132847255")                                                // Some large Uint128 number (as U256)
//             ),
//             from_amount: Uint128::from(4920222095670429824873974121747892731u128),                          // Some large Uint128 number
//             from_asset: "from_asset".to_string(),
//             underwrite_incentive_x16: 1001u16,
//             block_number: 1356u32,
//             calldata: Binary(vec![])
//         }
//     }

//     fn mock_vault_receive_asset_msg(
//         channel_id: &str,
//         from_vault: Vec<u8>,
//     ) -> catalyst_vault_common::msg::ExecuteMsg<(), CustomMsg> {
//         catalyst_vault_common::msg::ExecuteMsg::ReceiveAsset {
//             channel_id: channel_id.into(),
//             from_vault: CatalystEncodedAddress::try_encode(from_vault.as_ref()).unwrap().to_binary(),
//             to_asset_index: 1u8,
//             to_account: "to_account".to_string(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             min_out: Uint128::from(323476719582585693194107115743132847255u128),                                // Some large Uint128 number
//             from_asset: CatalystEncodedAddress::try_encode("from_asset".as_bytes()).unwrap().to_binary(),
//             from_amount: U256::from(4920222095670429824873974121747892731u128),
//             from_block_number_mod: 1356u32
//         }
//     }

//     fn mock_vault_send_asset_success_msg(
//         channel_id: &str,
//     ) -> catalyst_vault_common::msg::ExecuteMsg<(), CustomMsg> {
//         catalyst_vault_common::msg::ExecuteMsg::OnSendAssetSuccess {
//             channel_id: channel_id.into(),
//             to_account: CatalystEncodedAddress::try_encode(b"to_account").unwrap().to_binary(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             escrow_amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
//             asset_ref: "from_asset".to_string(),
//             block_number_mod: 1356u32
//         }
//     }

//     fn mock_vault_send_asset_failure_msg(
//         channel_id: &str,
//     ) -> catalyst_vault_common::msg::ExecuteMsg<(), CustomMsg> {
//         catalyst_vault_common::msg::ExecuteMsg::OnSendAssetFailure {
//             channel_id: channel_id.into(),
//             to_account: CatalystEncodedAddress::try_encode(b"to_account").unwrap().to_binary(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             escrow_amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
//             asset_ref: "from_asset".to_string(),
//             block_number_mod: 1356u32
//         }
//     }


//     // Send liquidity helpers
    
//     fn mock_send_liquidity_msg(
//         channel_id: &str,
//         to_vault: Vec<u8>,
//         min_vault_tokens: Option<U256>,          // Allow to override the default value to provide invalid configs
//         min_reference_asset: Option<U256>       // Allow to override the default value to provide invalid configs
//     ) -> ExecuteMsg<CustomMsg> {
//         ExecuteMsg::SendCrossChainLiquidity {
//             channel_id: channel_id.into(),
//             to_vault: CatalystEncodedAddress::try_encode(to_vault.as_ref()).unwrap().to_binary(),
//             to_account: CatalystEncodedAddress::try_encode(b"to_account").unwrap().to_binary(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             min_vault_tokens: min_vault_tokens.unwrap_or(
//                 u256!("323476719582585693194107115743132847255")                                                // Some large Uint128 number (as U256)
//             ),
//             min_reference_asset: min_reference_asset.unwrap_or(
//                 u256!("1385371954613879816514345798135479")                                                     // Some large Uint128 number (as U256)
//             ),
//             from_amount: Uint128::from(4920222095670429824873974121747892731u128),                              // Some large Uint128 number
//             block_number: 1356u32,
//             calldata: Binary(vec![])
//         }
//     }

//     fn mock_vault_receive_liquidity_msg(
//         channel_id: &str,
//         from_vault: Vec<u8>,
//     ) -> catalyst_vault_common::msg::ExecuteMsg<()> {
//         catalyst_vault_common::msg::ExecuteMsg::ReceiveLiquidity {
//             channel_id: channel_id.into(),
//             from_vault: CatalystEncodedAddress::try_encode(from_vault.as_ref()).unwrap().to_binary(),
//             to_account: "to_account".to_string(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             min_vault_tokens: Uint128::from(323476719582585693194107115743132847255u128),                        // Some large Uint128 number
//             min_reference_asset: Uint128::from(1385371954613879816514345798135479u128),                         // Some large Uint128 number
//             from_amount: U256::from(4920222095670429824873974121747892731u128),
//             from_block_number_mod: 1356u32
//         }
//     }

//     fn mock_vault_send_liquidity_success_msg(
//         channel_id: &str,
//     ) -> catalyst_vault_common::msg::ExecuteMsg<()> {
//         catalyst_vault_common::msg::ExecuteMsg::OnSendLiquiditySuccess {
//             channel_id: channel_id.into(),
//             to_account: CatalystEncodedAddress::try_encode(b"to_account").unwrap().to_binary(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             escrow_amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
//             block_number_mod: 1356u32
//         }
//     }

//     fn mock_vault_send_liquidity_failure_msg(
//         channel_id: &str,
//     ) -> catalyst_vault_common::msg::ExecuteMsg<()> {
//         catalyst_vault_common::msg::ExecuteMsg::OnSendLiquidityFailure {
//             channel_id: channel_id.into(),
//             to_account: CatalystEncodedAddress::try_encode(b"to_account").unwrap().to_binary(),
//             u: u256!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
//             escrow_amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
//             block_number_mod: 1356u32
//         }
//     }


//     fn build_payload(
//         from_vault: &[u8],
//         msg: ExecuteMsg<CustomMsg>,
//         override_from_amount: Option<U256>    // Allow to override the msg 'from_amount' to provide invalid configs
//     ) -> Result<Binary, ContractError> {
//         let packet = match msg {
//             ExecuteMsg::SendCrossChainAsset {
//                 channel_id: _,
//                 to_vault,
//                 to_account,
//                 to_asset_index,
//                 u,
//                 min_out,
//                 from_amount,
//                 from_asset,
//                 underwrite_incentive_x16,
//                 block_number,
//                 calldata
//             } => CatalystV1Packet::SendAsset(
//                 CatalystV1SendAssetPayload {
//                     from_vault: CatalystEncodedAddress::try_encode(from_vault.as_ref()).unwrap(),
//                     to_vault: CatalystEncodedAddress::from_slice_unchecked(to_vault.as_ref()),
//                     to_account: CatalystEncodedAddress::from_slice_unchecked(to_account.as_ref()),
//                     u,
//                     variable_payload: SendAssetVariablePayload {
//                         to_asset_index,
//                         min_out,
//                         from_amount: override_from_amount.unwrap_or(U256::from(from_amount)),
//                         from_asset: CatalystEncodedAddress::try_encode(from_asset.as_ref()).unwrap(),
//                         block_number,
//                         underwrite_incentive_x16,
//                         calldata
//                     },
//                 }
//             ),
//             ExecuteMsg::SendCrossChainLiquidity {
//                 channel_id: _,
//                 to_vault,
//                 to_account,
//                 u,
//                 min_vault_tokens,
//                 min_reference_asset,
//                 from_amount,
//                 block_number,
//                 calldata
//             } => CatalystV1Packet::SendLiquidity(
//                 CatalystV1SendLiquidityPayload {
//                     from_vault: CatalystEncodedAddress::try_encode(from_vault.as_ref()).unwrap(),
//                     to_vault: CatalystEncodedAddress::from_slice_unchecked(to_vault.as_ref()),
//                     to_account: CatalystEncodedAddress::from_slice_unchecked(to_account.as_ref()),
//                     u,
//                     variable_payload: SendLiquidityVariablePayload {
//                         min_vault_tokens,
//                         min_reference_asset,
//                         from_amount: override_from_amount.unwrap_or(U256::from(from_amount)),
//                         block_number,
//                         calldata
//                     },
//                 }
//             ),
//             _ => panic!("Payload building not implemented for the given message")
//         };

//         packet.try_encode()
//     }



//     // Channel Management Tests *************************************************************************************************

//     #[test]
//     fn test_instantiate() {

//         let mut deps = mock_dependencies();
      

//         // Tested action: instantiate contract
//         let response = instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();


//         // No response attributes are expected
//         assert_eq!(response.attributes.len(), 0usize);

//         //TODO add more checks?

//     }


//     #[test]
//     fn test_open_channel_and_query() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         // Add mock channel
//         let channel_id = "mock-channel-1";


//         // Tested action: open channel
//         open_channel(deps.as_mut(), channel_id, None, None);


//         // Query open channels
//         let open_channels: ListChannelsResponse = from_binary(
//             &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
//         ).unwrap();

//         assert_eq!(open_channels.channels.len(), 1);
//         assert_eq!(open_channels.channels[0], mock_channel_info(channel_id));

//     }


//     #[test]
//     fn test_open_multiple_channels_and_query() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         // Open mock channels
//         let channel_id_1 = "mock-channel-1";
//         let channel_id_2 = "mock-channel-2";

//         open_channel(deps.as_mut(), channel_id_1, None, None);
//         open_channel(deps.as_mut(), channel_id_2, None, None);


//         // Tested action: query open channels
//         let open_channels: ListChannelsResponse = from_binary(
//             &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
//         ).unwrap();


//         assert_eq!(open_channels.channels.len(), 2);
//         assert_eq!(open_channels.channels[0], mock_channel_info(channel_id_1));
//         assert_eq!(open_channels.channels[1], mock_channel_info(channel_id_2));

//     }


//     #[test]
//     fn test_query_open_channels_empty() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();


//         // Tested action: query open channels
//         let open_channels: ListChannelsResponse = from_binary(
//             &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
//         ).unwrap();


//         assert_eq!(open_channels.channels.len(), 0);

//     }


//     #[test]
//     fn test_delete_channel_and_query() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         // Open mock channel
//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Query open channels
//         let open_channels: ListChannelsResponse = from_binary(
//             &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
//         ).unwrap();

//         assert_eq!(open_channels.channels.len(), 1);
//         assert_eq!(open_channels.channels[0], mock_channel_info(channel_id));


//         // Tested action: delete channel
//         close_channel(deps.as_mut(), channel_id, None, None);


//         // Query open channels
//         let open_channels: ListChannelsResponse = from_binary(
//             &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
//         ).unwrap();

//         assert_eq!(open_channels.channels.len(), 0);

//     }


//     // //TODO test_port not working! 'query(...).unwrap' fails
//     // #[test]
//     // fn test_port() {


//     //     let mut deps = mock_dependencies();
      
//     //     // Instantiate contract
//     //     instantiate(
//     //         deps.as_mut(),
//     //         mock_env(),
//     //         mock_info(DEPLOYER_ADDR, &vec![]),
//     //         InstantiateMsg {}
//     //     ).unwrap();


//     //     // Tested action: query port
//     //     let port: PortResponse = from_binary(
//     //         &query(deps.as_ref(), mock_env(), QueryMsg::Port {}).unwrap()
//     //     ).unwrap();


//     //     assert_eq!(port.port_id, TEST_LOCAL_PORT);

//     // }


//     //TODO add tests to open/close channels with invalid configuration



//     // Send Asset Tests *********************************************************************************************************

//     #[test]
//     fn test_send_asset() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = b"to_vault";
//         let execute_msg = mock_send_asset_msg(channel_id, to_vault.to_vec(), None);


//         // Tested action: send asset
//         let response_result = execute(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(from_vault, &[]),
//             execute_msg.clone()
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Response should include a message to send the IBC message
//         assert_eq!(response.messages.len(), 1);

//         // Make sure the IBC message matches the expected one
//         assert_eq!(
//             &response.messages[0],
//             &SubMsg::new(IbcMsg::SendPacket {
//                 channel_id: channel_id.to_string(),
//                 data: build_payload(from_vault.as_bytes(), execute_msg, None).unwrap().into(),
//                 timeout: IbcTimeout::with_timestamp(mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT_SECONDS))
//             })
//         );

//     }


//     //TODO REVIEW TEST UNDERWRITING
//     #[test]
//     fn test_receive_asset() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_asset_msg(channel_id, to_vault.as_bytes().to_vec(), None);
//         let receive_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: receive asset
//         let response_result = ibc_packet_receive(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketReceiveMsg::new(receive_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault is invoked
//         assert_eq!(response.messages.len(), 1);

//         assert_eq!(
//             response.messages[0],
//             SubMsg {
//                 id: RECEIVE_REPLY_ID,
//                 msg: cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: to_vault.to_string(),
//                     msg: to_binary(&mock_vault_receive_asset_msg(channel_id, from_vault.as_bytes().to_vec())).unwrap(),
//                     funds: vec![]
//                 }.into(),
//                 reply_on: cosmwasm_std::ReplyOn::Always,
//                 gas_limit: None

//             }
//         )

//     }


//     //TODO REVIEW TEST UNDERWRITING
//     #[test]
//     fn test_receive_asset_invalid_min_out() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = b"to_vault";
//         let send_msg = mock_send_asset_msg(
//             channel_id,
//             to_vault.to_vec(),
//             Some(U256::MAX)                                // ! Specify a min_out larger than Uint128
//         );
//         let receive_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: receive asset
//         let response_result = ibc_packet_receive(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketReceiveMsg::new(receive_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Check the returned ack
//         assert_eq!(
//             response.acknowledgement.clone(),
//             Binary(vec![1u8])                   // ! Check ack returned has value of 1 (i.e. error)
//         );
    
//         // Check vault is not invoked
//         assert_eq!(response.messages.len(), 0);

//     }


//     #[test]
//     fn test_send_asset_ack() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_asset_msg(channel_id, to_vault.as_bytes().to_vec(), None);
//         let ibc_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);



//         // Tested action: send asset ack SUCCESSFUL
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for success
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault ack is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_asset_success_msg(channel_id)).unwrap(),
//                     funds: vec![]
//                 }
//             )
//         );



//         // Tested action: send asset ack UNSUCCESSFUL
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for failure
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault ack is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_asset_failure_msg(channel_id)).unwrap(),
//                     funds: vec![]
//                 }
//             )
//         );



//         // Tested action: send asset ack INVALID
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![9u8])),         // ! Some invalid response
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault ack is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_asset_failure_msg(channel_id)).unwrap(),    // Invalid responses are treated as failures.
//                     funds: vec![]
//                 }
//             )
//         );

//     }


//     #[test]
//     fn test_send_asset_timeout() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_asset_msg(channel_id, to_vault.as_bytes().to_vec(), None);
//         let ibc_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: send asset timeout
//         let response_result = ibc_packet_timeout(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketTimeoutMsg::new(ibc_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault timeout is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_asset_failure_msg(channel_id)).unwrap(),
//                     funds: vec![]
//                 }
//             )
//         )

//     }


//     #[test]
//     fn test_send_asset_ack_timeout_invalid_from_amount() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_asset_msg(channel_id, to_vault.as_bytes().to_vec(), None);
//         let ibc_packet = mock_ibc_packet(channel_id, from_vault, send_msg, Some(U256::from(Uint128::MAX) + U256::from(1u64)));   // ! Inject an invalid from_amount into the ibc_packet



//         // Tested action: send asset ACK SUCCESSFUL with invalid packet (from_amount)
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for ack-success
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction does not pass
//         assert!(matches!(
//             response_result.err().unwrap(),
//             ContractError::PayloadDecodingError {}
//         ));



//         // Tested action: send asset ACK UNSUCCESSFUL with invalid packet
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for ack-failure
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction does not pass
//         assert!(matches!(
//             response_result.err().unwrap(),
//             ContractError::PayloadDecodingError {}
//         ));



//         // Tested action: send asset TIMEOUT with invalid packet
//         let response_result = ibc_packet_timeout(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketTimeoutMsg::new(                               // ! Test for timeout
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction does not pass
//         assert!(matches!(
//             response_result.err().unwrap(),
//             ContractError::PayloadDecodingError {}
//         ));

//     }



//     // Send Liquidity Tests *****************************************************************************************************

//     #[test]
//     fn test_send_liquidity() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = b"to_vault";
//         let execute_msg = mock_send_liquidity_msg(channel_id, to_vault.to_vec(), None, None);


//         // Tested action: send liquidity
//         let response_result = execute(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(from_vault, &[]),
//             execute_msg.clone()
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Response should include a message to send the IBC message
//         assert_eq!(response.messages.len(), 1);

//         // Make sure the IBC message matches the expected one
//         assert_eq!(
//             &response.messages[0],
//             &SubMsg::new(IbcMsg::SendPacket {
//                 channel_id: channel_id.to_string(),
//                 data: build_payload(from_vault.as_bytes(), execute_msg, None).unwrap().into(),
//                 timeout: IbcTimeout::with_timestamp(mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT_SECONDS))
//             })
//         );

//     }


//     #[test]
//     fn test_receive_liquidity() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_liquidity_msg(channel_id, to_vault.as_bytes().to_vec(), None, None);
//         let receive_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: receive liquidity
//         let response_result = ibc_packet_receive(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketReceiveMsg::new(receive_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault is invoked
//         assert_eq!(response.messages.len(), 1);

//         assert_eq!(
//             response.messages[0],
//             SubMsg {
//                 id: RECEIVE_REPLY_ID,
//                 msg: cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: to_vault.to_string(),
//                     msg: to_binary(&mock_vault_receive_liquidity_msg(channel_id, from_vault.as_bytes().to_vec())).unwrap(),
//                     funds: vec![]
//                 }.into(),
//                 reply_on: cosmwasm_std::ReplyOn::Always,
//                 gas_limit: None

//             }
//         )

//     }


//     #[test]
//     fn test_receive_liquidity_invalid_min_vault_tokens() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = b"to_vault";
//         let send_msg = mock_send_liquidity_msg(
//             channel_id,
//             to_vault.to_vec(),
//             Some(U256::MAX),                                // ! Specify a min_vault_token that is larger than Uint128
//             None,
//         );
//         let receive_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: receive liquidity
//         let response_result = ibc_packet_receive(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketReceiveMsg::new(receive_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Check the returned ack
//         assert_eq!(
//             response.acknowledgement.clone(),
//             Binary(vec![1u8])                   // ! Check ack returned has value of 1 (i.e. error)
//         );
    
//         // Check vault is not invoked
//         assert_eq!(response.messages.len(), 0);

//     }


//     #[test]
//     fn test_receive_liquidity_invalid_min_reference_asset() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = b"to_vault";
//         let send_msg = mock_send_liquidity_msg(
//             channel_id,
//             to_vault.to_vec(),
//             None,
//             Some(U256::MAX)                                // ! Specify a min_reference_asset that is larger than Uint128
//         );
//         let receive_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: receive liquidity
//         let response_result = ibc_packet_receive(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketReceiveMsg::new(receive_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Check the returned ack
//         assert_eq!(
//             response.acknowledgement.clone(),
//             Binary(vec![1u8])                   // ! Check ack returned has value of 1 (i.e. error)
//         );
    
//         // Check vault is not invoked
//         assert_eq!(response.messages.len(), 0);

//     }


//     #[test]
//     fn test_send_liquidity_ack() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_liquidity_msg(channel_id, to_vault.as_bytes().to_vec(), None, None);
//         let ibc_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);



//         // Tested action: send liquidity ack SUCCESSFUL
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for success
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault ack is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_liquidity_success_msg(channel_id)).unwrap(),
//                     funds: vec![]
//                 }
//             )
//         );



//         // Tested action: send liquidity ack UNSUCCESSFUL
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for failure
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault ack is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_liquidity_failure_msg(channel_id)).unwrap(),
//                     funds: vec![]
//                 }
//             )
//         );



//         // Tested action: send liquidity ack INVALID
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![9u8])),         // ! Some invalid response
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault ack is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_liquidity_failure_msg(channel_id)).unwrap(),    // Invalid responses are treated as failures.
//                     funds: vec![]
//                 }
//             )
//         );

//     }


//     #[test]
//     fn test_send_liquidity_timeout() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_liquidity_msg(channel_id, to_vault.as_bytes().to_vec(), None, None);
//         let ibc_packet = mock_ibc_packet(channel_id, from_vault, send_msg, None);


//         // Tested action: send liquidity timeout
//         let response_result = ibc_packet_timeout(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketTimeoutMsg::new(ibc_packet.clone())
//         );


//         // Check the transaction passes
//         let response = response_result.unwrap();
    
//         // Check vault timeout is invoked
//         assert_eq!(response.messages.len(), 1);
//         assert_eq!(
//             response.messages[0],
//             SubMsg::new(
//                 cosmwasm_std::WasmMsg::Execute {
//                     contract_addr: from_vault.to_string(),
//                     msg: to_binary(&mock_vault_send_liquidity_failure_msg(channel_id)).unwrap(),
//                     funds: vec![]
//                 }
//             )
//         )

//     }


//     #[test]
//     fn test_send_liquidity_ack_timeout_invalid_from_amount() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();

//         let channel_id = "mock-channel-1";
//         open_channel(deps.as_mut(), channel_id, None, None);

//         // Get mock params
//         let from_vault = "sender";
//         let to_vault = "to_vault";
//         let send_msg = mock_send_liquidity_msg(channel_id, to_vault.as_bytes().to_vec(), None, None);
//         let ibc_packet = mock_ibc_packet(channel_id, from_vault, send_msg, Some(U256::from(Uint128::MAX) + U256::from(1u64)));   // ! Inject an invalid from_amount into the ibc_packet



//         // Tested action: send liquidity ACK SUCCESSFUL with invalid packet (from_amount)
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for ack-success
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction does not pass
//         assert!(matches!(
//             response_result.err().unwrap(),
//             ContractError::PayloadDecodingError {}
//         ));



//         // Tested action: send liquidity ACK UNSUCCESSFUL with invalid packet
//         let response_result = ibc_packet_ack(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketAckMsg::new(
//                 IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for ack-failure
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction does not pass
//         assert!(matches!(
//             response_result.err().unwrap(),
//             ContractError::PayloadDecodingError {}
//         ));



//         // Tested action: send liquidity TIMEOUT with invalid packet
//         let response_result = ibc_packet_timeout(
//             deps.as_mut(),
//             mock_env(),
//             IbcPacketTimeoutMsg::new(                               // ! Test for timeout
//                 ibc_packet.clone()
//             )
//         );

//         // Check the transaction does not pass
//         assert!(matches!(
//             response_result.err().unwrap(),
//             ContractError::PayloadDecodingError {}
//         ));

//     }



//     // Common Tests *************************************************************************************************************
    

//     #[test]
//     fn test_receive_reply() {

//         let mut deps = mock_dependencies();
      
//         // Instantiate contract and open channel
//         instantiate(
//             deps.as_mut(),
//             mock_env(),
//             mock_info(DEPLOYER_ADDR, &vec![]),
//             InstantiateMsg {}
//         ).unwrap();


//         // Tested action: reply ok
//         let response_result = reply(
//             deps.as_mut(),
//             mock_env(),
//             Reply {
//                 id: RECEIVE_REPLY_ID,
//                 result: SubMsgResult::Ok(
//                     SubMsgResponse { events: vec![], data: None }       // SubMsgResponse contents do not matter
//                 )
//             }
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Check response
//         // TODO overhaul this is the desired result
//         assert_eq!(response.messages.len(), 0);
//         assert_eq!(
//             response.data,
//             Some(Binary(vec![0]))     // ! If the submessage call by 'ibc_packet_receive' is successful, the response 'data' field should be a success ack.
//         );



//         // Tested action: reply error
//         let response_result = reply(
//             deps.as_mut(),
//             mock_env(),
//             Reply {
//                 id: RECEIVE_REPLY_ID,
//                 result: SubMsgResult::Err("".to_string())
//             }
//         );

//         // Check the transaction passes
//         let response = response_result.unwrap();

//         // Check response contains an ack-fail
//         assert_eq!(response.messages.len(), 0);
//         assert_eq!(
//             response.data,
//             Some(Binary(vec![1]))     // ! If the submessage call by 'ibc_packet_receive' returns an error, the response 'data' field should be a failed ack.
//         );

//     }

// }
