use std::ops::{Shr, Shl};

use cosmwasm_std::{StdResult, Uint128};
use ethnum::U256;



// Deposits and Withdrawals *****************************************************************************************************

pub fn calc_asset_amount_for_pool_tokens(pool_token_balance: Uint128, asset_balance: Uint128, asset_eq_balance: Uint128) -> StdResult<Uint128> {
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
    approx: bool
) -> StdResult<U256> {
    // Computes the integral
    // int_{At}^{At+x} WA/w dw

    // if approx {
    //     return Ok(
    //         div_x64(
    //             source_asset_weight.checked_mul(input).unwrap().shl(64),
    //             source_asset_balance.checked_add(input).unwrap().checked_mul(LN2_X64).unwrap()
    //         ).unwrap()
    //     )
    // }

    // // WA * ln((At + x)/At)
    // Ok(source_asset_weight.checked_mul(
    //     log2_x64(
    //         div_x64(source_asset_balance.checked_add(input).unwrap(), source_asset_balance).unwrap()
    //     ).unwrap()
    // ).unwrap())

    unimplemented!()
}


pub fn in_swap(
    units_x64: U256,                // U
    target_asset_balance: U256,     // Bt
    target_asset_weight: U256,      // WB
    approx: bool
) -> StdResult<U256> {
    // Solves the following integral for 'y'
    // int_{Bt-y}^{Bt} WB/w dW

    // if approx {
    //     let units_times_ln2_x64 = mul_x64(units_x64, LN2_X64).unwrap();

    //     return Ok(
    //         div_x64(
    //             target_asset_balance.checked_mul(units_times_ln2_x64).unwrap(),
    //             target_asset_weight.shl(64).checked_add(units_times_ln2_x64).unwrap()
    //         ).unwrap()
    //     )
    // }

    // // Bt * (1 - exp(-U/WB))
    // Ok(target_asset_balance.checked_mul(
    //     ONE_X64.checked_sub(
    //         inv_pow2_x64(units_x64.checked_div(target_asset_weight).unwrap()).unwrap()
    //     ).unwrap()
    // ).unwrap().shr(64u8))

    unimplemented!()
}

pub fn full_swap(
    input: U256,
    source_asset_balance: U256,
    source_asset_weight: U256,
    target_asset_balance: U256,
    target_asset_weight: U256,
    approx: bool
) -> StdResult<U256> {

    // Bt * (1 - (At + input) / At) ^ (-WA/WB))       NOTE: (At + input) / At >= 1 as input > 0

    // if source_asset_weight == target_asset_weight {
    //     return Ok(target_asset_balance.checked_mul(input).unwrap().checked_div(source_asset_balance.checked_add(input).unwrap()).unwrap())
    // }

    // if approx {
    //     return Ok(
    //         target_asset_balance.checked_mul(source_asset_weight).unwrap().checked_mul(input).unwrap().checked_div(
    //             target_asset_weight.checked_mul(source_asset_balance).unwrap().checked_add(
    //                 source_asset_weight.checked_add(target_asset_weight).unwrap().checked_mul(input).unwrap()
    //             ).unwrap()
    //         ).unwrap()
    //     )
    // }

    // Ok(target_asset_balance.checked_mul( 
    //     ONE_X64.checked_sub(
    //         inv_pow_x64(
    //             div_x64(source_asset_balance.checked_add(input).unwrap(), source_asset_balance).unwrap(),
    //             div_x64(
    //                 source_asset_weight,
    //                 target_asset_weight
    //             ).unwrap()
    //         ).unwrap()
    //     ).unwrap()
    // ).unwrap().shr(64u8))

    unimplemented!()
}


// Liquidity swaps **************************************************************************************************************

pub fn calc_out_liquidity_swap_x64(
    input_liquidity: U256,          // x
    source_asset_eq_balance: U256,  // A0
    source_asset_weight: U256       // WA
) -> StdResult<U256> {
    // Computes the integral
    // int_{At}^{At+x} WA/w dw

    // WA * ln((At + x)/At)
    // Ok(source_asset_weight.checked_mul(
    //     log2_x64(
    //         div_x64(source_asset_eq_balance, source_asset_eq_balance.checked_sub(input_liquidity).unwrap()).unwrap()
    //     ).unwrap()
    // ).unwrap())

    unimplemented!()
}


pub fn calc_in_liquidity_swap(
    liquidity_units_x64: U256,       // U
    target_asset_eq_balance: U256,   // B0
    target_assets_aggr_weight: U256  // W_SUM
) -> StdResult<U256> {
    // Solves the following integral for 'y'
    // int_{Bt-y}^{Bt} W_SUM/w dW

    // Bt * (2^(U/W_SUM)-1)
    // Ok(target_asset_eq_balance.checked_mul(
    //     pow2_x64(
    //         liquidity_units_x64.checked_div(target_assets_aggr_weight).unwrap()     // Weight is an integer for this case, can safely >> 64
    //     ).unwrap().checked_sub(ONE_X64).unwrap()
    // ).unwrap().shr(64u8))

    unimplemented!()
}
