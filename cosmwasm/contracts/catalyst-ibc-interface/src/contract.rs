#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, to_binary, IbcQuery, PortIdResponse, Order};
use cw2::set_contract_version;
use ethnum::U256;

use crate::catalyst_ibc_payload::{CatalystV1SendAssetPayload, SendAssetVariablePayload, CatalystV1SendLiquidityPayload, SendLiquidityVariablePayload};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, AssetSwapMetadata, LiquiditySwapMetadata, PortResponse, ListChannelsResponse};
use crate::state::OPEN_CHANNELS;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-ibc-interface";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const TRANSACTION_TIMEOUT: u64 = 2 * 60 * 60;   // 2 hours      //TODO allow this to be set on interface instantiation?
                                                                //TODO allow this to be customized on a per-channel basis?
                                                                //TODO allow this to be overriden on 'sendAsset' and 'sendLiquidity'?


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(
        Response::new()
    )
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {

        ExecuteMsg::SendCrossChainAsset {
            channel_id,
            to_pool,
            to_account,
            to_asset_index,
            u,
            min_out,
            metadata,
            calldata
        } => execute_send_cross_chain_asset(
            env,
            info,
            channel_id,
            to_pool,
            to_account,
            to_asset_index,
            u,
            min_out,
            metadata,
            calldata
        ),

        ExecuteMsg::SendCrossChainLiquidity {
            channel_id,
            to_pool,
            to_account,
            u,
            min_out,
            metadata,
            calldata
        } => execute_send_cross_chain_liquidity(
            env,
            info,
            channel_id,
            to_pool,
            to_account,
            u,
            min_out,
            metadata,
            calldata
        )

    }
}

fn execute_send_cross_chain_asset(
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_pool: Vec<u8>,
    to_account: Vec<u8>,
    to_asset_index: u8,
    u: U256,
    min_out: U256,
    metadata: AssetSwapMetadata,    //TODO do we want this?
    calldata: Vec<u8>
) -> Result<Response, ContractError> {

    // Build payload
    let payload = CatalystV1SendAssetPayload {
        from_pool: info.sender.as_bytes(),
        to_pool: to_pool.as_slice(),
        to_account: to_account.as_slice(),
        u,
        variable_payload: SendAssetVariablePayload {
            to_asset_index,
            min_out,
            from_amount: U256::from(metadata.from_amount.u128()),
            from_asset: metadata.from_asset.as_bytes(),
            block_number: metadata.block_number,
            swap_hash: metadata.swap_hash,
            calldata,
        },
    };

    // Build the ibc message
    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: payload.try_encode()?.into(),     // Encode the parameters into a byte vector
        timeout: env.block.time.plus_seconds(TRANSACTION_TIMEOUT).into()
    };

    //TODO since this is permissionless, do we want to add a log here (e.g. sender and target pool)?
    Ok(Response::new()
        .add_message(ibc_msg)
    )
}

fn execute_send_cross_chain_liquidity(
    env: Env,
    info: MessageInfo,
    channel_id: String,
    to_pool: Vec<u8>,
    to_account: Vec<u8>,
    u: U256,
    min_out: U256,
    metadata: LiquiditySwapMetadata,    //TODO do we want this?
    calldata: Vec<u8>
) -> Result<Response, ContractError> {

    // Build payload
    let payload = CatalystV1SendLiquidityPayload {
        from_pool: info.sender.as_bytes(),
        to_pool: to_pool.as_slice(),
        to_account: to_account.as_slice(),
        u,
        variable_payload: SendLiquidityVariablePayload {
            min_out,
            from_amount: U256::from(metadata.from_amount.u128()),
            block_number: metadata.block_number,
            swap_hash: metadata.swap_hash,
            calldata,
        },
    };

    // Build the ibc message
    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: payload.try_encode()?.into(),     // Encode the parameters into a byte vector
        timeout: env.block.time.plus_seconds(TRANSACTION_TIMEOUT).into()
    };

    //TODO since this is permissionless, do we want to add a log here (e.g. sender and target pool)?
    Ok(Response::new()
        .add_message(ibc_msg)
    )
}


// The following 'query' code has been taken in part from the cw20-ics20 contract of the cw-plus repository.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Port {} => to_binary(&query_port(deps)?),
        QueryMsg::ListChannels {} => to_binary(&query_list(deps)?)
    }
}

