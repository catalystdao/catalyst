use cosmwasm_std::{Uint128, Uint64};
use cw_multi_test::{ContractWrapper, App};
use catalyst_types::U256;
use test_helpers::{math::{u256_to_f64, uint128_to_f64}, contract::{ExpectedLocalSwapResult, ExpectedSendAssetResult, ExpectedReceiveAssetResult, ExpectedSendLiquidityResult, ExpectedReceiveLiquidityResult, ExpectedReferenceAsset}};



// Contracts
pub fn amplified_vault_contract_storage(
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



// Swap Utils

pub fn compute_expected_local_swap(
    swap_amount: Uint128,
    from_weight: Uint64,
    from_balance: Uint128,
    to_weight: Uint64,
    to_balance: Uint128,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedLocalSwapResult {

    todo!();

}

pub fn compute_expected_send_asset(
    swap_amount: Uint128,
    from_weight: Uint64,
    from_balance: Uint128,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedSendAssetResult {

    todo!();

}

pub fn compute_expected_receive_asset(
    u: U256,
    to_weight: Uint64,
    to_balance: Uint128
) -> ExpectedReceiveAssetResult {

    todo!();
    
}


pub fn compute_expected_send_liquidity(
    swap_amount: Uint128,
    from_weights: Vec<Uint64>,
    from_total_supply: Uint128
) -> ExpectedSendLiquidityResult {

    todo!();

}

pub fn compute_expected_receive_liquidity(
    u: U256,
    to_weights: Vec<Uint64>,
    to_total_supply: Uint128
) -> ExpectedReceiveLiquidityResult {

    todo!();

}

pub fn compute_expected_reference_asset(
    vault_tokens: Uint128,
    vault_balances: Vec<Uint128>,
    vault_weights: Vec<Uint64>,
    vault_total_supply: Uint128,
    vault_escrowed_vault_tokens: Uint128
) -> ExpectedReferenceAsset {

    todo!();

}


pub fn compute_expected_deposit_mixed(
    deposit_amounts: Vec<Uint128>,
    from_weights: Vec<Uint64>,
    from_balances: Vec<Uint128>,
    from_total_supply: Uint128,
    vault_fee: Option<Uint64>,
) -> f64 {

    todo!();

}


pub fn compute_expected_withdraw_mixed(
    withdraw_amount: Uint128,
    withdraw_ratio: Vec<Uint64>,
    vault_weights: Vec<Uint64>,
    vault_balances: Vec<Uint128>,
    vault_supply: Uint128
) -> Vec<f64> {

    todo!();

}

