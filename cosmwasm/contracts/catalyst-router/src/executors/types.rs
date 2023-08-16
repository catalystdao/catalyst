pub mod types {
    use cosmwasm_std::{Coin, DepsMut, Env};

    use crate::error::ContractError;

    pub type Denom = String;

    pub enum Balance {
        RouterBalance(Denom),
        Coin(Coin)
    }

    impl Balance {
        
        pub fn get_balance(
            &self,
            deps: &mut DepsMut,
            env: &Env
        ) -> Result<Coin, ContractError> {
            match self {
                Balance::RouterBalance(denom) => {
                    deps.querier
                        .query_balance(env.contract.address.clone(), denom)
                        .map_err(|err| err.into())
                },
                Balance::Coin(coin) => Ok(coin.clone()),
            }
        }
    }

    pub enum Account {
        Sender,
        Router,
        Address(String)
    }
}
