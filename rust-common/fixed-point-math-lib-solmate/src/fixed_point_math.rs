use ethnum::{U256, I256, uint, int, AsU256};

/// @notice Arithmetic library with operations for fixed-point numbers.
/// @author Solmate (https://github.com/transmissions11/solmate/blob/ed67feda67b24fdeff8ad1032360f0ee6047ba0a/src/utils/FixedPointMathLib.sol)


/***************************************************************
                SIMPLIFIED FIXED POINT OPERATIONS
***************************************************************/

pub const WAD    : U256 = uint!("1000000000000000000");                      // The scalar of ETH and most ERC20s.
pub const WADWAD : U256 = uint!("1000000000000000000000000000000000000");    // The scalar of ETH and most ERC20s squared.
pub const LN2    : U256 = uint!("693147180559945344");                       // from numpy import np; int(np.log(2)*10**18).

pub fn mul_wad_down(x: U256, y: U256) -> Result<U256, ()> {
    mul_div_down(x, y, WAD)     // Equivalent to (x * y) / WAD rounded down.
}

pub fn mul_wad_up(x: U256, y: U256) -> Result<U256, ()> {
    mul_div_up(x, y, WAD)       // Equivalent to (x * y) / WAD rounded up.
}

pub fn div_wad_down(x: U256, y: U256) -> Result<U256, ()> {
    mul_div_down(x, WAD, y)  // Equivalent to (x * WAD) / y rounded down.
}

pub fn div_wad_up(x: U256, y: U256) -> Result<U256, ()> {
    mul_div_up(x, WAD, y)    // Equivalent to (x * WAD) / y rounded up.
}

pub fn pow_wad(x: I256, y: I256) -> Result<I256, ()> {
    // Equivalent to x to the power of y because x ** y = (e ** ln(x)) ** y = e ** (ln(x) * y)
    exp_wad(
        ln_wad(x)?.checked_mul(y).ok_or(())? / WAD.as_i256()     // Using ln(x) means x must be greater than 0.
    )
}

pub fn exp_wad(x: I256) -> Result<I256, ()> {   //TODO make input U256?   //TODO make output be U256? (result will always be positive)

    // When the result is < 0.5 we return zero. This happens when
    // x <= floor(log(0.5e18) * 1e18) ~ -42e18
    if x <= int!("-42139678854452767551") { return Ok(I256::ZERO) }     //TODO assume x is positive and skip this?

    // When the result is > (2**255 - 1) / 1e18 we can not represent it as an
    // int. This happens when x >= floor(log((2**255 - 1) / 1e18) * 1e18) ~ 135.
    if x >= int!("135305999368893231589") { return Err(()) }    //TODO ERROR overflow

    // x is now in the range (-42, 136) * 1e18. Convert to (-42, 136) * 2**96
    // for more intermediate precision and a binary basis. This base conversion
    // is a multiplication by 1e18 / 2**96 = 5**18 / 2**78.
    let mut x = (x << 78) / int!("3814697265625");

    // Reduce range of x to (-½ ln 2, ½ ln 2) * 2**96 by factoring out powers
    // of two such that exp(x) = exp(x') * 2**k, where k is an integer.
    // Solving this gives k = round(x / log(2)) and x' = x - k * log(2).
    let k: I256 = ((x << 96) / int!("54916777467707473351141471128") + int!("39614081257132168796771975168")) >> 96;
    x = x - k * int!("54916777467707473351141471128");

    // k is in the range [-61, 195].

    // Evaluate using a (6, 7)-term rational approximation.
    // p is made monic, we'll multiply by a scale factor later.
    let mut y = x + int!("1346386616545796478920950773328");
    y = ((y * x) >> 96) + int!("57155421227552351082224309758442");
    let mut p = y + x - int!("94201549194550492254356042504812");
    p = ((p * y) >> 96) + int!("28719021644029726153956944680412240");
    p = p * x + (int!("4385272521454847904659076985693276") << 96);

    // We leave p in 2**192 basis so we don't need to scale it back up for the division.
    let mut q = x - int!("2855989394907223263936484059900");
    q = ((q * x) >> 96) + int!("50020603652535783019961831881945");
    q = ((q * x) >> 96) - int!("533845033583426703283633433725380");
    q = ((q * x) >> 96) + int!("3604857256930695427073651918091429");
    q = ((q * x) >> 96) - int!("14423608567350463180887372962807573");
    q = ((q * x) >> 96) + int!("26449188498355588339934803723976023");

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
        ((r.as_u256() * uint!("3822833074963236453042738258902158003155416615667")) >> (195i32 - k.as_i32())).as_i256()
    )
}

