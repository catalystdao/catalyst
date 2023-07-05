#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw2::set_contract_version;
use catalyst_types::U256;

use catalyst_ibc_interface::ContractError;
use catalyst_ibc_interface::catalyst_ibc_payload::{CatalystV1SendAssetPayload, SendAssetVariablePayload, CatalystV1SendLiquidityPayload, SendLiquidityVariablePayload, CatalystEncodedAddress};

use crate::mock_ibc::{execute_ibc_packet_receive, execute_ibc_packet_ack, execute_ibc_packet_timeout};
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{set_owner_unchecked, set_owner};

// version info for migration info
const CONTRACT_NAME: &str = "catalyst-ibc-interface-poa";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const TRANSACTION_TIMEOUT: u64 = 2 * 60 * 60;   // 2 hours      //TODO allow this to be set on interface instantiation?
                                                                //TODO allow this to be customized on a per-channel basis?
                                                                //TODO allow this to be overriden on 'sendAsset' and 'sendLiquidity'?


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    set_owner_unchecked(&mut deps, info.sender)?;

    Ok(
        Response::new()
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
            to_vault,
            to_account,
            to_asset_index,
            u,
            min_out,
            from_amount,
            from_asset,
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

        ExecuteMsg::IBCPacketReceive {
            data,
            channel_id
        } => execute_ibc_packet_receive(
            deps,
            info,
            data,
            channel_id
        ),
    
        ExecuteMsg::IBCPacketAck {
            data,
            response,
            channel_id
        } => execute_ibc_packet_ack(
            deps,
            info,
            data,
            response,
            channel_id
        ),
    
        ExecuteMsg::IBCPacketTimeout {
            data,
            channel_id
        } => execute_ibc_packet_timeout(
            deps,
            info,
            data,
            channel_id
        ),

        // Ownership msgs
        ExecuteMsg::TransferOwnership {
            new_owner
        } => execute_transfer_ownership(
            deps,
            info,
            new_owner
        )

    }
}

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
    block_number: u32,
    calldata: Binary
) -> Result<Response, ContractError> {

    // Build payload
    let payload = CatalystV1SendAssetPayload {
        from_vault: CatalystEncodedAddress::try_encode(info.sender.as_bytes())?,
        to_vault: CatalystEncodedAddress::try_from(to_vault)?,                        // to_vault should already be encoded
        to_account: CatalystEncodedAddress::try_from(to_account)?,                  // to_account should already be encoded
        u,
        variable_payload: SendAssetVariablePayload {
            to_asset_index,
            min_out,
            from_amount: from_amount.into(),
            from_asset: CatalystEncodedAddress::try_encode(from_asset.as_bytes())?,
            block_number,
            calldata,
        },
    };

    Ok(Response::new()
        .add_attribute("action", "ibc_send")
        .add_attribute("channel_id", channel_id)
        .add_attribute("data", payload.try_encode()?.to_base64())
        .add_attribute("timeout", env.block.time.plus_seconds(TRANSACTION_TIMEOUT).seconds().to_string())
    )
}

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
) -> Result<Response, ContractError> {

    // Build payload
    let payload = CatalystV1SendLiquidityPayload {
        from_vault: CatalystEncodedAddress::try_encode(info.sender.as_bytes())?,
        to_vault: CatalystEncodedAddress::try_from(to_vault)?,                        // to_vault should already be encoded
        to_account: CatalystEncodedAddress::try_from(to_account)?,                  // to_account should already be encoded
        u,
        variable_payload: SendLiquidityVariablePayload {
            min_vault_tokens,
            min_reference_asset,
            from_amount: U256::from(from_amount.u128()),
            block_number,
            calldata,
        },
    };

    Ok(Response::new()
        .add_attribute("action", "ibc_send")
        .add_attribute("channel_id", channel_id)
        .add_attribute("data", payload.try_encode()?.to_base64())
        .add_attribute("timeout", env.block.time.plus_seconds(TRANSACTION_TIMEOUT).seconds().to_string())
    )
}


fn execute_transfer_ownership(
    mut deps: DepsMut,
    info: MessageInfo,
    new_owner: String
) -> Result<Response, ContractError> {
    set_owner(&mut deps, info, new_owner)
}

// The following 'query' code has been taken in part from the cw20-ics20 contract of the cw-plus repository.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}
