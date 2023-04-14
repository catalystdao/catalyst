use cosmwasm_std::{Response, DepsMut, testing::{mock_env, mock_info}};
use swap_pool_common::msg::InstantiateMsg;

use crate::contract::instantiate;

pub const DEPLOYER_ADDR: &str = "deployer_addr";
pub const DEPOSITOR_ADDR: &str = "depositor_addr";

pub fn mock_instantiate_msg(
    chain_interface: Option<String>
) -> InstantiateMsg {
    InstantiateMsg {
        name: "TestPool".to_string(),
        symbol: "TP".to_string(),
        chain_interface,
        pool_fee: 10000u64,
        governance_fee: 50000u64,
        fee_administrator: "fee_administrator".to_string(),
        setup_master: "setup_master".to_string()
    }
}

pub fn mock_instantiate(deps: DepsMut) -> Response {
    instantiate(
        deps,
        mock_env(),
        mock_info(DEPLOYER_ADDR, &vec![]),
        mock_instantiate_msg(Some("chain_interface".to_string()))
    ).unwrap()
}