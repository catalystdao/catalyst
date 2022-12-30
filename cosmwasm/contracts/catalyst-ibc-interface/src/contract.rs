#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, IbcChannelOpenMsg, IbcChannelConnectMsg, IbcBasicResponse, IbcChannelCloseMsg, IbcPacketReceiveMsg, IbcReceiveResponse, IbcPacketAckMsg, IbcPacketTimeoutMsg};
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
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}


// IBC entrypoints

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_open(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcChannelOpenMsg
) -> StdResult<()> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_connect(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcChannelConnectMsg,
) -> StdResult<IbcBasicResponse> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_channel_close(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcChannelCloseMsg,
) -> StdResult<IbcBasicResponse> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_receive(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketReceiveMsg,
) -> StdResult<IbcReceiveResponse> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_ack(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketAckMsg,
) -> StdResult<IbcBasicResponse> {
    unimplemented!();
}


#[cfg_attr(not(feature = "library"), entry_point)]
pub fn ibc_packet_timeout(
    _deps: DepsMut,
    _env: Env,
    _msg: IbcPacketTimeoutMsg,
) -> StdResult<IbcBasicResponse> {
    unimplemented!();
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
