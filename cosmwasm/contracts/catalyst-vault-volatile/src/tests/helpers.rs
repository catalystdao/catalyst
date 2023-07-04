use cosmwasm_std::{Uint128, Uint64};
use cw_multi_test::{ContractWrapper, App};
use catalyst_types::U256;
use test_helpers::{math::{u256_to_f64, uint128_to_f64}, contract::{ExpectedLocalSwapResult, ExpectedSendAssetResult, ExpectedReceiveAssetResult, ExpectedSendLiquidityResult, ExpectedReceiveLiquidityResult, ExpectedReferenceAsset}};



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



// Swap Utils

pub fn compute_expected_local_swap(
    swap_amount: Uint128,
    from_weight: Uint128,
    from_balance: Uint128,
    to_weight: Uint128,
    to_balance: Uint128,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedLocalSwapResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight.u128() as f64;
    let from_balance = from_balance.u128() as f64;
    let to_weight = to_weight.u128() as f64;
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
    from_weight: Uint128,
    from_balance: Uint128,
    vault_fee: Option<Uint64>,
    governance_fee_share: Option<Uint64>
) -> ExpectedSendAssetResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_weight = from_weight.u128() as f64;
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
    to_weight: Uint128,
    to_balance: Uint128
) -> ExpectedReceiveAssetResult {

    // Convert arguments into float
    let u = u256_to_f64(u) / 1e18;
    let to_weight = to_weight.u128() as f64;
    let to_balance = to_balance.u128() as f64;

    // Compute swap
    ExpectedReceiveAssetResult {
        to_amount: to_balance * (1. - (-u/to_weight).exp())
    }
    
}


pub fn compute_expected_send_liquidity(
    swap_amount: Uint128,
    from_weights: Vec<Uint128>,
    from_total_supply: Uint128
) -> ExpectedSendLiquidityResult {

    // Convert arguments to float
    let swap_amount = swap_amount.u128() as f64;
    let from_total_supply = from_total_supply.u128() as f64;

    // Compute swap
    let from_weights_sum: f64 = from_weights.iter().sum::<Uint128>().u128() as f64;
    let u = (from_total_supply/(from_total_supply - swap_amount)).ln() * from_weights_sum;

    ExpectedSendLiquidityResult {
        u
    }

}

pub fn compute_expected_receive_liquidity(
    u: U256,
    to_weights: Vec<Uint128>,
    to_total_supply: Uint128
) -> ExpectedReceiveLiquidityResult {

    // Convert arguments to float
    let u = u256_to_f64(u) / 1e18;
    let to_total_supply = to_total_supply.u128() as f64;

    // Compute swap
    let to_weights_sum: f64 = to_weights.iter().sum::<Uint128>().u128() as f64;
    let share = 1. - (-u/to_weights_sum).exp();
    let to_amount = to_total_supply * (share/(1.-share));

    ExpectedReceiveLiquidityResult {
        to_amount
    }

}

pub fn compute_expected_reference_asset(
    vault_tokens: Uint128,
    vault_balances: Vec<Uint128>,
    vault_weights: Vec<Uint128>,
    vault_total_supply: Uint128,
    vault_escrowed_vault_tokens: Uint128
) -> ExpectedReferenceAsset {

    let weights_sum = vault_weights.iter().sum::<Uint128>().u128() as f64;

    let vault_reference_amount: f64 = vault_balances.iter()
        .zip(vault_weights)
        .map(|(balance, weight)| {

            let balance = uint128_to_f64(*balance);
            let weight = weight.u128() as f64;

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
    from_weights: Vec<Uint128>,
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
            let from_weight = from_weight.u128() as f64;
            let from_balance = uint128_to_f64(*from_balance);

            from_weight * (1. + deposit_amount/from_balance).ln()
        })
        .sum();

    // Take vault fee
    let units = units * (1. - (vault_fee.unwrap_or(Uint64::zero()).u64() as f64)/1e18);

    // Compute the deposit share
    let weights_sum = from_weights.iter().sum::<Uint128>().u128() as f64;
    let from_total_supply = uint128_to_f64(from_total_supply);

    let deposit_share = (units / weights_sum).exp() - 1.;

    from_total_supply * deposit_share

}


pub fn compute_expected_withdraw_mixed(
    withdraw_amount: Uint128,
    withdraw_ratio: Vec<Uint64>,
    vault_weights: Vec<Uint128>,
    vault_balances: Vec<Uint128>,
    vault_supply: Uint128
) -> Vec<f64> {

    // Compute the units corresponding to the vault tokens
    let withdraw_amount = uint128_to_f64(withdraw_amount);
    let vault_supply = uint128_to_f64(vault_supply);

    let vault_weights_sum = vault_weights.iter().sum::<Uint128>().u128() as f64;

    let mut units: f64 = (
        vault_supply/(vault_supply - withdraw_amount)
    ).ln() * vault_weights_sum;

    vault_balances.iter()
        .zip(vault_weights)
        .zip(withdraw_ratio)
        .map(|((balance, weight), ratio)| {

            let balance = uint128_to_f64(*balance);
            let weight = weight.u128() as f64;
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

