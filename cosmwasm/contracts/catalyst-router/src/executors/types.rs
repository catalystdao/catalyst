pub mod types {
    use cosmwasm_schema::cw_serde;
    use cosmwasm_std::{Coin, Env, Deps};

    use crate::{error::ContractError, state::get_router_locker};

    pub type Denom = String;

    #[cw_serde]
    pub enum Amount {
        Coin(Coin),
        RouterBalance(Denom),
    }

    impl Amount {
        
        pub fn get_amount(
            &self,
            deps: &Deps,
            env: &Env
        ) -> Result<Coin, ContractError> {
            match self {
                Amount::Coin(coin) => Ok(coin.clone()),
                Amount::RouterBalance(denom) => {
                    deps.querier
                        .query_balance(env.contract.address.clone(), denom)
                        .map_err(|err| err.into())
                },
            }
        }
    }

    #[cw_serde]
    pub enum Account {
        Sender,
        Router,
        Address(String)
    }

    impl Account {

        pub fn get_address(
            &self,
            deps: &Deps,
            env: &Env
        ) -> Result<String, ContractError> {
            match  &self {
                Account::Sender => {
                    let router_locker = get_router_locker(deps)?;
                    Ok(router_locker.to_string())
                },
                Account::Router => Ok(env.contract.address.to_string()),
                Account::Address(address) => Ok(address.to_owned()),
            }
        }
    }
}
