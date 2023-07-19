use catalyst_types::U256;
use catalyst_vault_common::ContractError;
use fixed_point_math::{ln_wad, div_wad_down, mul_wad_down, WAD, exp_wad};


// Integral Helpers *************************************************************************************************************

/// Compute the price curve integral: $\int_{a}^{a+input} w/x dx = w·ln((a+x)/a)$
/// 
/// Returns the units that the provided assets are worth (in WAD notation).
/// 
/// # Arguments:
/// * `input` - The input amount provided by the user.
/// * `a` - The vault's asset balance.
/// * `w` - The vault's asset weight.
/// 
pub fn calc_price_curve_area(
    input: U256,
    a: U256,
    w: U256
) -> Result<U256, ContractError> {
    // Compute w·ln((a+x)/a)
    w.checked_mul(
        ln_wad(
            div_wad_down(
                a.checked_add(input)?,
                a
            )?.as_i256()    // If casting overflows to a negative number 'ln_wad' will fail
        )?.as_u256()        // Casting is safe as 'ln_wad' result is always positive (note that its argument is >= 1)
    ).map_err(|err| err.into())
}


/// Solve the limit of the price curve integral: $u = \int_{b-y}^{b} w/x dx => y = b·(1-exp(-u/w))$
/// 
/// Returns the output asset amount for the given units.
/// 
/// # Arguments:
/// * `u` - The incoming 'units'.
/// * `b` - The vault's asset balance.
/// * `w` - The vault's asset weight.
/// 
pub fn calc_price_curve_limit(
    u: U256,
    b: U256,
    w: U256
) -> Result<U256, ContractError> {
    // Compute b·(1-exp(-u/w))
    mul_wad_down(
        b,
        WAD.checked_sub(
            exp_wad(
                // If the casting to i256 overflows to a negative value:
                //   - If the casting overflows exactly by 1 (i.e. the result is exactly i256::MIN), when the 
                //     value is negated it again overflows and remains unchanged (i.e. -i256::MIN = i256::MIN),
                //     This is not a problem, as it is exactly what it is desired.
                //   - Otherwise, the value becomes positive when it gets negated. This will cause the result 
                //     of the following exponent calculation to be greater than one, which will cause the 
                //     following 'checked_sub' operation to fail.
                (u.checked_div(w)?)
                    .as_i256()
                    .wrapping_neg()
            )?.as_u256()            // Casting is safe, as 'exp_wad' result is always positive.
        )?
    ).map_err(|err| err.into())
}


/// Solve the combined price curve equations. To reduce attack vectors, this is implemented
/// using the individual equations, and is not simplified into a reduced form.
/// 
/// Returns the output asset amount for the given input asset amount.
/// 
/// # Arguments:
/// * `input` - The input amount provided by the user.
/// * `a` - The vault's input asset balance.
/// * `b` - The vault's output asset balance.
/// * `w_a` - The vault's input asset weight.
/// * `w_b` - The vault's output asset weight.
/// 
pub fn calc_combined_price_curves(
    input: U256,
    a: U256,
    b: U256,
    w_a: U256,
    w_b: U256
) -> Result<U256, ContractError> {
    calc_price_curve_limit(
        calc_price_curve_area(input, a, w_a)?,
        b,
        w_b
    )
}


/// Solve the 'liquidity-to-units' equation.
/// 
/// Returns the vault's share of the provided units (in WAD notation).
/// 
/// # Arguments:
/// * `u` - The incoming 'units'.
/// * `w_sum` - The vault's weights sum.
/// 
pub fn calc_price_curve_limit_share(
    u: U256,
    w_sum: U256
) -> Result<U256, ContractError> {
    // Compute 1 - vault_ownership = exp(-u/w_sum)
    let non_vault_ownership = exp_wad(
        // If the casting to i256 overflows to a negative value:
        //   - If the casting overflows exactly by 1 (i.e. the result is exactly i256::MIN), when the 
        //     value is negated it again overflows and remains unchanged (i.e. -i256::MIN = i256::MIN),
        //     This is not a problem, as it is exactly what it is desired.
        //   - Otherwise, the value becomes positive when it gets negated. This will cause the result 
        //     of the following exponent calculation to be greater than one, which will cause the 
        //     'checked_sub' operation inside the following 'div_wad_down' to fail.
        (u.checked_div(w_sum)?)
            .as_i256()
            .wrapping_neg()
    )?.as_u256();    // Casting is safe, as 'exp_wad' result is always positive.

    // Return the vault ownership share *before* the share is included in the vault.
    div_wad_down(
        WAD.checked_sub(non_vault_ownership)?,
        non_vault_ownership
    ).map_err(|err| err.into())
}
