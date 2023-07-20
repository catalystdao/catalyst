#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use catalyst_types::U256;
use catalyst_ibc_interface::{msg::{ExecuteMsg, InstantiateMsg, QueryMsg}, ContractError};


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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
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
    block_number: u32,
    calldata: Binary
) -> Result<Response, ContractError> {

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
            .add_attribute("block_number", block_number.to_string())
            .add_attribute("calldata", calldata.to_base64())
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

    Ok(
        Response::new()
            .add_attribute("action", "mock-interface-send-asset")
            .add_attribute("channel_id", channel_id)
            .add_attribute("to_vault", to_vault.to_base64())
            .add_attribute("to_account", to_account.to_base64())
            .add_attribute("u", u)
            .add_attribute("min_vault_tokens", min_vault_tokens)
            .add_attribute("min_reference_asset", min_reference_asset)
            .add_attribute("from_amount", from_amount)
            .add_attribute("block_number", block_number.to_string())
            .add_attribute("calldata", calldata.to_base64())
    )

}


// The following 'query' code has been taken in part from the cw20-ics20 contract of the cw-plus repository.
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {

    panic!("'query' is not implemented for the mock interface.")

}
