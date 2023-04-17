use cosmwasm_std::{Response, DepsMut, testing::{mock_env, mock_info}, Uint128};
use swap_pool_common::msg::InstantiateMsg;

use crate::{contract::{instantiate, execute}, msg::VolatileExecuteMsg};

pub const DEPLOYER_ADDR: &str = "deployer_addr";
pub const FACTORY_OWNER_ADDR: &str = "factory_owner_addr";
pub const SETUP_MASTER_ADDR: &str = "setup_master_addr";
pub const CHAIN_INTERFACE_ADDR: &str = "chain_interface";
pub const DEPOSITOR_ADDR: &str = "depositor_addr";
pub const FEE_ADMINISTRATOR: &str = "fee_administrator_addr";

#[derive(Clone)]
pub struct InitializeSwapCurvesMockMsg {
    pub assets: Vec<String>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<u64>,
    pub amp: u64,
    pub depositor: String
}

impl Into<VolatileExecuteMsg> for InitializeSwapCurvesMockMsg {
    fn into(self) -> VolatileExecuteMsg {
        VolatileExecuteMsg::InitializeSwapCurves {
            assets: self.assets,
            assets_balances: self.assets_balances,
            weights: self.weights,
            amp: self.amp,
            depositor: self.depositor
        }
    }
}


pub fn mock_instantiate_msg(
    chain_interface: Option<String>
) -> InstantiateMsg {
    InstantiateMsg {
        name: "TestPool".to_string(),
        symbol: "TP".to_string(),
        chain_interface,
        pool_fee: 10000u64,
        governance_fee: 50000u64,
        fee_administrator: FEE_ADMINISTRATOR.to_string(),
        setup_master: SETUP_MASTER_ADDR.to_string()
    }
}

pub fn mock_instantiate(deps: DepsMut) -> Response {
    instantiate(
        deps,
        mock_env(),
        mock_info(DEPLOYER_ADDR, &vec![]),
        mock_instantiate_msg(Some(CHAIN_INTERFACE_ADDR.to_string()))
    ).unwrap()
}

pub fn finish_pool_setup(deps: DepsMut) -> Response {
    execute(
        deps,
        mock_env(),
        mock_info(SETUP_MASTER_ADDR, &vec![]),
        VolatileExecuteMsg::FinishSetup {}
    ).unwrap()
}