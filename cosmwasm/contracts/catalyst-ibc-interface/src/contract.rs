#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128, Addr};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{CatalystIBCInterfaceState, CATALYST_IBC_INTERFACE_STATE};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:catalyst-ibc-interface";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


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
        default_timeout: msg.default_timeout
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
        ExecuteMsg::CrossChainSwap {
            chain_id,
            target_pool,
            target_user,
            target_asset_index,
            units_x64,
            min_out,
            approx,
            source_amount,
            source_asset,
            calldata
        } => execute_cross_chain_swap(
            deps,
            env,
            info,
            chain_id,
            target_pool,
            target_user,
            target_asset_index,
            units_x64,
            min_out,
            approx,
            source_amount,
            source_asset,
            calldata
        )
    }
}

fn execute_register_pool(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    pool: String
) -> Result<Response, ContractError> {
    unimplemented!();
}

fn execute_cross_chain_swap(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    chain_id: String,
    target_pool: [u8; 32],
    target_user: [u8; 32],
    target_asset_index: u8,
    units_x64: [u8; 32],
    min_out: [u8; 32],
    approx: bool,
    source_amount: Uint128,
    source_asset: Addr,
    calldata: Vec<u8>
) -> Result<Response, ContractError> {
    unimplemented!();
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
