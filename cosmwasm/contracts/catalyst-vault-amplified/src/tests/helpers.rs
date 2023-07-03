use cosmwasm_std::{Uint128, Uint64};
use cw_multi_test::{ContractWrapper, App};
use catalyst_types::{U256, I256};
use test_helpers::{math::{u256_to_f64, uint128_to_f64, i256_to_f64}, contract::{ExpectedLocalSwapResult, ExpectedSendAssetResult, ExpectedReceiveAssetResult, ExpectedSendLiquidityResult, ExpectedReceiveLiquidityResult, ExpectedReferenceAsset}};



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
    amplification: Uint64,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedLocalSwapResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight.u64() as f64;
    let from_balance = from_balance.u128() as f64;
    let to_weight = to_weight.u64() as f64;
    let to_balance = to_balance.u128() as f64;
    let amplification = (amplification.u64() as f64) / 1e18;

    // Compute fees
    let vault_fee = (vault_fee.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;
    let governance_fee_share = (governance_fee_share.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;

    let net_fee = vault_fee * swap_amount;
    let net_vault_fee = vault_fee * (1. - governance_fee_share) * swap_amount;
    let net_governance_fee = vault_fee * governance_fee_share * swap_amount;

    // Compute swap
    let x = swap_amount - net_fee;
    let one_minus_amplification = 1. - amplification;

    let weighted_from_balance = from_weight * from_balance;
    let weighted_to_balance = to_weight * to_balance;
    let weighted_swap_amount = from_weight * x;

    let weighted_to_balance_ampped = weighted_to_balance.powf(one_minus_amplification);

    let u = (weighted_from_balance + weighted_swap_amount).powf(one_minus_amplification) - weighted_from_balance.powf(one_minus_amplification);

    let to_amount = to_balance * (
        1. - ((weighted_to_balance_ampped - u)/weighted_to_balance_ampped).powf(1./one_minus_amplification)
    );

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
    amplification: Uint64,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedSendAssetResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight.u64() as f64;
    let from_balance = from_balance.u128() as f64;
    let amplification = (amplification.u64() as f64) / 1e18;

    // Compute fees
    let vault_fee = (vault_fee.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;
    let governance_fee_share = (governance_fee_share.unwrap_or(Uint64::zero()).u64() as f64) / 1e18;

    let net_fee = vault_fee * swap_amount;
    let net_vault_fee = vault_fee * (1. - governance_fee_share) * swap_amount;
    let net_governance_fee = vault_fee * governance_fee_share * swap_amount;

    // Compute swap
    let x = swap_amount - net_fee;
    let one_minus_amplification = 1. - amplification;

    let weighted_from_balance = from_weight * from_balance;
    let weighted_swap_amount = from_weight * x;

    let u = (weighted_from_balance + weighted_swap_amount).powf(one_minus_amplification) - weighted_from_balance.powf(one_minus_amplification);

    ExpectedSendAssetResult {
        u,
        vault_fee: net_vault_fee,
        governance_fee: net_governance_fee
    }

}

pub fn compute_expected_receive_asset(
    u: U256,
    to_weight: Uint64,
    to_balance: Uint128,
    amplification: Uint64
) -> ExpectedReceiveAssetResult {

    // Convert arguments into float
    let u = u256_to_f64(u) / 1e18;
    let to_weight = to_weight.u64() as f64;
    let to_balance = to_balance.u128() as f64;
    let amplification = (amplification.u64() as f64) / 1e18;

    // Compute swap
    let one_minus_amplification = 1. - amplification;
    let weighted_to_balance = to_weight * to_balance;
    let weighted_to_balance_ampped = weighted_to_balance.powf(one_minus_amplification);

    let to_amount = to_balance * (
        1. - ((weighted_to_balance_ampped - u)/weighted_to_balance_ampped).powf(1./one_minus_amplification)
    );

    ExpectedReceiveAssetResult {
        to_amount
    }
    
}


pub fn compute_expected_send_liquidity(
    swap_amount: Uint128,
    from_weights: Vec<Uint64>,
    from_balances: Vec<Uint128>,
    from_total_supply: Uint128,
    from_unit_tracker: I256,
    amplification: Uint64
) -> ExpectedSendLiquidityResult {

    let asset_count = from_balances.len() as f64;

    // Compute from vault balance0
    let from_balance_0 = compute_balance_0(from_weights, from_balances, from_unit_tracker, amplification);

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_total_supply = from_total_supply.u128() as f64;
    let amplification = (amplification.u64() as f64) / 1e18;
    let one_minus_amplification = 1. - amplification;
    
    // Compute swap
    let u = asset_count * (
        (
            from_balance_0 + (from_balance_0 * swap_amount)/from_total_supply
        ).powf(one_minus_amplification)
        - (
            from_balance_0
        ).powf(one_minus_amplification)
    );


    ExpectedSendLiquidityResult {
        u
    }

}

pub fn compute_expected_receive_liquidity(
    u: U256,
    to_weights: Vec<Uint64>,
    to_balances: Vec<Uint128>,
    to_total_supply: Uint128,
    to_unit_tracker: I256,
    amplification: Uint64
) -> ExpectedReceiveLiquidityResult {

    let asset_count = to_balances.len() as f64;

    // Compute from vault balance0
    let to_balance_0 = compute_balance_0(to_weights, to_balances, to_unit_tracker, amplification);

    // Convert arguments to float
    let u = u256_to_f64(u) / 1e18;
    let to_total_supply = to_total_supply.u128() as f64;
    let amplification = (amplification.u64() as f64) / 1e18;
    let one_minus_amplification = 1. - amplification;
    
    // Compute swap
    let weighted_vault_tokens = (
        to_balance_0.powf(one_minus_amplification) + u/asset_count
    ).powf(1./one_minus_amplification) - to_balance_0;

    let to_amount = weighted_vault_tokens * to_total_supply / to_balance_0;

    return ExpectedReceiveLiquidityResult {
        to_amount
    }

}

pub fn compute_expected_reference_asset(
    vault_tokens: Uint128,
    vault_balances: Vec<Uint128>,
    vault_weights: Vec<Uint64>,
    vault_total_supply: Uint128,
    vault_unit_tracker: I256,
    vault_escrowed_vault_tokens: Uint128,
    amplification: Uint64
) -> ExpectedReferenceAsset {

    // Compute vault balance0
    let balance_0 = compute_balance_0(
        vault_weights,
        vault_balances,
        vault_unit_tracker,
        amplification
    );

    // Convert arguments to float
    let vault_tokens = uint128_to_f64(vault_tokens);
    let vault_total_supply = uint128_to_f64(vault_total_supply);
    let vault_escrowed_vault_tokens = uint128_to_f64(vault_escrowed_vault_tokens);

    let user_reference_amount = (balance_0 * vault_tokens) / (vault_total_supply + vault_escrowed_vault_tokens + vault_tokens);

    ExpectedReferenceAsset {
        amount: user_reference_amount
    }
}


pub fn compute_expected_deposit_mixed(
    deposit_amounts: Vec<Uint128>,
    vault_weights: Vec<Uint64>,
    vault_balances: Vec<Uint128>,
    vault_total_supply: Uint128,
    vault_unit_tracker: I256,
    vault_fee: Option<Uint64>,
    amplification: Uint64
) -> f64 {

    let asset_count = vault_balances.len() as f64;

    // Compute vault balance0
    let balance_0 = compute_balance_0(
        vault_weights.clone(),
        vault_balances.clone(),
        vault_unit_tracker,
        amplification
    );

    // Convert arguments to float
    let vault_total_supply = vault_total_supply.u128() as f64;
    let amplification = (amplification.u64() as f64) / 1e18;
    let one_minus_amplification = 1. - amplification;
    
    // Compute units
    let units: f64 = deposit_amounts.iter()
        .zip(&vault_weights)
        .zip(&vault_balances)
        .map(|((deposit_amount, vault_weight), vault_balance)| {

            let deposit_amount = uint128_to_f64(*deposit_amount);
            let vault_weight = vault_weight.u64() as f64;
            let vault_balance = uint128_to_f64(*vault_balance);

            let weighted_balance = vault_weight * vault_balance;
            let weighted_deposit = vault_weight * deposit_amount;

            (weighted_balance + weighted_deposit).powf(one_minus_amplification)
                - weighted_balance.powf(one_minus_amplification)

        })
        .sum();

    // Take the vault fee
    let units = units * (1. - (vault_fee.unwrap_or(Uint64::zero()).u64() as f64)/1e18);

    // Compute the deposit share
    let balance_0_ampped = balance_0.powf(one_minus_amplification);

    vault_total_supply * (
        (
            1. + units/(asset_count * balance_0_ampped)
        ).powf(1./one_minus_amplification) - 1.
    )

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


pub fn compute_invariant(
    vault_weights: Vec<Uint64>,
    vault_balances: Vec<Uint128>,
    amplification: Uint64
) -> f64 {
    
    let amplification = (amplification.u64() as f64) / 1e18;

    let one_minus_amplification = 1. - amplification;

    vault_weights
        .iter()
        .zip(vault_balances)
        .fold(0., |acc, (weight, balance)| -> f64 {
            acc + ((weight.u64() as f64) * uint128_to_f64(balance)).powf(one_minus_amplification)
        })
}


pub fn compute_balance_0(
    vault_weights: Vec<Uint64>,
    vault_balances: Vec<Uint128>,
    unit_tracker: I256,
    amplification: Uint64
) -> f64 {
    
    let asset_count = vault_balances.len() as f64;
    let invariant = compute_invariant(vault_weights, vault_balances, amplification);

    let unit_tracker = i256_to_f64(unit_tracker) / 1e18;
    let amplification = (amplification.u64() as f64) / 1e18;

    let weighted_alpha_ampped = (invariant - unit_tracker) / asset_count;

    weighted_alpha_ampped.powf(1./(1.-amplification))
}
