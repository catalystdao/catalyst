use std::str::FromStr;

use cosmwasm_std::{Uint128, Addr, Event};
use cw20::{Cw20Coin, MinterResponse, Cw20ExecuteMsg, BalanceResponse, Cw20QueryMsg, TokenInfoResponse};
use cw_multi_test::{ContractWrapper, App, Executor, AppResponse};
use ethnum::U256;
use swap_pool_common::msg::InstantiateMsg;

use crate::{msg::VolatileExecuteMsg, tests::math_helpers::{u256_to_f64, uint128_to_f64}};

pub const DEPLOYER              : &str = "deployer_addr";
pub const FACTORY_OWNER         : &str = "factory_owner_addr";
pub const SETUP_MASTER          : &str = "setup_master_addr";
pub const CHAIN_INTERFACE       : &str = "chain_interface";
pub const DEPOSITOR             : &str = "depositor_addr";
pub const WITHDRAWER            : &str = "withdrawer_addr";
pub const FEE_ADMINISTRATOR     : &str = "fee_administrator_addr";
pub const LOCAL_SWAPPER         : &str = "local_swapper_addr";

pub const WAD: Uint128 = Uint128::new(1000000000000000000u128);

pub const DEFAULT_TEST_POOL_FEE : u64 = 70000000000000000u64;   // 7%
pub const DEFAULT_TEST_GOV_FEE  : u64 = 50000000000000000u64;   // 5%


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
                address: SETUP_MASTER.to_string(),
                amount: self.initial_mint
            }],
            mint: Some(MinterResponse {
                minter: SETUP_MASTER.to_string(),
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
                Addr::unchecked(SETUP_MASTER),
                &(definition.clone()).into(),
                &[],
                definition.symbol.clone(),
                None
            ).unwrap()
        })
        .collect()
}

pub fn query_token_balance(
    app: &mut App,
    asset: Addr,
    account: String
) -> Uint128 {
    
    app.wrap().query_wasm_smart::<BalanceResponse>(
        asset,
        &Cw20QueryMsg::Balance { address: account }
    ).unwrap().balance

}

pub fn query_token_info(
    app: &mut App,
    asset: Addr
) -> TokenInfoResponse {
    
    app.wrap().query_wasm_smart::<TokenInfoResponse>(
        asset,
        &Cw20QueryMsg::TokenInfo {}
    ).unwrap()

}

