use cosmwasm_std::{Uint128, Addr};
use cw20::{Cw20Coin, MinterResponse, Cw20ExecuteMsg};
use cw_multi_test::{ContractWrapper, App, Executor, AppResponse};
use swap_pool_common::msg::InstantiateMsg;

use crate::msg::VolatileExecuteMsg;

pub const DEPLOYER_ADDR         : &str = "deployer_addr";
pub const FACTORY_OWNER_ADDR    : &str = "factory_owner_addr";
pub const SETUP_MASTER_ADDR     : &str = "setup_master_addr";
pub const CHAIN_INTERFACE_ADDR  : &str = "chain_interface";
pub const DEPOSITOR_ADDR        : &str = "depositor_addr";
pub const FEE_ADMINISTRATOR     : &str = "fee_administrator_addr";
pub const LOCAL_SWAPPER         : &str = "local_swapper_addr";

pub const WAD: Uint128 = Uint128::new(1000000000000000000u128);


// Contracts
pub fn volatile_vault_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        crate::contract::execute,
        crate::contract::instantiate,
        crate::contract::query,
    );
    
    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

pub fn cw20_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        cw20_base::contract::execute,
        cw20_base::contract::instantiate,
        cw20_base::contract::query
    );

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}



// Test tokens helpers

#[derive(Clone)]
pub struct TestTokenDefinition {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_mint: Uint128
}

impl Into<cw20_base::msg::InstantiateMsg> for TestTokenDefinition {
    fn into(self) -> cw20_base::msg::InstantiateMsg {
        cw20_base::msg::InstantiateMsg {
            name: self.name,
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            initial_balances: vec![Cw20Coin {
                address: SETUP_MASTER_ADDR.to_string(),
                amount: self.initial_mint
            }],
            mint: Some(MinterResponse {
                minter: SETUP_MASTER_ADDR.to_string(),
                cap: None
            }),
            marketing: None
        }
    }
}

pub fn mock_test_token_definitions(count: usize) -> Vec<TestTokenDefinition> {
    vec![
        TestTokenDefinition {
            name: "Test Token A".to_string(),
            symbol: "TTA".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD
        },
        TestTokenDefinition {
            name: "Test Token B".to_string(),
            symbol: "TTB".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD
        },
        TestTokenDefinition {
            name: "Test Token C".to_string(),
            symbol: "TTC".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD
        },
        TestTokenDefinition {
            name: "Test Token D".to_string(),
            symbol: "TTD".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD
        },
        TestTokenDefinition {
            name: "Test Token E".to_string(),
            symbol: "TTE".to_string(),
            decimals: 18,
            initial_mint: Uint128::from(100000000u64) * WAD
        }
    ][0..count].to_vec()
}

pub fn deploy_test_tokens(
    app: &mut App,
    cw20_contract: Option<u64>,
    token_definitions: Option<Vec<TestTokenDefinition>>
) -> Vec<Addr> {

    let cw20_contract = cw20_contract.unwrap_or(cw20_contract_storage(app));

    token_definitions
        .unwrap_or(mock_test_token_definitions(3))
        .iter()
        .map(|definition| {
            app.instantiate_contract::<cw20_base::msg::InstantiateMsg, _>(
                cw20_contract,
                Addr::unchecked(SETUP_MASTER_ADDR),
                &(definition.clone()).into(),
                &[],
                definition.symbol.clone(),
                None
            ).unwrap()
        })
        .collect()
}



// Vault management helpers

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

pub fn mock_instantiate(
    app: &mut App,
    only_local: bool
) -> Addr {

    let chain_interface = match only_local {
        true => None,
        false => Some(CHAIN_INTERFACE_ADDR.to_string())
    };

    let instantiate_msg = mock_instantiate_msg(chain_interface);

    let contract_code_storage = volatile_vault_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER_ADDR),
        &instantiate_msg,
        &[],
        "vault",
        None
    ).unwrap()
}

#[derive(Clone)]
pub struct InitializeSwapCurvesMockMsg {
    pub assets: Vec<String>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<u64>,
    pub amp: u64,
    pub depositor: String
}

impl InitializeSwapCurvesMockMsg {
    pub fn set_vault_allowances(
        &self,
        app: &mut App,
        vault: String,
        spender: Addr
    ) {
        self.assets
            .iter()
            .zip(&self.assets_balances)
            .for_each(|(asset, amount)| {
                app.execute_contract::<Cw20ExecuteMsg>(
                    spender.clone(),
                    Addr::unchecked(asset),
                    &Cw20ExecuteMsg::IncreaseAllowance {
                        spender: vault.to_string(),
                        amount: *amount,
                        expires: None
                    },
                    &[]
                ).unwrap();
            });
    }
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

pub fn mock_finish_pool_setup(
    app: &mut App,
    vault_contract: Addr
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(SETUP_MASTER_ADDR),
        vault_contract,
        &VolatileExecuteMsg::FinishSetup {},
        &[]
    ).unwrap()
}