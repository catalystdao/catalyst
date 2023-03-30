#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcMsg};
use cw2::set_contract_version;
use ethnum::U256;

use crate::catalyst_ibc_payload::{CTX0_ASSET_SWAP, CTX0_DATA_START, CTX1_DATA_START, CTX1_LIQUIDITY_SWAP};
use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, AssetSwapMetadata, LiquiditySwapMetadata};
use crate::state::{CatalystIBCInterfaceState, CATALYST_IBC_INTERFACE_STATE};

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
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let catalyst_ibc_interface_state = CatalystIBCInterfaceState {
        admin: deps.api.addr_validate(&msg.gov_contract)?,   // Validate ibc_endpoint
        default_timeout: msg.default_timeout                 //TODO remove
    };

    CATALYST_IBC_INTERFACE_STATE.save(deps.storage, &catalyst_ibc_interface_state)?;

    Ok(
        Response::new()
            .add_attribute("gov_contract", msg.gov_contract)
            .add_attribute("default_timeout", msg.default_timeout.to_string())
    )
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
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



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}


#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::{testing::{mock_dependencies, mock_env, mock_info}, Attribute};

    pub const SOME_ADDR: &str = "some_addr";
    pub const GOV_ADDR: &str = "gov_addr";

    #[test]
    fn test_instantiate() {

        let mut deps = mock_dependencies();
        let env = mock_env();
        let info = mock_info(SOME_ADDR, &vec![]);
      
        let msg = InstantiateMsg {
            gov_contract: GOV_ADDR.to_string(),
            default_timeout: 3600       // 1 hour
        };
      
        // Instantiate contract
        let response = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // Verify response attributes
        assert_eq!(
            response.attributes[0],
            Attribute { key: "gov_contract".to_string(), value: GOV_ADDR.to_string() }
        );

        assert_eq!(
            response.attributes[1],
            Attribute { key: "default_timeout".to_string(), value: 3600.to_string() }
        );

        // TODO Verify state
    }
}
