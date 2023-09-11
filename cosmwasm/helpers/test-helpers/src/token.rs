use cosmwasm_schema::{schemars::JsonSchema, serde::{Serialize, de::DeserializeOwned}};
use cosmwasm_std::{Uint128, Addr, Empty};
use cw20::{Cw20Coin, MinterResponse, BalanceResponse, Cw20QueryMsg, TokenInfoResponse, Cw20ExecuteMsg, AllowanceResponse};
use cw_multi_test::{ContractWrapper, AppResponse, Executor, Module};
use std::fmt::Debug;

use crate::env::CustomApp;

pub const WAD: Uint128 = Uint128::new(1000000000000000000u128);

#[derive(Clone)]
pub struct TestTokenDefinition {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_mint: Uint128,
    pub minter: String
}

impl TestTokenDefinition {

    pub fn deploy_token<AppC, ExecC>(
        &self,
        app: &mut AppC,
        cw20_contract: u64,
        minter: Addr
    ) -> Addr
    where
        AppC: Executor<ExecC>,
        ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
    {
        app.instantiate_contract::<cw20_base::msg::InstantiateMsg, _>(
            cw20_contract,
            minter.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: self.name.clone(),
                symbol: self.symbol.clone(),
                decimals: self.decimals,
                initial_balances: vec![Cw20Coin {
                    address: minter.to_string(),
                    amount: self.initial_mint
                }],
                mint: Some(MinterResponse {
                    minter: minter.to_string(),
                    cap: None
                }),
                marketing: None
            },
            &[],
            self.symbol.clone(),
            None
        ).unwrap()
    }

}

impl Into<cw20_base::msg::InstantiateMsg> for TestTokenDefinition {
    fn into(self) -> cw20_base::msg::InstantiateMsg {
        cw20_base::msg::InstantiateMsg {
            name: self.name,
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            initial_balances: vec![Cw20Coin {
                address: self.minter.clone(),
                amount: self.initial_mint
            }],
            mint: Some(MinterResponse {
                minter: self.minter,
                cap: None
            }),
            marketing: None
        }
    }
}


pub fn mock_test_token_definitions(
    minter: String,
    count: usize
) -> Vec<TestTokenDefinition> {
    vec![
        TestTokenDefinition {
            name: "Test Token A".to_string(),
            symbol: "TTA".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD,
            minter: minter.clone()
        },
        TestTokenDefinition {
            name: "Test Token B".to_string(),
            symbol: "TTB".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD,
            minter: minter.clone()
        },
        TestTokenDefinition {
            name: "Test Token C".to_string(),
            symbol: "TTC".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD,
            minter: minter.clone()
        },
        TestTokenDefinition {
            name: "Test Token D".to_string(),
            symbol: "TTD".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD,
            minter: minter.clone()
        },
        TestTokenDefinition {
            name: "Test Token E".to_string(),
            symbol: "TTE".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD,
            minter
        }
    ][0..count].to_vec()
}


pub fn cw20_contract_storage<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>
) -> u64
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{

    // Create contract wrapper
    let contract = ContractWrapper::new_with_empty(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query
    );

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}


pub fn deploy_test_tokens<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    minter: String,
    cw20_contract: Option<u64>,
    count: usize
) -> Vec<Addr>
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{

    deploy_test_token_definitions(
        app,
        minter.clone(),
        cw20_contract,
        mock_test_token_definitions(minter, count)
    )

}


pub fn deploy_test_token_definitions<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    minter: String,
    cw20_contract: Option<u64>,
    token_definitions: Vec<TestTokenDefinition>
) -> Vec<Addr>
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{

    let cw20_contract = cw20_contract.unwrap_or(cw20_contract_storage(app));

    token_definitions
        .iter()
        .map(|definition| {
            app.instantiate_contract::<cw20_base::msg::InstantiateMsg, _>(
                cw20_contract,
                Addr::unchecked(minter.clone()),
                &(definition.clone()).into(),
                &[],
                definition.symbol.clone(),
                None
            ).unwrap()
        })
        .collect()
}


pub fn query_token_balance<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    asset: Addr,
    account: String
) -> Uint128
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{
    
    app.wrap().query_wasm_smart::<BalanceResponse>(
        asset,
        &Cw20QueryMsg::Balance { address: account }
    ).unwrap().balance

}


pub fn query_token_info<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    asset: Addr
) -> TokenInfoResponse
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{
    
    app.wrap().query_wasm_smart::<TokenInfoResponse>(
        asset,
        &Cw20QueryMsg::TokenInfo {}
    ).unwrap()

}


pub fn get_token_allowance<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    asset: Addr,
    account: Addr,
    spender: String,
) -> AllowanceResponse
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{

    app.wrap().query_wasm_smart(
        asset,
        &Cw20QueryMsg::Allowance {
            owner: account.to_string(),
            spender
        }
    ).unwrap()

}


pub fn increase_token_allowance<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    amount: Uint128,
    asset: Addr,
    account: Addr,
    spender: String,
) -> AppResponse
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{
    app.execute_contract::<Cw20ExecuteMsg>(
        account,
        asset,
        &Cw20ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires: None
        },
        &[]
    ).unwrap()
}


pub fn decrease_token_allowance<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    amount: Uint128,
    asset: Addr,
    account: Addr,
    spender: String,
) -> AppResponse
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{
    app.execute_contract::<Cw20ExecuteMsg>(
        account,
        asset,
        &Cw20ExecuteMsg::DecreaseAllowance {
            spender,
            amount,
            expires: None
        },
        &[]
    ).unwrap()
}


pub fn transfer_tokens<HandlerC, ExecC>(
    app: &mut CustomApp<HandlerC, ExecC>,
    amount: Uint128,
    asset: Addr,
    account: Addr,
    recipient: String
) -> AppResponse
where
    HandlerC: Module<ExecT = ExecC, QueryT = Empty, SudoT = Empty>,
    ExecC: Clone + Debug + PartialEq + JsonSchema + Serialize + DeserializeOwned + 'static
{
    app.execute_contract::<Cw20ExecuteMsg>(
        account,
        asset,
        &Cw20ExecuteMsg::Transfer {
            recipient,
            amount
        },
        &[]
    ).unwrap()
}

