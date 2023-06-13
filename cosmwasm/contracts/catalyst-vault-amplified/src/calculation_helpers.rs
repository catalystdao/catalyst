use catalyst_types::{U256, AsI256, AsU256};
use catalyst_vault_common::ContractError;
use fixed_point_math::{ln_wad, div_wad_down, mul_wad_down, WAD, exp_wad};


// TODO use Uint128 where suitable instead of U256?
// TODO add overflow safety comments
// Integral Helpers *************************************************************************************************************

pub fn calc_price_curve_area(
    input: U256,
    a: U256,
    w: U256
) -> Result<U256, ContractError> {
    todo!()
}


pub fn calc_price_curve_limit(
    u: U256,
    b: U256,
    w: U256
) -> Result<U256, ContractError> {
    todo!()
}


pub fn calc_combined_price_curves(
    input: U256,
    a: U256,
    b: U256,
    w_a: U256,
    w_b: U256
) -> Result<U256, ContractError> {
    todo!()
}


pub fn calc_price_curve_limit_share(
    u: U256,
    w_sum: U256
) -> Result<U256, ContractError> {
    todo!()
}
