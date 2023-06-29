use cosmwasm_std::{Uint128, Addr, Binary, Uint64};
use cw20::{Cw20ExecuteMsg};
use cw_multi_test::{ContractWrapper, App, Executor, AppResponse};
use catalyst_types::U256;
use catalyst_vault_common::msg::InstantiateMsg;
use test_helpers::{math::{u256_to_f64, uint128_to_f64}, misc::get_response_attribute, definitions::{SETUP_MASTER, FACTORY_OWNER}};

use crate::msg::VolatileExecuteMsg;


pub const DEFAULT_TEST_VAULT_FEE : Uint64 = Uint64::new(70000000000000000u64);   // 7%
pub const DEFAULT_TEST_GOV_FEE  : Uint64 = Uint64::new(50000000000000000u64);   // 5%

//TODO move common helpers somewhere else

// Contracts
pub fn vault_factory_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        catalyst_factory::contract::execute,
        catalyst_factory::contract::instantiate,
        catalyst_factory::contract::query,
    ).with_reply(catalyst_factory::contract::reply);
    
    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}

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

pub fn interface_contract_storage(
    app: &mut App
) -> u64 {

    // Create contract wrapper
    let contract = ContractWrapper::new(
        mock_catalyst_ibc_interface::contract::execute,
        mock_catalyst_ibc_interface::contract::instantiate,
        mock_catalyst_ibc_interface::contract::query,
    );

    // 'Deploy' the contract
    app.store_code(Box::new(contract))
}



// Interface helpers

pub fn mock_instantiate_interface(
    app: &mut App
) -> Addr {

    let contract_code_storage = interface_contract_storage(app);

    app.instantiate_contract(
        contract_code_storage,
        Addr::unchecked(SETUP_MASTER),
        &catalyst_ibc_interface::msg::InstantiateMsg {},
        &[],
        "interface",
        None
    ).unwrap()
}


// Factory management helpers
pub fn mock_instantiate_factory(
    app: &mut App,
    default_governance_fee_share: Option<Uint64>
) -> Addr {

    let factory_contract_code = vault_factory_contract_storage(app);

    app.instantiate_contract(
        factory_contract_code,
        Addr::unchecked(FACTORY_OWNER),
        &catalyst_factory::msg::InstantiateMsg {
            default_governance_fee_share: default_governance_fee_share.unwrap_or(DEFAULT_TEST_GOV_FEE)
        },
        &[],
        "factory",
        None
    ).unwrap()
}

pub fn mock_factory_deploy_vault(
    app: &mut App,
    assets: Vec<String>,
    assets_balances: Vec<Uint128>,
    weights: Vec<Uint64>,
    vault_code_id: Option<u64>,
    chain_interface: Option<Addr>,
    factory: Option<Addr>
) -> Addr {

    // Deploy factory if not provided
    let factory = factory.unwrap_or(
        mock_instantiate_factory(app, None)
    );

    // Deploy vault contract if not provided
    let vault_code_id = vault_code_id.unwrap_or(
        volatile_vault_contract_storage(app)
    );

    // Set asset allowances for the factory
    assets
        .iter()
        .zip(&assets_balances)
        .filter(|(_, amount)| *amount != Uint128::zero())
        .for_each(|(asset, amount)| {
            app.execute_contract::<Cw20ExecuteMsg>(
                Addr::unchecked(SETUP_MASTER),
                Addr::unchecked(asset),
                &Cw20ExecuteMsg::IncreaseAllowance {
                    spender: factory.to_string(),
                    amount: *amount,
                    expires: None
                },
                &[]
            ).unwrap();
        });

    // Deploy the new vault
    let response = app.execute_contract(
        Addr::unchecked(SETUP_MASTER),
        factory,
        &catalyst_factory::msg::ExecuteMsg::DeployVault {
            vault_code_id,
            assets,
            assets_balances,
            weights,
            amplification: Uint64::new(1000000000000000000u64),
            vault_fee: DEFAULT_TEST_VAULT_FEE,
            name: "TestVault".to_string(),
            symbol: "TP".to_string(),
            chain_interface: chain_interface.map(|value| value.to_string())
        },
        &[]
    ).unwrap();

    let vault = get_response_attribute::<String>(response.events[6].clone(), "vault_address").unwrap();

    Addr::unchecked(vault)
}


