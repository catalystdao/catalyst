use std::ops::Div;

use catalyst_types::{U256, AsI256, AsU256, I256};
use catalyst_vault_common::ContractError;
use fixed_point_math::{ln_wad, div_wad_down, mul_wad_down, WAD, exp_wad, pow_wad, div_wad_up, WADWAD};


// TODO use Uint128 where suitable instead of U256?
// TODO     => This would allow removing of some of the 'checked' functions, as it would be possible to make assumptions 
// TODO        on the size of the input variables (e.g. adding to U256 when they have been casted from Uint128)
// TODO add overflow safety comments
// Integral Helpers *************************************************************************************************************

pub fn calc_price_curve_area(
    input: U256,
    a: U256,
    w: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {

    let mut calc = pow_wad(
        w.checked_mul(a.checked_add(input)?)?.checked_mul(WAD)?.as_i256(),
        one_minus_amp
    )?;

    if !a.is_zero() {
        calc = calc.wrapping_sub(
            pow_wad(
                w.checked_mul(a)?.checked_mul(WAD)?.as_i256(),
                one_minus_amp
            )?
        );
    }

    Ok(calc.as_u256())
}


pub fn calc_price_curve_limit(
    u: U256,
    b: U256,
    w: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {
    
    let weighted_balance_ampped = pow_wad(
        w.checked_mul(b)?.checked_mul(WAD)?.as_i256(),
        one_minus_amp
    )?.as_u256();

    mul_wad_down(
        b,
        WAD.wrapping_sub(
            pow_wad(
                div_wad_up(
                    weighted_balance_ampped.checked_sub(u)?,
                    weighted_balance_ampped
                )?.as_i256(),
                WADWAD.div(one_minus_amp)
            )?.as_u256()
        )
    ).map_err(|err| err.into())
}


pub fn calc_combined_price_curves(
    input: U256,
    a: U256,
    b: U256,
    w_a: U256,
    w_b: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {
    calc_price_curve_limit(
        calc_price_curve_area(input, a, w_a, one_minus_amp)?,
        b,
        w_b,
        one_minus_amp
    )
}


pub fn calc_price_curve_limit_share(
    u: U256,
    w_sum: U256
) -> Result<U256, ContractError> {
    todo!()
}
