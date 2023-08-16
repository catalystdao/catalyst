pub mod payments_executors {
    use cosmwasm_std::{DepsMut, Env, Binary};

    use crate::{commands::CommandResult, error::ContractError};


    pub fn execute_sweep(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_transfer(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_pay_portion(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }


    pub fn execute_balance_check(
        deps: &mut DepsMut,
        env: &Env,
        input: &Binary
    ) -> Result<CommandResult, ContractError> {
        todo!()
    }
}
