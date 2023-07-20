#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use crate::{msg::{InstantiateMsg, ExecuteMsg, QueryMsg}, error::ContractError};



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg
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

        ExecuteMsg::OnCatalystCall {
            purchased_tokens,
            data
        } => execute_on_catalyst_call(purchased_tokens, data)

    }
}

fn execute_on_catalyst_call(
    purchased_tokens: Uint128,
    data: Binary
) -> Result<Response, ContractError> {

    Ok(
        Response::new()
            .add_attribute("action", "on-catalyst-call")
            .add_attribute("purchased_tokens", purchased_tokens)
            .add_attribute("data", data.to_base64())
    )

}



#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
    }
}
