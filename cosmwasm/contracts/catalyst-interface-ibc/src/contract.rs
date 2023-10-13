#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, to_binary, IbcQuery, PortIdResponse, Order, Uint128, Reply, SubMsgResult};
use cw2::set_contract_version;
use catalyst_types::U256;
use catalyst_interface_common::{bindings::{InterfaceResponse, CustomMsg}, state::{encode_send_cross_chain_liquidity, encode_send_cross_chain_asset, underwrite, underwrite_and_check_connection, wrap_sub_msgs, query_underwrite_identifier, set_max_underwriting_duration, expire_underwrite, update_owner, ack_success, ack_fail, setup, handle_reply}, msg::{InstantiateMsg, ExecuteMsg}, ContractError};
use crate::{msg::{QueryMsg, PortResponse, ListChannelsResponse}, state::{OPEN_CHANNELS, TRANSACTION_TIMEOUT_SECONDS, WRAPPED_MESSAGES_REPLY_ID, MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS, MAX_UNDERWRITE_DURATION_ALLOWED_BLOCKS}};


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
) -> Result<InterfaceResponse, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    setup(
        deps,
        info,
        MAX_UNDERWRITE_DURATION_INITIAL_BLOCKS,
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
    msg: ExecuteMsg<CustomMsg>,
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

        ExecuteMsg::SetMaxUnderwriteDuration {
            new_max_underwrite_duration
        } => set_max_underwriting_duration(
            &mut deps,
            &info,
            new_max_underwrite_duration,
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
) -> Result<InterfaceResponse, ContractError> {

    // Run the common reply handler before handling any custom replies
    let common_response = handle_reply(
        deps,
        env,
        reply.clone()
    )?;

    if let Some(response) = common_response {
        return Ok(response);
    }

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




// Tests
// ************************************************************************************************

#[cfg(test)]
mod catalyst_interface_ibc_tests {

    use crate::test_helpers::{open_channel, mock_channel_info, close_channel};

    use super::*;
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary};

    pub const DEPLOYER_ADDR: &str = "deployer_addr";



    // Channel Management Tests
    // ********************************************************************************************

    #[test]
    fn test_instantiate() {

        let mut deps = mock_dependencies();
      

        // Tested action: instantiate contract
        let response = instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();


        // No response attributes are expected
        assert_eq!(response.attributes.len(), 0usize);

        //TODO add more checks?

    }


    #[test]
    fn test_open_channel_and_query() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        // Add mock channel
        let channel_id = "mock-channel-1";


        // Tested action: open channel
        open_channel(deps.as_mut(), channel_id, None, None);


        // Query open channels
        let open_channels: ListChannelsResponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
        ).unwrap();

        assert_eq!(open_channels.channels.len(), 1);
        assert_eq!(open_channels.channels[0], mock_channel_info(channel_id));

    }


    #[test]
    fn test_open_multiple_channels_and_query() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        // Open mock channels
        let channel_id_1 = "mock-channel-1";
        let channel_id_2 = "mock-channel-2";

        open_channel(deps.as_mut(), channel_id_1, None, None);
        open_channel(deps.as_mut(), channel_id_2, None, None);


        // Tested action: query open channels
        let open_channels: ListChannelsResponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
        ).unwrap();


        assert_eq!(open_channels.channels.len(), 2);
        assert_eq!(open_channels.channels[0], mock_channel_info(channel_id_1));
        assert_eq!(open_channels.channels[1], mock_channel_info(channel_id_2));

    }


    #[test]
    fn test_query_open_channels_empty() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();


        // Tested action: query open channels
        let open_channels: ListChannelsResponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
        ).unwrap();


        assert_eq!(open_channels.channels.len(), 0);

    }


    #[test]
    fn test_delete_channel_and_query() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        // Open mock channel
        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Query open channels
        let open_channels: ListChannelsResponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
        ).unwrap();

        assert_eq!(open_channels.channels.len(), 1);
        assert_eq!(open_channels.channels[0], mock_channel_info(channel_id));


        // Tested action: delete channel
        close_channel(deps.as_mut(), channel_id, None, None);


        // Query open channels
        let open_channels: ListChannelsResponse = from_binary(
            &query(deps.as_ref(), mock_env(), QueryMsg::ListChannels {}).unwrap()
        ).unwrap();

        assert_eq!(open_channels.channels.len(), 0);

    }


    // //TODO test_port not working! 'query(...).unwrap' fails
    // #[test]
    // fn test_port() {


    //     let mut deps = mock_dependencies();
      
    //     // Instantiate contract
    //     instantiate(
    //         deps.as_mut(),
    //         mock_env(),
    //         mock_info(DEPLOYER_ADDR, &vec![]),
    //         InstantiateMsg {}
    //     ).unwrap();


    //     // Tested action: query port
    //     let port: PortResponse = from_binary(
    //         &query(deps.as_ref(), mock_env(), QueryMsg::Port {}).unwrap()
    //     ).unwrap();


    //     assert_eq!(port.port_id, TEST_LOCAL_PORT);

    // }


    //TODO add tests to open/close channels with invalid configuration
}
