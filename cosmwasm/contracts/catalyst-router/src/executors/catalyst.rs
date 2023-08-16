pub mod catalyst_executors {
    use cosmwasm_std::{DepsMut, Env, Binary, CosmosMsg, to_binary, from_binary};

    use crate::{commands::CommandResult, error::ContractError};


    pub fn execute_local_swap(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_send_asset(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_send_liquidity(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_withdraw_equal(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_withdraw_mixed(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_deposit_mixed(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }
}
