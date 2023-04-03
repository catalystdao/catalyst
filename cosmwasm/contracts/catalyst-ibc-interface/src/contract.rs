#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg, to_binary, IbcQuery, PortIdResponse, Order};
use cw2::set_contract_version;
use ethnum::U256;

use crate::catalyst_ibc_payload::{CTX0_ASSET_SWAP, CTX0_DATA_START, CTX1_DATA_START, CTX1_LIQUIDITY_SWAP};
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
    to_pool: String,
    to_account: String,
    to_asset_index: u8,
    u: U256,
    min_out: U256,
    metadata: AssetSwapMetadata,    //TODO do we want this?
    calldata: Vec<u8>
) -> Result<Response, ContractError> {

    // Encode the parameters into a byte vector

    let from_pool  = info.sender.as_bytes();
    let to_pool    = to_pool.as_bytes();
    let to_account = to_account.as_bytes();
    let from_asset = metadata.from_asset.as_bytes();

    // Preallocate the required size for the IBC payload to avoid runtime reallocations.
    let mut data: Vec<u8> = Vec::with_capacity(
        CTX0_DATA_START     // This defines the size of all the fixed-length elements of the payload
        + from_pool.len()
        + to_pool.len()
        + to_account.len()
        + from_asset.len()
        + calldata.len()
    );   // Addition is way below the overflow threshold, and even if it were to overflow the code would still function properly, as this is just a runtime optimization.

    // Context
    data.push(CTX0_ASSET_SWAP);

    // From pool
    data.push(
        from_pool.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&from_pool);

    // To pool
    data.push(
        to_pool.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&to_pool);

    // To account
    data.push(
        to_account.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&to_account);

    // Units
    data.extend_from_slice(&u.to_be_bytes());

    // To asset index
    data.push(to_asset_index);

    // Min out
    data.extend_from_slice(&min_out.to_be_bytes());

    // From amount
    data.extend_from_slice(&U256::from(metadata.from_amount.u128()).to_be_bytes());

    // From asset
    data.push(
        from_asset.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&from_asset);

    // Block number
    data.extend_from_slice(&metadata.block_number.to_be_bytes());

    // Swap hash
    let swap_hash = metadata.swap_hash.as_bytes();
    if swap_hash.len() != 32 {
        return Err(ContractError::PayloadDecodingError {});
    }
    data.extend_from_slice(&swap_hash);

    // Calldata
    let calldata_length: u16 = calldata.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow
    data.extend_from_slice(&calldata_length.to_be_bytes());
    data.extend_from_slice(&calldata);


    // Build the ibc message
    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: data.into(),
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
    to_pool: String,
    to_account: String,
    u: U256,
    min_out: U256,
    metadata: LiquiditySwapMetadata,    //TODO do we want this?
    calldata: Vec<u8>
) -> Result<Response, ContractError> {

    // Encode the parameters into a byte vector

    let from_pool  = info.sender.as_bytes();
    let to_pool    = to_pool.as_bytes();
    let to_account = to_account.as_bytes();

    // Preallocate the required size for the IBC payload to avoid runtime reallocations.
    let mut data: Vec<u8> = Vec::with_capacity(
        CTX1_DATA_START     // This defines the size of all the fixed-length elements of the payload
        + from_pool.len()
        + to_pool.len()
        + to_account.len()
        + calldata.len()
    );   // Addition is way below the overflow threshold, and even if it were to overflow the code would still function properly, as this is just a runtime optimization.

    // Context
    data.push(CTX1_LIQUIDITY_SWAP);

    // From pool
    data.push(
        from_pool.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&from_pool);

    // To pool
    data.push(
        to_pool.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&to_pool);

    // To account
    data.push(
        to_account.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?    // Cast length into u8 catching overflow
    );
    data.extend_from_slice(&to_account);

    // Units
    data.extend_from_slice(&u.to_be_bytes());

    // Min out
    data.extend_from_slice(&min_out.to_be_bytes());

    // From amount
    data.extend_from_slice(&U256::from(metadata.from_amount.u128()).to_be_bytes());

    // Block number
    data.extend_from_slice(&metadata.block_number.to_be_bytes());

    // Swap hash
    let swap_hash = metadata.swap_hash.as_bytes();
    if swap_hash.len() != 32 {
        return Err(ContractError::PayloadDecodingError {});
    }
    data.extend_from_slice(&swap_hash);

    // Calldata
    let calldata_length: u16 = calldata.len().try_into().map_err(|_| ContractError::PayloadEncodingError {})?;    // Cast length into u16 catching overflow
    data.extend_from_slice(&calldata_length.to_be_bytes());
    data.extend_from_slice(&calldata);


    // Build the ibc message
    let ibc_msg = IbcMsg::SendPacket {
        channel_id,
        data: data.into(),
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

    use crate::ibc_test_helpers::{open_channel, mock_channel_info, TEST_LOCAL_PORT, close_channel};

    use super::*;
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, from_binary};

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

}
