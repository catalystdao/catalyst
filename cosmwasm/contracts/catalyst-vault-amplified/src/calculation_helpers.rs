use std::ops::Div;
use catalyst_types::{U256, AsI256, AsU256, I256};
use catalyst_vault_common::ContractError;
use cosmwasm_std::Uint128;
use fixed_point_math::{div_wad_down, mul_wad_down, WAD, pow_wad, div_wad_up, WADWAD};


// Integral Helpers *************************************************************************************************************

/// Compute the price curve integral \int_{w·a}^{w·(a + input)} 1/(x^k) · (1-k) dx
///     = (w·(a + input))^(1-k) - (w·a)^(1-k) 
/// 
/// Returns the units that the provided assets are worth (in WAD notation).
/// 
/// **NOTE**: This function will revert for w == 0 or (a + input) == 0
/// 
/// # Arguments:
/// * `input` - The input amount provided by the user.
/// * `a` - The vault's asset balance.
/// * `w` - The vault's asset weight.
/// * `one_minus_amp` - One minus the vault's amplification (in WAD notation).
/// 
pub fn calc_price_curve_area(
    input: U256,
    a: U256,
    w: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {

    // Compute calc = (w·(a + input))^(1-k)
    let mut calc = pow_wad(
        w.checked_mul(a.checked_add(input)?)?
            .checked_mul(WAD)?
            .as_i256(), // If casting overflows to a negative number 'pow_wad' will fail
        one_minus_amp
    )?;

    // Compute 'calc' - (w·a)^(1-k) 
    // The calculation will fail for a vault balance of 0. Skip this case, as
    // the result of the calculation will also be 0.
    if !a.is_zero() {
        calc = calc.wrapping_sub(   // 'wrapping_sub' is safe as (w·a)^(1-k) < (w·(a + input))^(1-k), i.e. 'calc'
            pow_wad(
                w
                    .wrapping_mul(a)     // 'wrapping_mul' is safe, as otherwise the 'calc' calculation above would have failed
                    .wrapping_mul(WAD)   // 'wrapping_mul' is safe, as otherwise the 'calc' calculation above would have failed
                    .as_i256(),          // If casting overflows to a negative number 'pow_wad' will fail
                one_minus_amp
            )?
        );
    }

    Ok(calc.as_u256())      // Casting always casts a positive number
}


/// Solve the limit of the price curve integral:
/// u = \int_{w·(b - y)}^{w·b} 1/(x^k) · (1-k) dx
///     = b \[ 1 - (
///             ((w·b)^(1-k) - u) / ((w·b)^(1-k))
///         )^( 1/(1-k) )
///     \]
/// 
/// Returns the output asset amount for the given units.
/// 
/// # Arguments:
/// * `u` - The incoming 'units'.
/// * `b` - The vault's asset balance.
/// * `w` - The vault's asset weight.
/// * `one_minus_amp` - One minus the vault's amplification (in WAD notation).
/// 
pub fn calc_price_curve_limit(
    u: U256,
    b: U256,
    w: U256,
    one_minus_amp: I256
) -> Result<U256, ContractError> {
    
    // Compute (w·b)^(1-k)
    let weighted_balance_ampped = pow_wad(
        w.checked_mul(b)?.checked_mul(WAD)?.as_i256(),  // If casting overflows to a negative number 'pow_wad' will fail
        one_minus_amp
    )?.as_u256();                                       // Casting always casts a positive number

    // Compute b·[1 - ((wbampped - u) / wbampped)^(1/(1-k))]
    mul_wad_down(
        b,
        WAD.checked_sub(   // 'checked_sub' used for extra precaution ('wrapping_sub' should be sufficient).
            pow_wad(
                div_wad_up(
                    weighted_balance_ampped.checked_sub(u)?,
                    weighted_balance_ampped
                )?.as_i256(),                           // Casting never overflows, as the division result is always <= 1
                WADWAD.div(one_minus_amp)
            )?.as_u256()                                // Casting always casts a positive number
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
/// * `one_minus_amp` - One minus the vault's amplification (in WAD notation).
/// 
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


/// Solve the 'liquidity-to-units' equation.
/// 
/// Returns the vault's share of the provided units (in WAD notation).
/// 
/// # Arguments:
/// * `u` - The incoming 'units'.
/// * `total_supply` - The vault's vault token supply.
/// * `n_weighted_balance_ampped` - (w·a_0)^(1-k)
/// * `one_minus_amp_inverse` - The inverse of one minus the vault's amplification (in WAD notation).
/// 
pub fn calc_price_curve_limit_share(
    u: U256,
    total_supply: U256,
    n_weighted_balance_ampped: U256,
    one_minus_amp_inverse: I256
) -> Result<U256, ContractError> {
    mul_wad_down(
        total_supply,
        pow_wad(
            div_wad_down(
                n_weighted_balance_ampped.checked_add(u)?,
                n_weighted_balance_ampped
            )?.as_i256(),                   // If casting overflows to a negative number 'pow_wad' will fail
            one_minus_amp_inverse
        )?.as_u256()                        // Casting always casts a positive number
            .checked_sub(WAD)?    // 'checked_sub' used for extra precaution ('wrapping_sub' should be sufficient)
    ).map_err(|err| err.into())
}



// Invariant Helpers **************************************************************************************************

/// Compute balance0^(1-k) via (w·a_0)^(1-k) in WAD notation.
/// 
/// ! **IMPORTANT!**: All the vectors passed to this function must be of equal length!
/// 
/// **NOTE-DEV**: When computing the balance0, the true vault balances should always be used
/// (i.e. not modified by the escrow balances).
/// 
/// **NOTE-DEV**: This function is called `_computeBalance0` on the EVM implementation.
/// 
/// # Arguments:
/// * `weights` - The vault's asset weights.
/// * `vault_balances` - The current vault asset balances (**not** modified by the escrowed amounts!)
/// * `one_minus_amp` - One minus the vault's amplification (in WAD notation).
/// * `unit_tracker` - The current state of the `unit_tracker` (in WAD notation).
/// * 
pub fn calc_weighted_alpha_0_ampped(
    weights: Vec<Uint128>,
    vault_balances: Vec<Uint128>,
    one_minus_amp: I256,
    unit_tracker: I256
) -> Result<U256, ContractError> {

    // Compute the sum of (w·asset_balance)^(1-k) for every asset in the vault
    let weighted_asset_balance_ampped_sum: I256 = weights.iter()
        .zip(vault_balances)    // ! The caller of this function should make sure
                                // ! that vault_balances.len() is equal to weights.len()
        .try_fold(I256::zero(), |acc, (weight, vault_asset_balance)| -> Result<I256, ContractError> {
            
            if !vault_asset_balance.is_zero() {

                let weighted_asset_balance = U256::from(vault_asset_balance)
                    .wrapping_mul((*weight).into());           // 'wrapping_mul' is safe as U256.max > Uint128.max * Uint128.max
    
                let weighted_asset_balance_ampped = pow_wad(
                    weighted_asset_balance.checked_mul(WAD)?.as_i256(), // If casting overflows to a negative number 'pow_wad' will fail
                    one_minus_amp
                )?;

                acc.checked_add(weighted_asset_balance_ampped)
                    .map_err(|err| err.into())
            }
            else {
                Ok(acc)
            }
        })?;

    // Compute (w·a_0)^(1-k) as ('weighted_asset_balance_ampped_sum' - 'unit_tracker)/asset_count
    // 'weighted_asset_balance_ampped_sum' is always larger than 'unit_tracker', since 'unit_tracker'
    // *is* the difference between 'weighted_asset_balance_ampped_sum' and 'weighted_asset_balance_0_ampped_sum' 
    // (that is, its equivalent when the vault is balanced).
    Ok(
        weighted_asset_balance_ampped_sum
            .wrapping_sub(unit_tracker)             // 'wrapping_sub' is safe:
                                                    //   - for positive 'unit_tracker', the reasoning above applies
                                                    //   - for negative 'unit_tracker', the subtraction could actually
                                                    //     overflow, but the result will be correct once casted to u256.
            .as_u256()                              // Casting is safe, see the reasoning of the above line
            .div(U256::from(weights.len() as u64))
    )

}
