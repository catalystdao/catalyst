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
            swap_hash: metadata.swap_hash.as_bytes(),
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
            swap_hash: metadata.swap_hash.as_bytes(),
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

    use crate::{ibc_test_helpers::{open_channel, mock_channel_info, TEST_LOCAL_PORT, close_channel}, catalyst_ibc_payload::CatalystV1Packet};

    use super::*;
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary, Uint128, SubMsg, IbcTimeout};
    use ethnum::uint;

    pub const DEPLOYER_ADDR: &str = "deployer_addr";


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

    fn mock_send_asset_msg(
        channel_id: &str
    ) -> ExecuteMsg {
        ExecuteMsg::SendCrossChainAsset {
            channel_id: channel_id.into(),
            to_pool: b"to_pool".to_vec(),
            to_account: b"to_account".to_vec(),
            to_asset_index: 1u8,
            u: uint!("78556986031590987483442276103933364935747871949630657171867302091643025206701"),          // Some large U256 number
            min_out: uint!("77771416171275077608607853342894031286390393230134350600148629070726594954633"),    // Some large U256 number
            metadata: AssetSwapMetadata {
                from_amount: Uint128::from(4920222095670429824873974121747892731u128),                          // Some large Uint128 number
                from_asset: "from_asset".to_string(),
                swap_hash: "1aefweftegnedtwdwaagwwetgajyrgwd".to_string(),
                block_number: 1356u32
            },
            calldata: vec![]
        }
    }

    
    // TODO move into struct implementation?
    fn build_payload(
        from_pool: &[u8],
        msg: &ExecuteMsg
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
                        from_amount: U256::from(metadata.from_amount.u128()),
                        from_asset: metadata.from_asset.as_bytes(),
                        block_number: metadata.block_number,
                        swap_hash: metadata.swap_hash.as_bytes(),
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
                        from_amount: U256::from(metadata.from_amount.u128()),
                        block_number: metadata.block_number,
                        swap_hash: metadata.swap_hash.as_bytes(),
                        calldata: calldata.clone()
                    },
                }
            )
        };

        packet.try_encode()
    }


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
        let execute_msg = mock_send_asset_msg(channel_id);

        // Test action: send asset
        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info(from_pool, &[]),
            execute_msg.clone()
        ).unwrap();

        // Response should include a message to send the IBC message
        assert_eq!(response.messages.len(), 1);

        // Make sure the IBC message matches the expected one
        assert_eq!(
            &response.messages[0],
            &SubMsg::new(IbcMsg::SendPacket {
                channel_id: channel_id.to_string(),
                data: build_payload(from_pool.as_bytes(), &execute_msg).unwrap().into(),
                timeout: IbcTimeout::with_timestamp(mock_env().block.time.plus_seconds(TRANSACTION_TIMEOUT))
            })
        );

    }


}