fn query_port(deps: Deps) -> StdResult<PortResponse> {
    let query = IbcQuery::PortId {}.into();
    let PortIdResponse { port_id } = deps.querier.query(&query)?;
    Ok(PortResponse { port_id })
}

fn query_list(deps: Deps) -> StdResult<ListChannelsResponse> {
    //TODO use IbcQuery like with query_port?
    let channels = OPEN_CHANNELS
        .range_raw(deps.storage, None, None, Order::Ascending)
        .map(|r| r.map(|(_, v)| v))
        .collect::<StdResult<_>>()?;
    Ok(ListChannelsResponse { channels })
}


#[cfg(test)]
mod catalyst_ibc_interface_tests {

    use crate::{ibc_test_helpers::{open_channel, mock_channel_info, TEST_LOCAL_PORT, close_channel, TEST_REMOTE_PORT}, catalyst_ibc_payload::CatalystV1Packet, ibc::{ibc_packet_receive, RECEIVE_REPLY_ID, ibc_packet_ack, ibc_packet_timeout, reply}};

    use super::*;
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary, Uint128, SubMsg, IbcTimeout, IbcPacket, IbcEndpoint, Timestamp, IbcPacketReceiveMsg, IbcPacketAckMsg, IbcAcknowledgement, IbcPacketTimeoutMsg, Reply, SubMsgResponse, SubMsgResult};
    use ethnum::uint;

    pub const DEPLOYER_ADDR: &str = "deployer_addr";



    // Helpers ******************************************************************************************************************
    //TODO move helpers somewhere else?

    fn mock_ibc_packet(
        channel_id: &str,
        from_pool: &str,
        send_msg: ExecuteMsg,
        from_amount: Option<U256>    // Allow to override the send_msg from_amount to provide invalid configs
    ) -> IbcPacket {
        IbcPacket::new(
            Binary::from(build_payload(from_pool.as_bytes(), &send_msg, from_amount).unwrap()),
            IbcEndpoint {
                port_id: TEST_REMOTE_PORT.to_string(),
                channel_id: format!("{}-remote", channel_id),
            },
            IbcEndpoint {
                port_id: TEST_LOCAL_PORT.to_string(),
                channel_id: channel_id.to_string(),
            },
            7,
            mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT).into(),     // Note mock_env() always returns the same block time
        )
    }
    

    // Send asset helpers

    fn mock_send_asset_msg(
        channel_id: &str,
        to_pool: Vec<u8>,
        min_out: Option<U256>          // Allow to override the default value to provide invalid configs
    ) -> ExecuteMsg {
        ExecuteMsg::SendCrossChainAsset {
            channel_id: channel_id.into(),
            to_pool,
            to_account: b"to_account".to_vec(),
            to_asset_index: 1u8,
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            min_out: min_out.unwrap_or(
                uint!("323476719582585693194107115743132847255")                                                // Some large Uint128 number (as U256)
            ),
            metadata: AssetSwapMetadata {
                from_amount: Uint128::from(4920222095670429824873974121747892731u128),                          // Some large Uint128 number
                from_asset: "from_asset".to_string(),
                swap_hash: b"1aefweftegnedtwdwaagwwetgajyrgwd".to_vec(),
                block_number: 1356u32
            },
            calldata: vec![]
        }
    }

    fn mock_pool_receive_asset_msg(
        channel_id: &str,
        from_pool: Vec<u8>,
    ) -> swap_pool_common::msg::ExecuteMsg<()> {
        swap_pool_common::msg::ExecuteMsg::ReceiveAsset {
            channel_id: channel_id.into(),
            from_pool,
            to_asset_index: 1u8,
            to_account: "to_account".to_string(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            min_out: Uint128::from(323476719582585693194107115743132847255u128),                                // Some large Uint128 number
            swap_hash: b"1aefweftegnedtwdwaagwwetgajyrgwd".to_vec(),
            calldata: vec![]
        }
    }

    fn mock_pool_send_asset_success_msg(
    ) -> swap_pool_common::msg::ExecuteMsg<()> {
        swap_pool_common::msg::ExecuteMsg::SendAssetAck {
            to_account: "to_account".as_bytes().to_vec(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
            asset: "from_asset".to_string(),
            block_number_mod: 1356u32
        }
    }

    fn mock_pool_send_asset_failure_msg(
    ) -> swap_pool_common::msg::ExecuteMsg<()> {
        swap_pool_common::msg::ExecuteMsg::SendAssetTimeout {
            to_account: "to_account".as_bytes().to_vec(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
            asset: "from_asset".to_string(),
            block_number_mod: 1356u32
        }
    }


    // Send liquidity helpers
    
    fn mock_send_liquidity_msg(
        channel_id: &str,
        to_pool: Vec<u8>,
        min_out: Option<U256>          // Allow to override the default value to provide invalid configs
    ) -> ExecuteMsg {
        ExecuteMsg::SendCrossChainLiquidity {
            channel_id: channel_id.into(),
            to_pool,
            to_account: b"to_account".to_vec(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            min_out: min_out.unwrap_or(
                uint!("323476719582585693194107115743132847255")                                                // Some large Uint128 number (as U256)
            ),
            metadata: LiquiditySwapMetadata {
                from_amount: Uint128::from(4920222095670429824873974121747892731u128),                          // Some large Uint128 number
                swap_hash: b"1aefweftegnedtwdwaagwwetgajyrgwd".to_vec(),
                block_number: 1356u32
            },
            calldata: vec![]
        }
    }

    fn mock_pool_receive_liquidity_msg(
        channel_id: &str,
        from_pool: Vec<u8>,
    ) -> swap_pool_common::msg::ExecuteMsg<()> {
        swap_pool_common::msg::ExecuteMsg::ReceiveLiquidity {
            channel_id: channel_id.into(),
            from_pool,
            to_account: "to_account".to_string(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            min_out: Uint128::from(323476719582585693194107115743132847255u128),                                // Some large Uint128 number
            swap_hash: b"1aefweftegnedtwdwaagwwetgajyrgwd".to_vec(),
            calldata: vec![]
        }
    }

    fn mock_pool_send_liquidity_success_msg(
    ) -> swap_pool_common::msg::ExecuteMsg<()> {
        swap_pool_common::msg::ExecuteMsg::SendLiquidityAck {
            to_account: "to_account".as_bytes().to_vec(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
            block_number_mod: 1356u32
        }
    }

    fn mock_pool_send_liquidity_failure_msg(
    ) -> swap_pool_common::msg::ExecuteMsg<()> {
        swap_pool_common::msg::ExecuteMsg::SendLiquidityTimeout {
            to_account: "to_account".as_bytes().to_vec(),
            u: uint!("78456988731590487483448276103933454935747871349630657124267302091643025406701"),          // Some large U256 number
            amount: Uint128::from(4920222095670429824873974121747892731u128),                                   // Some large Uint128 number
            block_number_mod: 1356u32
        }
    }


    // TODO move into struct implementation?
    fn build_payload(
        from_pool: &[u8],
        msg: &ExecuteMsg,
        from_amount: Option<U256>    // Allow to override the msg 'from_amount' to provide invalid configs
    ) -> Result<Vec<u8>, ContractError> {
        let packet = match msg {
            ExecuteMsg::SendCrossChainAsset {
                channel_id: _,
                to_pool,
                to_account,
                to_asset_index,
                u,
                min_out,
                metadata,
                calldata
            } => CatalystV1Packet::SendAsset(
                CatalystV1SendAssetPayload {
                    from_pool,
                    to_pool: to_pool.as_slice(),
                    to_account: to_account.as_slice(),
                    u: *u,
                    variable_payload: SendAssetVariablePayload {
                        to_asset_index: *to_asset_index,
                        min_out: *min_out,
                        from_amount: from_amount.unwrap_or(U256::from(metadata.from_amount.u128())),
                        from_asset: metadata.from_asset.as_bytes(),
                        block_number: metadata.block_number,
                        swap_hash: metadata.swap_hash.clone(),
                        calldata: calldata.clone()
                    },
                }
            ),
            ExecuteMsg::SendCrossChainLiquidity {
                channel_id: _,
                to_pool,
                to_account,
                u,
                min_out,
                metadata,
                calldata
            } => CatalystV1Packet::SendLiquidity(
                CatalystV1SendLiquidityPayload {
                    from_pool,
                    to_pool: to_pool.as_slice(),
                    to_account: to_account.as_slice(),
                    u: *u,
                    variable_payload: SendLiquidityVariablePayload {
                        min_out: *min_out,
                        from_amount: from_amount.unwrap_or(U256::from(metadata.from_amount.u128())),
                        block_number: metadata.block_number,
                        swap_hash: metadata.swap_hash.clone(),
                        calldata: calldata.clone()
                    },
                }
            )
        };

        packet.try_encode()
    }



    // Channel Management Tests *************************************************************************************************

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



    // Send Asset Tests *********************************************************************************************************

    #[test]
    fn test_send_asset() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = b"to_pool";
        let execute_msg = mock_send_asset_msg(channel_id, to_pool.to_vec(), None);


        // Tested action: send asset
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(from_pool, &[]),
            execute_msg.clone()
        );


        // Check the transaction passes
        let response = response_result.unwrap();

        // Response should include a message to send the IBC message
        assert_eq!(response.messages.len(), 1);

        // Make sure the IBC message matches the expected one
        assert_eq!(
            &response.messages[0],
            &SubMsg::new(IbcMsg::SendPacket {
                channel_id: channel_id.to_string(),
                data: build_payload(from_pool.as_bytes(), &execute_msg, None).unwrap().into(),
                timeout: IbcTimeout::with_timestamp(mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT))
            })
        );

    }


    #[test]
    fn test_receive_asset() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_asset_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let receive_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);


        // Tested action: receive asset
        let response_result = ibc_packet_receive(
            deps.as_mut(),
            mock_env(),
            IbcPacketReceiveMsg::new(receive_packet.clone())
        );


        // Check the transaction passes
        let response = response_result.unwrap();

        // Check the returned ack
        assert_eq!(
            response.acknowledgement.clone(),
            Binary(vec![0u8])                   // ! Check ack returned has value of 0 (i.e. no error)
        );
    
        // Check pool is invoked
        assert_eq!(response.messages.len(), 1);

        assert_eq!(
            response.messages[0],
            SubMsg {
                id: RECEIVE_REPLY_ID,
                msg: cosmwasm_std::WasmMsg::Execute {
                    contract_addr: to_pool.to_string(),
                    msg: to_binary(&mock_pool_receive_asset_msg(channel_id, from_pool.as_bytes().to_vec())).unwrap(),
                    funds: vec![]
                }.into(),
                reply_on: cosmwasm_std::ReplyOn::Always,
                gas_limit: None

            }
        )

    }


    #[test]
    fn test_receive_asset_invalid_min_out() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = b"to_pool";
        let send_msg = mock_send_asset_msg(
            channel_id,
            to_pool.to_vec(),
            Some(U256::MAX)                                // ! Specify a min_out larger than Uint128
        );
        let receive_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);


        // Tested action: receive asset
        let response_result = ibc_packet_receive(
            deps.as_mut(),
            mock_env(),
            IbcPacketReceiveMsg::new(receive_packet.clone())
        );


        // Check the transaction passes
        let response = response_result.unwrap();

        // Check the returned ack
        assert_eq!(
            response.acknowledgement.clone(),
            Binary(vec![1u8])                   // ! Check ack returned has value of 1 (i.e. error)
        );
    
        // Check pool is not invoked
        assert_eq!(response.messages.len(), 0);

    }


    #[test]
    fn test_send_asset_ack() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_asset_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let ibc_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);



        // Tested action: send asset ack SUCCESSFUL
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for success
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: from_pool.to_string(),
                    msg: to_binary(&mock_pool_send_asset_success_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send asset ack UNSUCCESSFUL
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for failure
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: from_pool.to_string(),
                    msg: to_binary(&mock_pool_send_asset_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send asset ack INVALID
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![9u8])),         // ! Some invalid response
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();    // ! Make sure the transaction does not return error even for the invalid response

        // Check pool is not invoked
        assert_eq!(response.messages.len(), 0);

    }


    #[test]
    fn test_send_asset_timeout() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_asset_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let ibc_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);


        // Tested action: send asset timeout
        let response_result = ibc_packet_timeout(
            deps.as_mut(),
            mock_env(),
            IbcPacketTimeoutMsg::new(ibc_packet.clone())
        );


        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool timeout is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: from_pool.to_string(),
                    msg: to_binary(&mock_pool_send_asset_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        )

    }


    #[test]
    fn test_send_asset_ack_timeout_invalid_from_amount() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_asset_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let ibc_packet = mock_ibc_packet(channel_id, from_pool, send_msg, Some(U256::from(Uint128::MAX.u128()) + U256::from(1u64)));   // ! Inject an invalid from_amount into the ibc_packet



        // Tested action: send asset ACK SUCCESSFUL with invalid packet (from_amount)
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for ack-success
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is not invoked
        assert_eq!(response.messages.len(), 0);



        // Tested action: send asset ACK UNSUCCESSFUL with invalid packet
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for ack-failure
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is not invoked
        assert_eq!(response.messages.len(), 0);



        // Tested action: send asset TIMEOUT with invalid packet
        let response_result = ibc_packet_timeout(
            deps.as_mut(),
            mock_env(),
            IbcPacketTimeoutMsg::new(                               // ! Test for timeout
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is not invoked
        assert_eq!(response.messages.len(), 0);

    }



    // Send Liquidity Tests *****************************************************************************************************

    #[test]
    fn test_send_liquidity() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = b"to_pool";
        let execute_msg = mock_send_liquidity_msg(channel_id, to_pool.to_vec(), None);


        // Tested action: send liquidity
        let response_result = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(from_pool, &[]),
            execute_msg.clone()
        );


        // Check the transaction passes
        let response = response_result.unwrap();

        // Response should include a message to send the IBC message
        assert_eq!(response.messages.len(), 1);

        // Make sure the IBC message matches the expected one
        assert_eq!(
            &response.messages[0],
            &SubMsg::new(IbcMsg::SendPacket {
                channel_id: channel_id.to_string(),
                data: build_payload(from_pool.as_bytes(), &execute_msg, None).unwrap().into(),
                timeout: IbcTimeout::with_timestamp(mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT))
            })
        );

    }


    #[test]
    fn test_receive_liquidity() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_liquidity_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let receive_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);


        // Tested action: receive liquidity
        let response_result = ibc_packet_receive(
            deps.as_mut(),
            mock_env(),
            IbcPacketReceiveMsg::new(receive_packet.clone())
        );


        // Check the transaction passes
        let response = response_result.unwrap();

        // Check the returned ack
        assert_eq!(
            response.acknowledgement.clone(),
            Binary(vec![0u8])                   // ! Check ack returned has value of 0 (i.e. no error)
        );
    
        // Check pool is invoked
        assert_eq!(response.messages.len(), 1);

        assert_eq!(
            response.messages[0],
            SubMsg {
                id: RECEIVE_REPLY_ID,
                msg: cosmwasm_std::WasmMsg::Execute {
                    contract_addr: to_pool.to_string(),
                    msg: to_binary(&mock_pool_receive_liquidity_msg(channel_id, from_pool.as_bytes().to_vec())).unwrap(),
                    funds: vec![]
                }.into(),
                reply_on: cosmwasm_std::ReplyOn::Always,
                gas_limit: None

            }
        )

    }


    #[test]
    fn test_receive_liquidity_invalid_min_out() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = b"to_pool";
        let send_msg = mock_send_liquidity_msg(
            channel_id,
            to_pool.to_vec(),
            Some(U256::MAX)                                // ! Specify a min_out larger than Uint128
        );
        let receive_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);


        // Tested action: receive liquidity
        let response_result = ibc_packet_receive(
            deps.as_mut(),
            mock_env(),
            IbcPacketReceiveMsg::new(receive_packet.clone())
        );


        // Check the transaction passes
        let response = response_result.unwrap();

        // Check the returned ack
        assert_eq!(
            response.acknowledgement.clone(),
            Binary(vec![1u8])                   // ! Check ack returned has value of 1 (i.e. error)
        );
    
        // Check pool is not invoked
        assert_eq!(response.messages.len(), 0);

    }


    #[test]
    fn test_send_liquidity_ack() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_liquidity_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let ibc_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);



        // Tested action: send liquidity ack SUCCESSFUL
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for success
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: from_pool.to_string(),
                    msg: to_binary(&mock_pool_send_liquidity_success_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send liquidity ack UNSUCCESSFUL
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for failure
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: from_pool.to_string(),
                    msg: to_binary(&mock_pool_send_liquidity_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        );



        // Tested action: send liquidity ack INVALID
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![9u8])),         // ! Some invalid response
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();    // ! Make sure the transaction does not return error even for the invalid response

        // Check pool is not invoked
        assert_eq!(response.messages.len(), 0);

    }


    #[test]
    fn test_send_liquidity_timeout() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_liquidity_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let ibc_packet = mock_ibc_packet(channel_id, from_pool, send_msg, None);


        // Tested action: send liquidity timeout
        let response_result = ibc_packet_timeout(
            deps.as_mut(),
            mock_env(),
            IbcPacketTimeoutMsg::new(ibc_packet.clone())
        );


        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool timeout is invoked
        assert_eq!(response.messages.len(), 1);
        assert_eq!(
            response.messages[0],
            SubMsg::new(
                cosmwasm_std::WasmMsg::Execute {
                    contract_addr: from_pool.to_string(),
                    msg: to_binary(&mock_pool_send_liquidity_failure_msg()).unwrap(),
                    funds: vec![]
                }
            )
        )

    }


    #[test]
    fn test_send_liquidity_ack_timeout_invalid_from_amount() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();

        let channel_id = "mock-channel-1";
        open_channel(deps.as_mut(), channel_id, None, None);

        // Get mock params
        let from_pool = "sender";
        let to_pool = "to_pool";
        let send_msg = mock_send_liquidity_msg(channel_id, to_pool.as_bytes().to_vec(), None);
        let ibc_packet = mock_ibc_packet(channel_id, from_pool, send_msg, Some(U256::from(Uint128::MAX.u128()) + U256::from(1u64)));   // ! Inject an invalid from_amount into the ibc_packet



        // Tested action: send liquidity ACK SUCCESSFUL with invalid packet (from_amount)
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![0u8])),         // ! Test for ack-success
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is not invoked
        assert_eq!(response.messages.len(), 0);



        // Tested action: send liquidity ACK UNSUCCESSFUL with invalid packet
        let response_result = ibc_packet_ack(
            deps.as_mut(),
            mock_env(),
            IbcPacketAckMsg::new(
                IbcAcknowledgement::new(Binary(vec![1u8])),         // ! Test for ack-failure
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is not invoked
        assert_eq!(response.messages.len(), 0);



        // Tested action: send liquidity TIMEOUT with invalid packet
        let response_result = ibc_packet_timeout(
            deps.as_mut(),
            mock_env(),
            IbcPacketTimeoutMsg::new(                               // ! Test for timeout
                ibc_packet.clone()
            )
        );

        // Check the transaction passes
        let response = response_result.unwrap();
    
        // Check pool ack is not invoked
        assert_eq!(response.messages.len(), 0);

    }



    // Common Tests *************************************************************************************************************
    

    #[test]
    fn test_receive_reply() {

        let mut deps = mock_dependencies();
      
        // Instantiate contract and open channel
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(DEPLOYER_ADDR, &vec![]),
            InstantiateMsg {}
        ).unwrap();


        // Tested action: reply ok
        let response_result = reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: RECEIVE_REPLY_ID,
                result: SubMsgResult::Ok(
                    SubMsgResponse { events: vec![], data: None }       // SubMsgResponse contents do not matter
                )
            }
        );

        // Check the transaction passes
        let response = response_result.unwrap();

        // Check response
        // TODO overhaul this is the desired result
        assert_eq!(response.messages.len(), 0);
        assert_eq!(
            response.data,
            None                        //  ! If the submessage call by 'ibc_packet_receive' does not return error, leave the response 'data' field untouched
        );



        // Tested action: reply error
        let response_result = reply(
            deps.as_mut(),
            mock_env(),
            Reply {
                id: RECEIVE_REPLY_ID,
                result: SubMsgResult::Err("".to_string())
            }
        );

        // Check the transaction passes
        let response = response_result.unwrap();

        // Check response contains an ack-fail
        assert_eq!(response.messages.len(), 0);
        assert_eq!(
            response.data,
            Some(Binary((vec![1])))     // ! If the submessage call by 'ibc_packet_receive' returns error, overwrite the response 'data' field to a failed ack
        );

    }

}
