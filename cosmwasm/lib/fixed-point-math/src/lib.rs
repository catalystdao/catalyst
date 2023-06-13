use catalyst_types::{U256, I256, AsI256, AsU256, u256, i256, errors::OverflowError};
use thiserror::Error;

/// @notice Arithmetic library with operations for fixed-point numbers.
/// @author Solmate (https://github.com/transmissions11/solmate/blob/ed67feda67b24fdeff8ad1032360f0ee6047ba0a/src/utils/FixedPointMathLib.sol)

// NOTE the following code uses 'wrapping' functions (e.g. 'wrapping_add') instead of the intrinsic types (e.g. '+') as they are more gas efficient.
// (The intrinsic types *always* check for overflows. This is specific to the U256 library and to CosmWasm's Uint classes, NOT to Rust native types)

/***************************************************************
                SIMPLIFIED FIXED POINT OPERATIONS
***************************************************************/

pub const WAD    : U256 = u256!("1000000000000000000");                      // The scalar of ETH and most ERC20s.
pub const WADWAD : I256 = i256!("1000000000000000000000000000000000000");    // The scalar of ETH and most ERC20s squared.
pub const LN2    : U256 = u256!("693147180559945344");                       // from numpy import np; int(np.log(2)*10**18).

pub fn mul_wad_down(x: U256, y: U256) -> Result<U256, FixedPointMathError> {
    mul_div_down(x, y, WAD)     // Equivalent to (x * y) / WAD rounded down.
}

pub fn mul_wad_up(x: U256, y: U256) -> Result<U256, FixedPointMathError> {
    mul_div_up(x, y, WAD)       // Equivalent to (x * y) / WAD rounded up.
}

pub fn div_wad_down(x: U256, y: U256) -> Result<U256, FixedPointMathError> {
    mul_div_down(x, WAD, y)  // Equivalent to (x * WAD) / y rounded down.
}

pub fn div_wad_up(x: U256, y: U256) -> Result<U256, FixedPointMathError> {
    mul_div_up(x, WAD, y)    // Equivalent to (x * WAD) / y rounded up.
}

pub fn pow_wad(x: I256, y: I256) -> Result<I256, FixedPointMathError> {
    // Equivalent to x to the power of y because x ** y = (e ** ln(x)) ** y = e ** (ln(x) * y)
    exp_wad(
        ln_wad(x)?.checked_mul(y)? / WAD.as_i256()     // Using ln(x) means x must be greater than 0.
    )
}