pub fn set_token_allowance(
    app: &mut App,
    amount: Uint128,
    asset: Addr,
    account: Addr,
    spender: String,
) -> AppResponse {
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

pub fn transfer_tokens(
    app: &mut App,
    amount: Uint128,
    asset: Addr,
    account: Addr,
    recipient: String
) -> AppResponse {
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


// Vault management helpers

pub fn mock_instantiate_msg(
    chain_interface: Option<String>
) -> InstantiateMsg {
    InstantiateMsg {
        name: "TestPool".to_string(),
        symbol: "TP".to_string(),
        chain_interface,
        pool_fee: DEFAULT_TEST_POOL_FEE,
        governance_fee: DEFAULT_TEST_GOV_FEE,
        fee_administrator: FEE_ADMINISTRATOR.to_string(),
        setup_master: SETUP_MASTER.to_string()
    }
}

pub fn mock_instantiate(
    app: &mut App,
    only_local: bool
) -> Addr {

    let chain_interface = match only_local {
        true => None,
        false => Some(CHAIN_INTERFACE.to_string())
    };

    let instantiate_msg = mock_instantiate_msg(chain_interface);

    let contract_code_storage = volatile_vault_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
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
        depositor: Addr
    ) {
        self.assets
            .iter()
            .zip(&self.assets_balances)
            .for_each(|(asset, amount)| {
                app.execute_contract::<Cw20ExecuteMsg>(
                    depositor.clone(),
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


pub fn mock_initialize_pool(
    app: &mut App,
    vault: Addr,
    assets: Vec<String>,
    assets_balances: Vec<Uint128>,
    weights: Vec<u64>
) -> InitializeSwapCurvesMockMsg {

    // Define InitializeSwapCurves parameters
    let initialize_msg = InitializeSwapCurvesMockMsg {
        assets,
        assets_balances,
        weights,
        amp: 1000000000000000000u64,
        depositor: SETUP_MASTER.to_string()
    };

    // Set token allowances
    initialize_msg.set_vault_allowances(
        app,
        vault.to_string(),
        Addr::unchecked(SETUP_MASTER)
    );

    // Execute initialize swap curves
    app.execute_contract::<VolatileExecuteMsg>(
        Addr::unchecked(SETUP_MASTER),
        vault,
        &initialize_msg.clone().into(),
        &[]
    ).unwrap();

    initialize_msg

}


pub fn mock_finish_pool_setup(
    app: &mut App,
    vault_contract: Addr
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(SETUP_MASTER),
        vault_contract,
        &VolatileExecuteMsg::FinishSetup {},
        &[]
    ).unwrap()
}


pub fn mock_set_pool_fee(
    app: &mut App,
    vault_contract: Addr,
    fee: u64
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(FEE_ADMINISTRATOR),
        vault_contract,
        &VolatileExecuteMsg::SetPoolFee { fee },
        &[]
    ).unwrap()
}


pub fn mock_set_governance_fee_share(
    app: &mut App,
    vault_contract: Addr,
    fee: u64
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(FEE_ADMINISTRATOR),
        vault_contract,
        &VolatileExecuteMsg::SetGovernanceFeeShare { fee },
        &[]
    ).unwrap()
}



// Swap Utils

pub struct ExpectedLocalSwapResult {
    pub u: f64,
    pub to_amount: f64,
    pub pool_fee: f64,
    pub governance_fee: f64
}

pub fn compute_expected_swap(
    swap_amount: Uint128,
    from_weight: u64,
    from_balance: Uint128,
    to_weight: u64,
    to_balance: Uint128,
    pool_fee: Option<u64>,
    governance_fee_share: Option<u64>
) -> ExpectedLocalSwapResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight as f64;
    let from_balance = from_balance.u128() as f64;
    let to_weight = to_weight as f64;
    let to_balance = to_balance.u128() as f64;

    // Compute fees
    let pool_fee = (pool_fee.unwrap_or(0u64) as f64) / 1e18;
    let governance_fee_share = (governance_fee_share.unwrap_or(0u64) as f64) / 1e18;

    let net_fee = pool_fee * swap_amount;
    let net_pool_fee = pool_fee * (1. - governance_fee_share) * swap_amount;
    let net_governance_fee = pool_fee * governance_fee_share * swap_amount;

    // Compute swap
    let x = swap_amount - net_fee;
    let u = from_weight * ((from_balance + x)/from_balance).ln();
    let to_amount = to_balance * (1. - (-u/to_weight).exp());

    ExpectedLocalSwapResult {
        u,
        to_amount,
        pool_fee: net_pool_fee,
        governance_fee: net_governance_fee
    }
}

pub fn compute_expected_swap_given_u(
    u: U256,
    to_weight: u64,
    to_balance: Uint128
) -> f64 {

    // Convert arguments into float
    let u = u256_to_f64(u);
    let to_weight = to_weight as f64;
    let to_balance = to_balance.u128() as f64;

    // Compute swap
    to_balance * (1. - (-u/to_weight).exp())
    
}

pub struct ExpectedLiquiditySwapResult {
    pub u: f64,
    pub to_amount: f64
}

pub fn compute_expected_liquidity_swap(
    swap_amount: Uint128,
    from_weights: Vec<u64>,
    from_total_supply: Uint128,
    to_weights: Vec<u64>,
    to_total_supply: Uint128
) -> ExpectedLiquiditySwapResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_total_supply = from_total_supply.u128() as f64;
    let to_total_supply = to_total_supply.u128() as f64;

    // Compute swap
    let from_weights_sum: f64 = from_weights.iter().sum::<u64>() as f64;
    let to_weights_sum: f64 = to_weights.iter().sum::<u64>() as f64;

    let u = (from_total_supply/(from_total_supply - swap_amount)).ln() * from_weights_sum;

    let share = 1. - (-u/to_weights_sum).exp();
    let to_amount = to_total_supply * (share/(1.-share));

    ExpectedLiquiditySwapResult {
        u,
        to_amount
    }

}


pub fn compute_expected_deposit_mixed(
    deposit_amounts: Vec<Uint128>,
    from_weights: Vec<u64>,
    from_balances: Vec<Uint128>,
    from_total_supply: Uint128,
    pool_fee: Option<u64>,
) -> f64 {
    
    // Compute units
    let units: f64 = deposit_amounts.iter()
        .zip(&from_weights)
        .zip(&from_balances)
        .map(|((deposit_amount, from_weight), from_balance)| {
            let deposit_amount = uint128_to_f64(*deposit_amount);
            let from_weight = *from_weight as f64;
            let from_balance = uint128_to_f64(*from_balance);

            from_weight * (1. + deposit_amount/from_balance).ln()
        })
        .sum();

    // Take pool fee
    let units = units * (1. - (pool_fee.unwrap_or(0u64) as f64)/1e18);

    // Compute the deposit share
    let weights_sum = from_weights.iter().sum::<u64>() as f64;
    let from_total_supply = uint128_to_f64(from_total_supply);

    let deposit_share = (units / weights_sum).exp() - 1.;

    from_total_supply * deposit_share

}



// Misc helpers

pub fn get_response_attribute<T: FromStr>(event: Event, attribute: &str) -> Result<T, String> {
    event.attributes
        .iter()
        .find(|attr| attr.key == attribute).ok_or("Attribute not found")?
        .value
        .parse::<T>().map_err(|_| "Parse error".to_string())
}