use std::ops::{Shr, Shl};

use anchor_lang::prelude::*;
use fixed_point_math_lib::u256::U256;
use fixed_point_math_lib::fixed_point_math_x64::{div_x64, pow_x64, inv_pow_x64};

const ONE_X64: U256 = U256([0, 1, 0, 0]);
const ONE_X128: U256 = U256([0, 0, 1, 0]);



// Deposits and Withdrawals *****************************************************************************************************

pub fn calc_asset_amount_for_pool_tokens(pool_token_balance: u64, asset_balance: u64, asset_eq_balance: u64) -> Result<u64> {
    if asset_eq_balance == asset_balance {
        return Ok(pool_token_balance)
    }

    Ok(asset_balance.checked_mul(pool_token_balance).unwrap().checked_div(asset_eq_balance).unwrap())
}



// Asset swaps ******************************************************************************************************************

pub fn out_swap_x64(
    input: U256,                    // x
    source_asset_balance: U256,     // At
    source_asset_weight: U256,      // WA
    amplification_x64: U256         // k
) -> Result<U256> {
    // Computes the integral
    // int_{At}^{At+x} WA/w dw

    let input_x64 = input.shl(64u8);
    let source_asset_balance_x64 = source_asset_balance.shl(64u8);

    let one_minus_amp_x64 = ONE_X64.checked_sub(amplification_x64).unwrap();
    
    Ok(source_asset_weight.checked_mul(
        pow_x64(
            source_asset_balance_x64.checked_add(input_x64).unwrap(), 
            one_minus_amp_x64
        ).unwrap().checked_sub(
            pow_x64(source_asset_balance_x64, one_minus_amp_x64).unwrap()
        ).unwrap()
    ).unwrap())

}


pub fn in_swap(
    units_x64: U256,                // U
    target_asset_balance: U256,     // Bt
    target_asset_weight: U256,      // WB
    amplification_x64: U256         // k
) -> Result<U256> {
    // Solves the following integral for 'y'
    // int_{Bt-y}^{Bt} WB/w dW

    let one_minus_amp_x64 = ONE_X64.checked_sub(amplification_x64).unwrap();

    let intermediate_x64 = target_asset_weight.checked_mul(
        pow_x64(target_asset_balance.shl(64u8), one_minus_amp_x64).unwrap()
    ).unwrap();

    Ok(target_asset_balance.checked_mul(
        ONE_X64.checked_sub(inv_pow_x64(
            div_x64(intermediate_x64, intermediate_x64.checked_sub(units_x64).unwrap()).unwrap(),
            div_x64(ONE_X64, one_minus_amp_x64).unwrap()
        ).unwrap()).unwrap()
    ).unwrap().shr(64u8))
}

pub fn full_swap(
    input: U256,
    source_asset_balance: U256,
    source_asset_weight: U256,
    target_asset_balance: U256,
    target_asset_weight: U256,
    amplification_x64: U256
) -> Result<U256> {

    let input_x64 = input.shl(64u8);
    let source_asset_balance_x64 = source_asset_balance.shl(64u8);

    let one_minus_amp_x64 = ONE_X64 - amplification_x64;

    let intermediate_x64 = target_asset_weight.checked_mul(
        pow_x64(target_asset_balance.shl(64u8), one_minus_amp_x64).unwrap()
    ).unwrap();
    
    Ok(target_asset_balance.checked_mul(
        ONE_X64.checked_sub(inv_pow_x64(
            div_x64(
                intermediate_x64,
                intermediate_x64.checked_sub(source_asset_weight.checked_mul(
                    pow_x64(
                        source_asset_balance_x64.checked_add(input_x64).unwrap(), 
                        one_minus_amp_x64
                    ).unwrap().checked_sub(
                        pow_x64(source_asset_balance_x64, one_minus_amp_x64).unwrap()
                    ).unwrap()
                ).unwrap()).unwrap()
            ).unwrap(),
            div_x64(ONE_X64, one_minus_amp_x64).unwrap()
        ).unwrap()).unwrap()
    ).unwrap().shr(64u8))
}


#[error_code]
pub enum IntegralCalculationErrorCode {
    #[msg("Arithmetic Error. Possible overflow/underflow.")]
    ArithmeticError,
}



// Liquidity swaps **************************************************************************************************************

pub fn calc_out_liquidity_swap_x64(
    input_liquidity: U256,          // x
    source_asset_eq_balance: U256,  // A0
    source_asset_weight: U256,      // WA
    amplification_x64: U256         // k
) -> Result<U256> {
    // Computes the integral
    // int_{At}^{At+x} WA/w dw

    let input_liquidity_x64 = input_liquidity.shl(64u8);                    // Safe, as input_liquidity comes from a u64 number
    let source_asset_eq_balance_x64 = source_asset_eq_balance.shl(64u8);    // Safe, as source_asset_eq_balance comes from a u64 number

    let one_minus_amp_x64 = ONE_X64.checked_sub(amplification_x64).unwrap();
    
    Ok(source_asset_weight.checked_mul(
        pow_x64(
            source_asset_eq_balance_x64, 
            one_minus_amp_x64
        ).unwrap().checked_sub(
            pow_x64(
                source_asset_eq_balance_x64.checked_sub(input_liquidity_x64).unwrap(), 
                one_minus_amp_x64
            ).unwrap()
        ).unwrap()
    ).unwrap())

}


pub fn calc_in_liquidity_swap(
    liquidity_units_x64: U256,           // U
    target_asset_eq_balance: U256,       // B0
    target_assets_aggr_weight_x64: U256, // W_SUM
    amplification_x64: U256              // k
) -> Result<U256> {
    // Solves the following integral for 'y'
    // int_{Bt-y}^{Bt} W_SUM/w dW

    Ok(target_asset_eq_balance.checked_mul(
        pow_x64(
            div_x64(
                target_assets_aggr_weight_x64.checked_add(liquidity_units_x64).unwrap(),
                target_assets_aggr_weight_x64
            ).unwrap(),
            div_x64(
                ONE_X64, 
                ONE_X64.checked_sub(amplification_x64).unwrap()
            ).unwrap()
        ).unwrap().checked_sub(ONE_X64).unwrap()
    ).unwrap().shr(64u8))
}