pub fn ln_wad(x: I256) -> Result<I256, ()> {   //TODO make input U256?

    if x <= I256::ZERO { return Err(()) }   //TODO ERROR undefined

    // We want to convert x from 10**18 fixed point to 2**96 fixed point.
    // We do this by multiplying by 2**96 / 10**18. But since
    // ln(x * C) = ln(x) + ln(C), we can simply do nothing here
    // and add ln(2**96 / 10**18) at the end.

    // Reduce range of x to (1, 2) * 2**96
    // ln(2^k * x) = k * ln(2) + ln(x)
    let k: I256 = log2(x.as_u256())?.as_i256() - 96;
    let mut x: I256 = x << (159 - k).as_u256();     //TODO remove casting to u256?
    x = (x.as_u256() >> 159u32).as_i256();

    // Evaluate using a (8, 8)-term rational approximation.
    // p is made monic, we will multiply by a scale factor later.
    let mut p: I256 = x + int!("3273285459638523848632254066296");
    p = ((p * x) >> 96) + int!("24828157081833163892658089445524");
    p = ((p * x) >> 96) + int!("43456485725739037958740375743393");
    p = ((p * x) >> 96) - int!("11111509109440967052023855526967");
    p = ((p * x) >> 96) - int!("45023709667254063763336534515857");
    p = ((p * x) >> 96) - int!("14706773417378608786704636184526");
    p = p * x - (int!("795164235651350426258249787498") << 96);     //TODO is '<< 96' evaluated at compile time?

    // We leave p in 2**192 basis so we don't need to scale it back up for the division.
    // q is monic by convention.
    let mut q: I256 = x + int!("5573035233440673466300451813936");
    q = ((q * x) >> 96) + int!("71694874799317883764090561454958");
    q = ((q * x) >> 96) + int!("283447036172924575727196451306956");
    q = ((q * x) >> 96) + int!("401686690394027663651624208769553");
    q = ((q * x) >> 96) + int!("204048457590392012362485061816622");
    q = ((q * x) >> 96) + int!("31853899698501571402653359427138");
    q = ((q * x) >> 96) + int!("909429971244387300277376558375");

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
    r *= int!("1677202110996718588342820967067443963516166");
    // add ln(2) * k * 5e18 * 2**192
    r += int!("16597577552685614221487285958193947469193820559219878177908093499208371") * k;
    // add ln(2**96 / 10**18) * 5e18 * 2**192
    r += int!("600920179829731861736702779321621459595472258049074101567377883020018308");
    // base conversion: mul 2**18 / 2**192
    r >>= 174;

    Ok(r)

}

/***************************************************************
                LOW LEVEL FIXED POINT OPERATIONS
***************************************************************/

pub fn mul_div_down(x: U256, y: U256, denominator: U256) -> Result<U256, ()> {
    // Store x * y in z for now.
    let z = x * y;

    // Equivalent to require(denominator != 0 && (x == 0 || (x * y) / x == y))
    if !(denominator != U256::ZERO && (x == U256::ZERO || (z/x == y))) {
        return Err(());     //TODO error
    }

    // Divide z by the denominator.
    Ok(z / denominator)
}

pub fn mul_div_up(x: U256, y: U256, denominator: U256) -> Result<U256, ()> {
    // Store x * y in z for now.
    let z = x * y;

    // Equivalent to require(denominator != 0 && (x == 0 || (x * y) / x == y))
    if !(denominator != U256::ZERO && (x == U256::ZERO || (z/x == y))) {
        return Err(());     //TODO error
    }

    // First, divide z - 1 by the denominator and add 1.
    // We allow z - 1 to underflow if z is 0, because we multiply the
    // end result by 0 if z is zero, ensuring we return 0 if z is zero.
    Ok((z != U256::ZERO).as_u256() * ((z - 1) / denominator + 1))     //TODO is '(z != U256::ZERO).as_u256()' more efficient than an if statement? //TODO does (bool).as_u256() always give the desired result?
}

/***************************************************************
                    GENERAL NUMBER UTILITIES
***************************************************************/

pub fn log2(x: U256) -> Result<U256, ()> {
    if x == U256::ZERO { return Err(()) }

    let mut r = (uint!("0xffffffffffffffffffffffffffffffff") < x).as_u256() << 7;
    r |=  (uint!("0xffffffffffffffff") < (x >> r)).as_u256() << 6;
    r |=  (uint!("0xffffffff") < (x >> r)).as_u256() << 5;
    r |=  (uint!("0xffff") < (x >> r)).as_u256() << 4;
    r |=  (uint!("0xff") < (x >> r)).as_u256() << 3;
    r |=  (uint!("0xf") < (x >> r)).as_u256() << 2;
    r |=  (uint!("0x3") < (x >> r)).as_u256() << 1;
    r |=  (uint!("0x1") < (x >> r)).as_u256();

    Ok(r)
}