// Vault management helpers

pub fn mock_instantiate_vault_msg(
    chain_interface: Option<String>
) -> InstantiateMsg {
    InstantiateMsg {
        name: "TestVault".to_string(),
        symbol: "TP".to_string(),
        chain_interface,
        vault_fee: DEFAULT_TEST_VAULT_FEE,
        governance_fee_share: DEFAULT_TEST_GOV_FEE,
        fee_administrator: FACTORY_OWNER.to_string(),   // The 'fee_administrator' is set to the 'factory_owner' as this is the default when vaults are deployed via the factory
        setup_master: SETUP_MASTER.to_string()
    }
}

pub fn mock_instantiate_vault(
    app: &mut App,
    chain_interface: Option<Addr>
) -> Addr {

    let instantiate_msg = mock_instantiate_vault_msg(
        chain_interface.map(|addr| addr.to_string())
    );

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
pub struct InitializeSwapCurvesMockConfig {
    pub assets: Vec<String>,
    pub assets_balances: Vec<Uint128>,
    pub weights: Vec<Uint64>,
    pub amp: Uint64,
    pub depositor: String
}

impl InitializeSwapCurvesMockConfig {
    pub fn transfer_vault_allowances(
        &self,
        app: &mut App,
        vault: String,
        depositor: Addr
    ) {
        self.assets
            .iter()
            .zip(&self.assets_balances)
            .filter(|(_, amount)| *amount != Uint128::zero())
            .for_each(|(asset, amount)| {
                app.execute_contract::<Cw20ExecuteMsg>(
                    depositor.clone(),
                    Addr::unchecked(asset),
                    &Cw20ExecuteMsg::Transfer {
                        recipient: vault.clone(),
                        amount: *amount
                    },
                    &[]
                ).unwrap();
            });
    }
}

impl Into<VolatileExecuteMsg> for InitializeSwapCurvesMockConfig {
    fn into(self) -> VolatileExecuteMsg {
        VolatileExecuteMsg::InitializeSwapCurves {
            assets: self.assets,
            weights: self.weights,
            amp: self.amp,
            depositor: self.depositor
        }
    }
}


pub fn mock_initialize_vault(
    app: &mut App,
    vault: Addr,
    assets: Vec<String>,
    assets_balances: Vec<Uint128>,
    weights: Vec<Uint64>
) -> InitializeSwapCurvesMockConfig {

    // Define InitializeSwapCurves parameters
    let initialize_msg = InitializeSwapCurvesMockConfig {
        assets,
        assets_balances,
        weights,
        amp: Uint64::new(1000000000000000000u64),
        depositor: SETUP_MASTER.to_string()
    };

    // Transfer the tokens to the vault
    initialize_msg.transfer_vault_allowances(
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


pub fn mock_finish_vault_setup(
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


pub fn mock_set_vault_connection(
    app: &mut App,
    vault_contract: Addr,
    channel_id: String,
    to_vault: Binary,
    state: bool
) -> AppResponse {
    app.execute_contract::<VolatileExecuteMsg>(
        Addr::unchecked(FACTORY_OWNER),
        vault_contract,
        &VolatileExecuteMsg::SetConnection {
            channel_id,
            to_vault,
            state
        },
        &[]
    ).unwrap()
}


pub fn mock_set_vault_fee(
    app: &mut App,
    vault_contract: Addr,
    fee: Uint64
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(FACTORY_OWNER),
        vault_contract,
        &VolatileExecuteMsg::SetVaultFee { fee },
        &[]
    ).unwrap()
}


pub fn mock_set_governance_fee_share(
    app: &mut App,
    vault_contract: Addr,
    fee: Uint64
) -> AppResponse {
    app.execute_contract(
        Addr::unchecked(FACTORY_OWNER),
        vault_contract,
        &VolatileExecuteMsg::SetGovernanceFeeShare { fee },
        &[]
    ).unwrap()
}



// Swap Utils

pub struct ExpectedLocalSwapResult {
    pub u: f64,
    pub to_amount: f64,
    pub vault_fee: f64,
    pub governance_fee: f64
}

pub struct ExpectedSendAssetResult {
    pub u: f64,
    pub vault_fee: f64,
    pub governance_fee: f64
}

pub struct ExpectedReceiveAssetResult {
    pub to_amount: f64,
}

pub fn compute_expected_local_swap(
    swap_amount: Uint128,
    from_weight: Uint64,
    from_balance: Uint128,
    to_weight: Uint64,
    to_balance: Uint128,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedLocalSwapResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight.u64() as f64;
    let from_balance = from_balance.u128() as f64;
    let to_weight = to_weight.u64() as f64;
    let to_balance = to_balance.u128() as f64;

    // Compute fees
    let vault_fee = (vault_fee.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;
    let governance_fee_share = (governance_fee_share.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;

    let net_fee = vault_fee * swap_amount;
    let net_vault_fee = vault_fee * (1. - governance_fee_share) * swap_amount;
    let net_governance_fee = vault_fee * governance_fee_share * swap_amount;

    // Compute swap
    let x = swap_amount - net_fee;
    let u = from_weight * ((from_balance + x)/from_balance).ln();
    let to_amount = to_balance * (1. - (-u/to_weight).exp());

    ExpectedLocalSwapResult {
        u,
        to_amount,
        vault_fee: net_vault_fee,
        governance_fee: net_governance_fee
    }
}

pub fn compute_expected_send_asset(
    swap_amount: Uint128,
    from_weight: Uint64,
    from_balance: Uint128,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedSendAssetResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight.u64() as f64;
    let from_balance = from_balance.u128() as f64;

    // Compute fees
    let vault_fee = (vault_fee.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;
    let governance_fee_share = (governance_fee_share.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;

    let net_fee = vault_fee * swap_amount;
    let net_vault_fee = vault_fee * (1. - governance_fee_share) * swap_amount;
    let net_governance_fee = vault_fee * governance_fee_share * swap_amount;

    // Compute swap
    let x = swap_amount - net_fee;
    let u = from_weight * ((from_balance + x)/from_balance).ln();

    ExpectedSendAssetResult {
        u,
        vault_fee: net_vault_fee,
        governance_fee: net_governance_fee
    }
}

pub fn compute_expected_receive_asset(
    u: U256,
    to_weight: Uint64,
    to_balance: Uint128
) -> ExpectedReceiveAssetResult {

    // Convert arguments into float
    let u = u256_to_f64(u) / 1e18;
    let to_weight = to_weight.u64() as f64;
    let to_balance = to_balance.u128() as f64;

    // Compute swap
    ExpectedReceiveAssetResult {
        to_amount: to_balance * (1. - (-u/to_weight).exp())
    }
    
}

pub struct ExpectedSendLiquidityResult {
    pub u: f64
}

pub struct ExpectedReceiveLiquidityResult {
    pub to_amount: f64
}

pub struct ExpectedReferenceAsset {
    pub amount: f64
}

pub fn compute_expected_send_liquidity(
    swap_amount: Uint128,
    from_weights: Vec<Uint64>,
    from_total_supply: Uint128
) -> ExpectedSendLiquidityResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_total_supply = from_total_supply.u128() as f64;

    // Compute swap
    let from_weights_sum: f64 = from_weights.iter().sum::<Uint64>().u64() as f64;
    let u = (from_total_supply/(from_total_supply - swap_amount)).ln() * from_weights_sum;

    ExpectedSendLiquidityResult {
        u
    }

}

pub fn compute_expected_receive_liquidity(
    u: U256,
    to_weights: Vec<Uint64>,
    to_total_supply: Uint128
) -> ExpectedReceiveLiquidityResult {

    // Convert arguments to float
    let u = u256_to_f64(u) / 1e18;
    let to_total_supply = to_total_supply.u128() as f64;

    // Compute swap
    let to_weights_sum: f64 = to_weights.iter().sum::<Uint64>().u64() as f64;
    let share = 1. - (-u/to_weights_sum).exp();
    let to_amount = to_total_supply * (share/(1.-share));

    ExpectedReceiveLiquidityResult {
        to_amount
    }

}

pub fn compute_expected_reference_asset(
    vault_tokens: Uint128,
    vault_balances: Vec<Uint128>,
    vault_weights: Vec<Uint64>,
    vault_total_supply: Uint128,
    vault_escrowed_vault_tokens: Uint128
) -> ExpectedReferenceAsset {

    let weights_sum = vault_weights.iter().sum::<Uint64>().u64() as f64;

    let vault_reference_amount: f64 = vault_balances.iter()
        .zip(vault_weights)
        .map(|(balance, weight)| {

            let balance = uint128_to_f64(*balance);
            let weight = weight.u64() as f64;

            balance.powf(weight/weights_sum)
        })
        .product::<f64>();

    let vault_tokens = uint128_to_f64(vault_tokens);
    let vault_total_supply = uint128_to_f64(vault_total_supply);
    let vault_escrowed_vault_tokens = uint128_to_f64(vault_escrowed_vault_tokens);

    let user_reference_amount = (vault_reference_amount * vault_tokens) / (vault_total_supply + vault_escrowed_vault_tokens + vault_tokens);

    ExpectedReferenceAsset {
        amount: user_reference_amount
    }
}


pub fn compute_expected_deposit_mixed(
    deposit_amounts: Vec<Uint128>,
    from_weights: Vec<Uint64>,
    from_balances: Vec<Uint128>,
    from_total_supply: Uint128,
    vault_fee: Option<Uint64>,
) -> f64 {
    
    // Compute units
    let units: f64 = deposit_amounts.iter()
        .zip(&from_weights)
        .zip(&from_balances)
        .map(|((deposit_amount, from_weight), from_balance)| {
            let deposit_amount = uint128_to_f64(*deposit_amount);
            let from_weight = from_weight.u64() as f64;
            let from_balance = uint128_to_f64(*from_balance);

            from_weight * (1. + deposit_amount/from_balance).ln()
        })
        .sum();

    // Take vault fee
    let units = units * (1. - (vault_fee.unwrap_or(Uint64::zero()).u64() as f64)/1e18);

    // Compute the deposit share
    let weights_sum = from_weights.iter().sum::<Uint64>().u64() as f64;
    let from_total_supply = uint128_to_f64(from_total_supply);

    let deposit_share = (units / weights_sum).exp() - 1.;

    from_total_supply * deposit_share

}


pub fn compute_expected_withdraw_mixed(
    withdraw_amount: Uint128,
    withdraw_ratio: Vec<Uint64>,
    vault_weights: Vec<Uint64>,
    vault_balances: Vec<Uint128>,
    vault_supply: Uint128
) -> Vec<f64> {

    // Compute the units corresponding to the vault tokens
    let withdraw_amount = uint128_to_f64(withdraw_amount);
    let vault_supply = uint128_to_f64(vault_supply);

    let vault_weights_sum = vault_weights.iter().sum::<Uint64>().u64() as f64;

    let mut units: f64 = (
        vault_supply/(vault_supply - withdraw_amount)
    ).ln() * vault_weights_sum;

    vault_balances.iter()
        .zip(vault_weights)
        .zip(withdraw_ratio)
        .map(|((balance, weight), ratio)| {

            let balance = uint128_to_f64(*balance);
            let weight = weight.u64() as f64;
            let ratio = (ratio.u64() as f64) / 1e18;

            let units_for_asset = units * ratio;
            if units_for_asset > units {
                panic!("Invalid withdraw ratios.");
            }
            units -= units_for_asset;

            balance * (
                1. - (-units_for_asset / weight).exp()
            )
        })
        .collect::<Vec<f64>>()

}

