use std::ops::Div;
use catalyst_types::{U256, AsI256, AsU256, I256};
use catalyst_vault_common::ContractError;
use fixed_point_math::{div_wad_down, mul_wad_down, WAD, pow_wad, div_wad_up, WADWAD};


// TODO use Uint128 where suitable instead of U256?
// TODO     => This would allow removing of some of the 'checked' functions, as it would be possible to make assumptions 
// TODO        on the size of the input variables (e.g. adding to U256 when they have been casted from Uint128)
// TODO add overflow safety comments
// Integral Helpers *************************************************************************************************************

/// Compute the integral \int_{w a}^{w (a + input)} 1 / (x^k) · (1-k) dx
///     = (w a + w input)^(1-k) - (w a)^(1-k) 
/// NOTE: This function will revert for w == 0 or (a + input) == 0
pub fn calc_price_curve_area(
    input: U256,
    a: U256,
    w: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {

    // Compute calc = (w a + w input)^(1-k)
    let mut calc = pow_wad(
        w.checked_mul(a.checked_add(input)?)?
            .checked_mul(WAD)?
            .as_i256(), // If casting overflows to a negative number 'pow_wad' will fail
        one_minus_amp
    )?;

    // Compute calc - (w a)^(1-k) 
    // The calculation will fail for a vault balance of 0. Skip this case, as
    // the result of the calculation will also be 0.
    if !a.is_zero() {
        calc = calc.wrapping_sub(
            pow_wad(
                w.checked_mul(a)?.checked_mul(WAD)?.as_i256(), // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp
            )?
        );
    }

    Ok(calc.as_u256())      // Casting always casts a positive number
}


/// Solve the equation u = \int_{w (a - y)}^{w a} 1 / (x^k) · (1-k) dx
///     = b [ 1 - (
///             ((w b) ^ (1-k) - u) / ((w b) ^ (1-k))
///         )^( 1 / (1-k) )
///     ]
pub fn calc_price_curve_limit(
    u: U256,
    b: U256,
    w: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {
    
    // Compute (w b)^(1-k)
    let weighted_balance_ampped = pow_wad(
        w.checked_mul(b)?.checked_mul(WAD)?.as_i256(),  // If casting overflows to a negative number 'pow_wad' will fail
        one_minus_amp
    )?.as_u256();                                       // Casting always casts a positive number

    // Compute b [1 - ( (wbampped - u) / wbampped )^( 1/(1-k) )]
    mul_wad_down(
        b,
        WAD.wrapping_sub(
            pow_wad(
                div_wad_up(
                    weighted_balance_ampped.checked_sub(u)?,
                    weighted_balance_ampped
                )?.as_i256(),                           // Casting never overflows, as the division result is always <= 1
                WADWAD.div(one_minus_amp)
            )?.as_u256()                                // Casting always casts a positive number
        )
    ).map_err(|err| err.into())
}


/// Solve the combined price equation. To reduce attack vectors, this is done with 
/// the individual 'calc_price_curve_area' and 'calc_price_curve_limit' rather than with 
/// the full simplified equation.
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


/// Solve the 'liquidity to units' equation.    //TODO write equation as in the functions above
pub fn calc_price_curve_limit_share(
    u: U256,
    ts: U256,
    n_weighted_balance_ampped: U256,
    one_minus_amp_inverse: I256
) -> Result<U256, ContractError> {
    mul_wad_down(
        ts,
        (
            pow_wad(
                div_wad_down(
                    n_weighted_balance_ampped.checked_add(u)?,
                    n_weighted_balance_ampped
                )?.as_i256(),                   // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp_inverse
            )?.wrapping_sub(WAD.as_i256())      // Subtraction is underflow safe, as the result of 'pow_wad' is 
                                                // always >= 1 (since the 'base' of the power is also >= 1)
        ).as_u256()                             // Casting always casts a positive number
    ).map_err(|err| err.into())
}
