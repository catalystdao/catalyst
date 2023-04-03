use ethnum::U256;
use fixed_point_math_lib::fixed_point_math::{ln_wad, div_wad_down, mul_wad_down, WAD, exp_wad};


// TODO use Uint128 where suitable instead of U256?
// Integral Helpers *************************************************************************************************************

pub fn calc_price_curve_area(
    input: U256,
    a: U256,
    w: U256
) -> Result<U256, ()> {
    w.checked_mul(
        ln_wad(
            div_wad_down(
                a.checked_add(input).ok_or(())?,
                a
            )?.as_i256()
        )?.as_u256()
    ).ok_or(())
}


pub fn calc_price_curve_limit(
    u: U256,
    b: U256,
    w: U256
) -> Result<U256, ()> {
    mul_wad_down(
        b,
        WAD.checked_sub(
            exp_wad(-(u / w).as_i256())?.as_u256()
        ).ok_or(())?
    )
}


pub fn calc_combined_price_curves(
    input: U256,
    a: U256,
    b: U256,
    w_a: U256,
    w_b: U256
) -> Result<U256, ()> {
    calc_price_curve_limit(
        calc_price_curve_area(input, a, w_a)?,
        b,
        w_b
    )
}


pub fn calc_price_curve_limit_share(
    u: U256,
    w_sum: U256
) -> Result<U256, ()> {
    let npos = (
        // If the casting to i256 overflows to a negative value:
        //   - If the result is exactly i256::MIN (i.e. overflows exactly by 1), when the value is
        //     negated it again overflows and remains unchanged (as the operation is unchecked),
        //     i.e. -i256::MIN = i256::MIN. This is not a problem, as it is exactly what it is desired.
        //   - Otherwise, the value becomes positive when it gets negated. This will cause the result 
        //     of the exponent (i.e. npos) to be greater than one, which will cause the 'checked_sub' 
        //     operation inside the following 'div_wad_down' to fail.
        exp_wad(-(u / w_sum).as_i256())?
    ).as_u256();

    div_wad_down(
        WAD.checked_sub(npos).ok_or(())?,
        npos
    )
}