pub fn exp_wad(x: I256) -> Result<I256, FixedPointMathError> {   //TODO make output be U256? (result will always be positive)

    // When the result is < 0.5 we return zero. This happens when
    // x <= floor(log(0.5e18) * 1e18) ~ -42e18
    if x <= i256!("-42139678854452767551") { return Ok(I256::zero()) }

    // When the result is > (2**255 - 1) / 1e18 we can not represent it as an
    // int. This happens when x >= floor(log((2**255 - 1) / 1e18) * 1e18) ~ 135.
    if x >= i256!("135305999368893231589") { return Err(FixedPointMathError::OverflowError {}) }

    // x is now in the range (-42, 136) * 1e18. Convert to (-42, 136) * 2**96
    // for more intermediate precision and a binary basis. This base conversion
    // is a multiplication by 1e18 / 2**96 = 5**18 / 2**78.
    let mut x = (x.wrapping_shl(78)) / i256!("3814697265625");

    // Reduce range of x to (-½ ln 2, ½ ln 2) * 2**96 by factoring out powers
    // of two such that exp(x) = exp(x') * 2**k, where k is an integer.
    // Solving this gives k = round(x / log(2)) and x' = x - k * log(2).
    let k: I256 = (
        (
            (x.wrapping_shl(96)) / i256!("54916777467707473351141471128")
        ).wrapping_add(i256!("39614081257132168796771975168"))
    ).wrapping_shr(96);
    x = x.wrapping_sub(k.wrapping_mul(i256!("54916777467707473351141471128")));

    // k is in the range [-61, 195].

    // Evaluate using a (6, 7)-term rational approximation.
    // p is made monic, we'll multiply by a scale factor later.
    let mut y = x.wrapping_add(i256!("1346386616545796478920950773328"));
    y = ((y.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("57155421227552351082224309758442"));
    let mut p = y.wrapping_add(x).wrapping_sub(i256!("94201549194550492254356042504812"));
    p = ((p.wrapping_mul(y)).wrapping_shr(96)).wrapping_add(i256!("28719021644029726153956944680412240"));
    p = p.wrapping_mul(x).wrapping_add(i256!("4385272521454847904659076985693276").wrapping_shl(96));

    // We leave p in 2**192 basis so we don't need to scale it back up for the division.
    let mut q = x.wrapping_sub(i256!("2855989394907223263936484059900"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("50020603652535783019961831881945"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_sub(i256!("533845033583426703283633433725380"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("3604857256930695427073651918091429"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_sub(i256!("14423608567350463180887372962807573"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("26449188498355588339934803723976023"));

    // The q polynomial won't have zeros in the domain as all its roots are complex.
    // No scaling is necessary because p is already 2**96 too large.
    let r: I256 = p / q;

    // r should be in the range (0.09, 0.25) * 2**96.

    // We now need to multiply r by:
    // * the scale factor s = ~6.031367120.
    // * the 2**k factor from the range reduction.
    // * the 1e18 / 2**96 factor for base conversion.
    // We do this all at once, with an intermediate result in 2**213
    // basis, so the final right shift is always by a positive amount.
    Ok(
        r.as_u256()
            .wrapping_mul(u256!("3822833074963236453042738258902158003155416615667"))
            .wrapping_shr(u256!("195").wrapping_sub(k.as_u256()).as_u32())
            .as_i256()
    )
}

pub fn ln_wad(x: I256) -> Result<I256, FixedPointMathError> {   //TODO make input U256?

    if x <= I256::zero() { return Err(FixedPointMathError::UndefinedError {}) }

    // We want to convert x from 10**18 fixed point to 2**96 fixed point.
    // We do this by multiplying by 2**96 / 10**18. But since
    // ln(x * C) = ln(x) + ln(C), we can simply do nothing here
    // and add ln(2**96 / 10**18) at the end.

    // Reduce range of x to (1, 2) * 2**96
    // ln(2^k * x) = k * ln(2) + ln(x)
    let k: i32 = log2(x.as_u256())?.as_i32() - 96;          // ! type u32 mismatch with EVM impl. (safe as log2(U256::MAX)=255, hence k is within [-96, 159])
    let mut x: I256 = x.wrapping_shl((159 - k) as u32);     // ! 159 - k is always >= 0 and <= 255 because of the result of the line above
    x = (x.as_u256().wrapping_shr(159u32)).as_i256();

    // Evaluate using a (8, 8)-term rational approximation.
    // p is made monic, we will multiply by a scale factor later.
    let mut p: I256 = x.wrapping_add(i256!("3273285459638523848632254066296"));
    p = ((p.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("24828157081833163892658089445524"));
    p = ((p.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("43456485725739037958740375743393"));
    p = ((p.wrapping_mul(x)).wrapping_shr(96)).wrapping_sub(i256!("11111509109440967052023855526967"));
    p = ((p.wrapping_mul(x)).wrapping_shr(96)).wrapping_sub(i256!("45023709667254063763336534515857"));
    p = ((p.wrapping_mul(x)).wrapping_shr(96)).wrapping_sub(i256!("14706773417378608786704636184526"));
    p = p.wrapping_mul(x).wrapping_sub(i256!("795164235651350426258249787498") << 96);     //TODO is '<< 96' evaluated at compile time?

    // We leave p in 2**192 basis so we don't need to scale it back up for the division.
    // q is monic by convention.
    let mut q: I256 = x.wrapping_add(i256!("5573035233440673466300451813936"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("71694874799317883764090561454958"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("283447036172924575727196451306956"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("401686690394027663651624208769553"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("204048457590392012362485061816622"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("31853899698501571402653359427138"));
    q = ((q.wrapping_mul(x)).wrapping_shr(96)).wrapping_add(i256!("909429971244387300277376558375"));

    // The q polynomial is known not to have zeros in the domain.
    // No scaling required because p is already 2**96 too large.
    let mut r = p / q;

    // r is in the range (0, 0.125) * 2**96

    // Finalization, we need to:
    // * multiply by the scale factor s = 5.549…
    // * add ln(2**96 / 10**18)
    // * add k * ln(2)
    // * multiply by 10**18 / 2**96 = 5**18 >> 78

    // mul s * 5e18 * 2**96, base is now 5**18 * 2**192
    r = r.wrapping_mul(i256!("1677202110996718588342820967067443963516166"));
    // add ln(2) * k * 5e18 * 2**192
    r = r.wrapping_add(i256!("16597577552685614221487285958193947469193820559219878177908093499208371").wrapping_mul(k.into()));
    // add ln(2**96 / 10**18) * 5e18 * 2**192
    r = r.wrapping_add(i256!("600920179829731861736702779321621459595472258049074101567377883020018308"));
    // base conversion: mul 2**18 / 2**192
    r = r.wrapping_shr(174);

    Ok(r)

}

/***************************************************************
                LOW LEVEL FIXED POINT OPERATIONS
***************************************************************/

pub fn mul_div_down(x: U256, y: U256, denominator: U256) -> Result<U256, FixedPointMathError> {
    // Store x * y in z for now.
    let z = x.wrapping_mul(y);

    // Equivalent to require(denominator != 0 && (x == 0 || (x * y) / x == y))
    if !(denominator != U256::zero() && (x == U256::zero() || (z/x == y))) {
        return Err(FixedPointMathError::ArithemticError {})     //NOTE: Using 'ArithmeticError' as the error could be 'undefined' or 'overflow'.
    }

    // Divide z by the denominator.
    Ok(z / denominator)
}

pub fn mul_div_up(x: U256, y: U256, denominator: U256) -> Result<U256, FixedPointMathError> {
    // Store x * y in z for now.
    let z = x.wrapping_mul(y);

    // Equivalent to require(denominator != 0 && (x == 0 || (x * y) / x == y))
    if !(denominator != U256::zero() && (x == U256::zero() || (z/x == y))) {
        return Err(FixedPointMathError::ArithemticError {})     //NOTE: Using 'ArithmeticError' as the error could be 'undefined' or 'overflow'.
    }

    // First, divide z - 1 by the denominator and add 1.
    // We allow z - 1 to underflow if z is 0, because we multiply the
    // end result by 0 if z is zero, ensuring we return 0 if z is zero.
    Ok(
        (z != U256::zero()).as_u256()
            .wrapping_mul(
                ((z.wrapping_sub(U256::one())) / denominator).wrapping_add(U256::one())
            )
    )
}

/***************************************************************
                    GENERAL NUMBER UTILITIES
***************************************************************/

pub fn log2(x: U256) -> Result<U256, FixedPointMathError> {
    if x == U256::zero() { return Err(FixedPointMathError::UndefinedError {}) }

    let mut r = (u256!("0xffffffffffffffffffffffffffffffff") < x).as_u256().wrapping_shl(7);
    r |= (u256!("0xffffffffffffffff") < (x.wrapping_shr(r.as_u32()))).as_u256().wrapping_shl(6);
    r |= (u256!("0xffffffff") < (x.wrapping_shr(r.as_u32()))).as_u256().wrapping_shl(5);
    r |= (u256!("0xffff") < (x.wrapping_shr(r.as_u32()))).as_u256().wrapping_shl(4);
    r |= (u256!("0xff") < (x.wrapping_shr(r.as_u32()))).as_u256().wrapping_shl(3);
    r |= (u256!("0xf") < (x.wrapping_shr(r.as_u32()))).as_u256().wrapping_shl(2);
    r |= (u256!("0x3") < (x.wrapping_shr(r.as_u32()))).as_u256().wrapping_shl(1);
    r |= (u256!("0x1") < (x.wrapping_shr(r.as_u32()))).as_u256();

    Ok(r)
}

/***************************************************************
                        LIBRARY ERRORS
***************************************************************/

#[derive(Error, Debug)]
pub enum FixedPointMathError {
    #[error("Overflow")]
    OverflowError {},

    #[error("Undefined")]
    UndefinedError {},

    #[error("ArithmeticError")]
    ArithemticError {}
}

impl From<OverflowError> for FixedPointMathError {
    fn from(_value: OverflowError) -> Self {
        FixedPointMathError::OverflowError {}
    }
}

