#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, Uint64};
use catalyst_types::U256;
use catalyst_ibc_interface::{msg::{ExecuteMsg, InstantiateMsg, QueryMsg}, ContractError, state::update_owner};


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    Ok(Response::new())

}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
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
            underwrite_incentive_x16,
            block_number,
            calldata
        } => execute_send_cross_chain_asset(
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
        } => execute_set_max_underwrite_duration(
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
        } => execute_underwrite(
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
        } => execute_underwrite_and_check_connection(
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
        } => execute_expire_underwrite(
            to_vault,
            to_asset_ref,
            u,
            min_out,
            to_account,
            underwrite_incentive_x16,
            calldata
        ),

        ExecuteMsg::WrapSubMsgs { sub_msgs } => todo!(),
    


        // Ownership msgs
        ExecuteMsg::TransferOwnership {
            new_owner
        } => todo!(),
    }

}

fn execute_send_cross_chain_asset(
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
) -> Result<Response, ContractError> {

    let calldata = match calldata.len() {
        0 => String::from("None"),  // NOTE: It is not possible to add empty attributes
        _ => calldata.to_base64()
    };

    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-send-asset")
            .add_attribute("channel_id", channel_id)
            .add_attribute("to_vault", to_vault.to_base64())
            .add_attribute("to_account", to_account.to_base64())
            .add_attribute("to_asset_index", to_asset_index.to_string())
            .add_attribute("u", u)
            .add_attribute("min_out", min_out)
            .add_attribute("from_amount", from_amount)
            .add_attribute("from_asset", from_asset)
            .add_attribute("underwrite_incentive_x16", underwrite_incentive_x16.to_string())
            .add_attribute("block_number", block_number.to_string())
            .add_attribute("calldata", calldata)
    )

}

fn execute_send_cross_chain_liquidity(
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

    let calldata = match calldata.len() {
        0 => String::from("None"),  // NOTE: It is not possible to add empty attributes
        _ => calldata.to_base64()
    };

    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-send-liquidity")
            .add_attribute("channel_id", channel_id)
            .add_attribute("to_vault", to_vault.to_base64())
            .add_attribute("to_account", to_account.to_base64())
            .add_attribute("u", u)
            .add_attribute("min_vault_tokens", min_vault_tokens)
            .add_attribute("min_reference_asset", min_reference_asset)
            .add_attribute("from_amount", from_amount)
            .add_attribute("block_number", block_number.to_string())
            .add_attribute("calldata", calldata)
    )

}


fn execute_set_max_underwrite_duration(
    new_max_underwrite_duration: Uint64
) -> Result<Response, ContractError> {
    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-set-max-underwrite-duration")
            .add_attribute("new_max_underwrite_duration", new_max_underwrite_duration.to_string())
    )
}


fn execute_underwrite(
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Result<Response, ContractError> {

    let calldata = match calldata.len() {
        0 => String::from("None"),  // NOTE: It is not possible to add empty attributes
        _ => calldata.to_base64()
    };

    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-underwrite")
            .add_attribute("to_vault", to_vault)
            .add_attribute("to_asset_ref", to_asset_ref)
            .add_attribute("u", u)
            .add_attribute("min_out", min_out)
            .add_attribute("to_account", to_account)
            .add_attribute("underwrite_incentive_x16", underwrite_incentive_x16.to_string())
            .add_attribute("calldata", calldata)
    )
}


fn execute_underwrite_and_check_connection(
    channel_id: String,
    from_vault: Binary,
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Result<Response, ContractError> {

    let calldata = match calldata.len() {
        0 => String::from("None"),  // NOTE: It is not possible to add empty attributes
        _ => calldata.to_base64()
    };

    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-underwrite-and-check-connection")
            .add_attribute("channel_id", channel_id)
            .add_attribute("from_vault", from_vault.to_base64())
            .add_attribute("to_vault", to_vault)
            .add_attribute("to_asset_ref", to_asset_ref)
            .add_attribute("u", u)
            .add_attribute("min_out", min_out)
            .add_attribute("to_account", to_account)
            .add_attribute("underwrite_incentive_x16", underwrite_incentive_x16.to_string())
            .add_attribute("calldata", calldata)
    )
}


fn execute_expire_underwrite(
    to_vault: String,
    to_asset_ref: String,
    u: U256,
    min_out: Uint128,
    to_account: String,
    underwrite_incentive_x16: u16,
    calldata: Binary
) -> Result<Response, ContractError> {

    let calldata = match calldata.len() {
        0 => String::from("None"),  // NOTE: It is not possible to add empty attributes
        _ => calldata.to_base64()
    };

    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-expire-underwrite")
            .add_attribute("to_vault", to_vault)
            .add_attribute("to_asset_ref", to_asset_ref)
            .add_attribute("u", u)
            .add_attribute("min_out", min_out)
            .add_attribute("to_account", to_account)
            .add_attribute("underwrite_incentive_x16", underwrite_incentive_x16.to_string())
            .add_attribute("calldata", calldata)
    )
}


// The following 'query' code has been taken in part from the cw20-ics20 contract of the cw-plus repository.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {

    panic!("'query' is not implemented for the mock interface.")

}
