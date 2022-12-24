use crate::u256::U256;

const P_XX      : u64  = 64;
const P_XX_MAX  : U256 = U256([0xFFFFFFFFFFFFFFFFu64, 0, 0, 0]);
const P_XX_ONE  : U256 = U256([1, 0, 0, 0]);

pub const ZERO_X64 : U256 = U256([0, 0, 0, 0]);
pub const ONE_X64  : U256 = U256([0, 1, 0, 0]);
pub const LN2_X64  : U256 = U256([12786308645202655660, 0, 0, 0]);
pub const U256_MAX : U256 = U256([0xFFFFFFFFFFFFFFFFu64, 0xFFFFFFFFFFFFFFFFu64, 0xFFFFFFFFFFFFFFFFu64, 0xFFFFFFFFFFFFFFFFu64]);

// See FixedPointMathX64.vy for insight into the following lookup tables
// TODO write explanation here
const TWO_TWO_MINUS_I       : [U256; 20] = [U256([7640891576956012809, 1, 0, 0]), U256([3490255227380126431, 1, 0, 0]), U256([1669572981167730126, 1, 0, 0]), U256([816707133613602346, 1, 0, 0]), U256([403931097166463918, 1, 0, 0]), U256([200871872941133543, 1, 0, 0]), U256([100163996173424344, 1, 0, 0]), U256([50014196964519265, 1, 0, 0]), U256([24990171141283490, 1, 0, 0]), U256([12490856599448656, 1, 0, 0]), U256([6244371414720417, 1, 0, 0]), U256([3121921530820282, 1, 0, 0]), U256([1560894726863213, 1, 0, 0]), U256([780430854493330, 1, 0, 0]), U256([390211300099399, 1, 0, 0]), U256([195104618273796, 1, 0, 0]), U256([97552051194286, 1, 0, 0]), U256([48775961111661, 1, 0, 0]), U256([24387964434481, 1, 0, 0]), U256([12193978186906, 1, 0, 0])];

const TWO_MINUS_TWO_MINUS_I : [U256; 20] = [U256([13043817825332782212, 0, 0, 0]), U256([15511800964685064948, 0, 0, 0]), U256([16915738899553466670, 0, 0, 0]), U256([17664662643191237676, 0, 0, 0]), U256([18051468387014017850, 0, 0, 0]), U256([18248035989933441396, 0, 0, 0]), U256([18347121020861646923, 0, 0, 0]), U256([18396865112328554661, 0, 0, 0]), U256([18421787711448657617, 0, 0, 0]), U256([18434261669329232139, 0, 0, 0]), U256([18440501815349552981, 0, 0, 0]), U256([18443622680442407997, 0, 0, 0]), U256([18445183311048607332, 0, 0, 0]), U256([18445963675871538003, 0, 0, 0]), U256([18446353870663572145, 0, 0, 0]), U256([18446548971154807802, 0, 0, 0]), U256([18446646522174239825, 0, 0, 0]), U256([18446695297877410579, 0, 0, 0]), U256([18446719685777359790, 0, 0, 0]), U256([18446731879739425374, 0, 0, 0])];

const EXP_POS_LOOKUP        : [U256; 8]  = [U256([13249961062380153450, 2, 0, 0]), U256([7176818287289529100, 7, 0, 0]), U256([11033920579092664090, 54, 0, 0]), U256([17671741784691597056, 2980, 0, 0]), U256([9601675514881374392, 8886110, 0, 0]), U256([12823456651613180037, 78962960182680, 0, 0]), U256([17127243763087097131, 17696838799656736180, 338008108, 0]), U256([13460297379963274417, 4294423684612430841, 17671928477841822154, 114249481722274167])];

const EXP_NEG_LOOKUP        : [U256; 20] = [U256([11966795255776918679, 1, 0, 0]), U256([5239344172067481206, 1, 0, 0]), U256([2456155437534072732, 1, 0, 0]), U256([1189712777830127573, 1, 0, 0]), U256([585562514163419534, 1, 0, 0]), U256([290493950045950330, 1, 0, 0]), U256([144679606912572172, 1, 0, 0]), U256([72198514957318098, 1, 0, 0]), U256([36064004308734226, 1, 0, 0]), U256([18023197466514909, 1, 0, 0]), U256([9009398635954180, 1, 0, 0]), U256([4504149427926357, 1, 0, 0]), U256([2251937258231296, 1, 0, 0]), U256([1125934267280053, 1, 0, 0]), U256([562958543443286, 1, 0, 0]), U256([281477124205226, 1, 0, 0]), U256([140738025227605, 1, 0, 0]), U256([70368878395562, 1, 0, 0]), U256([35184405643285, 1, 0, 0]), U256([17592194433026, 1, 0, 0])];

const INV_EXP_POS_LOOKUP    : [U256; 8]  = [U256([6786177901268885274, 0, 0, 0]), U256([2496495334008788799, 0, 0, 0]), U256([337863903126961437, 0, 0, 0]), U256([6188193243211692, 0, 0, 0]), U256([2075907567336, 0, 0, 0]), U256([233612, 0, 0, 0]), U256([0, 0, 0, 0]), U256([0, 0, 0, 0])];

const INV_EXP_NEG_LOOKUP    : [U256; 20] = [U256([11188515852577165299, 0, 0, 0]), U256([14366338729722795843, 0, 0, 0]), U256([16279194507819420732, 0, 0, 0]), U256([17329112349219823218, 0, 0, 0]), U256([17879197424118840458, 0, 0, 0]), U256([18160753814917686419, 0, 0, 0]), U256([18303190372430456779, 0, 0, 0]), U256([18374827034086858296, 0, 0, 0]), U256([18410750438167364677, 0, 0, 0]), U256([18428738468430479223, 0, 0, 0]), U256([18437739073120195921, 0, 0, 0]), U256([18442241023793258495, 0, 0, 0]), U256([18444492411329227605, 0, 0, 0]), U256([18445618208161748319, 0, 0, 0]), U256([18446181132345977515, 0, 0, 0]), U256([18446462600880313685, 0, 0, 0]), U256([18446603336758065834, 0, 0, 0]), U256([18446673705099591509, 0, 0, 0]), U256([18446708889371017194, 0, 0, 0]), U256([18446726481531895805, 0, 0, 0])];

// TODO currently these functions return Result<>. would it be better to simply panic?

pub fn mul_x64(a: U256, b: U256) -> Result<U256, ()> {
    let (r0, _) = a.overflowing_mul(b);
    let mut r1 = mulmod(a, b, U256_MAX);

    r1 = r1.overflowing_sub(r0).0 - U256::from((r1 < r0) as u64);

    // !!CRITICAL!! Check for second order overflow.
    // We know r1 expand with shift(r1, HighBase-pXX)
    // r1 · 2^(256-64) = r1 · 2^192 < 2^256
    // r1 < 2^64 (or r1 <= 2^64 - 1, that is P_XX_MAX)
    if r1 > P_XX_MAX { return Err(()); }

    Ok((r1 << (256-P_XX)) + (r0 >> P_XX))
}


pub fn div_x64(a: U256, b: U256) -> Result<U256, ()> {
    if b.is_zero() { return Err(()) };

    let m = P_XX_MAX % b; // 2**p-1 % b
    let r = P_XX_MAX / b; // 2**p-1 / b

    // Ok(r * a + (m + P_XX_ONE) * a / b)
    r.checked_mul(a).ok_or(())?
        .checked_add(
            (m.checked_add(P_XX_ONE).ok_or(())?).checked_mul(a / b).ok_or(())?
        ).ok_or(())

    //TODO fix overflow of (m + P_XX_ONE) * a
    // let (partial, overflowed) = (m + P_XX_ONE).overflowing_mul(a);
    // if overflowed {
    //     return Err(());
    // }
    // Ok(r * a + partial / b) 
}


pub fn log2_x64(x: U256) -> Result<U256, ()> {
    if x.is_zero() { return Err(()) }

    let mut x_i = x.clone();
    let mut log2_intermediate = ZERO_X64;

    if x_i >= U256([0x0, 0x0, 0x1, 0x0]) {          // 2**128
        x_i = x_i >> 128u32;
        log2_intermediate += U256([128, 0, 0, 0]);
    }
    if x_i >= U256([0x0, 0x1, 0x0, 0x0]) {          // 2**64
        x_i = x_i >> 64u32;
        log2_intermediate += U256([64, 0, 0, 0]);
    }
    if x_i >= U256([0x100000000, 0x0, 0x0, 0x0]) {  // 2**32
        x_i = x_i >> 32u32;
        log2_intermediate += U256([32, 0, 0, 0]);
    }
    if x_i >= U256([0x10000, 0x0, 0x0, 0x0]) {      // 2**16
        x_i = x_i >> 16u32;
        log2_intermediate += U256([16, 0, 0, 0]);
    }
    if x_i >= U256([0x100, 0x0, 0x0, 0x0]) {        // 2**8
        x_i = x_i >> 8u32;
        log2_intermediate += U256([8, 0, 0, 0]);
    }
    if x_i >= U256([0x10, 0x0, 0x0, 0x0]) {         // 2**4
        x_i = x_i >> 4u32;
        log2_intermediate += U256([4, 0, 0, 0]);
    }
    if x_i >= U256([0x4, 0x0, 0x0, 0x0]) {          // 2**2
        x_i = x_i >> 2u32;
        log2_intermediate += U256([2, 0, 0, 0]);
    }
    if x_i >= U256([0x2, 0x0, 0x0, 0x0]) {          // 2**1
        // x_i = x_i >> 1u32;
        log2_intermediate += U256([1, 0, 0, 0]);
    }
    log2_intermediate = log2_intermediate.checked_sub(U256([P_XX, 0, 0, 0])).ok_or(())?;
    // TODO: Is it cheaper to get the major and then run this on the major
    // instead of running it on the whole and then removing 64.

    // Secure the decimal point
    x_i = x / (P_XX_ONE << log2_intermediate);
    log2_intermediate = log2_intermediate << P_XX;
    for i in 0..32 {
        if x_i >= (U256([2, 0, 0, 0]) << P_XX) {    //TODO inefficient - why computing each time?
            log2_intermediate += P_XX_ONE << (P_XX - i);
            x_i = x_i >> 1u64;
        }
        x_i = (x_i * x_i) >> P_XX  // Since x_i is max 2**64*2*2 => (2**66) ** 2  = 2**132 => x_i · x_i < 2**132. No overflow
    }

    Ok(log2_intermediate)
}


pub fn ln_x64(x: U256) -> Result<U256, ()> {
    Ok(mul_x64(log2_x64(x)?, LN2_X64)?)
}


pub fn pow2_x64(x: U256) -> Result<U256, ()> {
    let major_x = x >> P_XX;
    if major_x >= U256([192, 0, 0, 0]) { return Err(()) };

    let mut intermediate = ONE_X64; // 2**64
    for i in 1..20 {
        let cond = x & P_XX_ONE << ((P_XX as usize) - i);
        if !cond.is_zero() {
            // TODO remove if
            intermediate = (intermediate * TWO_TWO_MINUS_I[i-1]) >> P_XX;
        }
    }

    // The major part is added here to increase the size of the number we can compute.
    Ok(intermediate << major_x)

}


pub fn inv_pow2_x64(x: U256) -> Result<U256, ()> {
    let major_x = x >> P_XX;

    // dev: Major larger than fixed points. Reserve a few (64-41=23) bits for accuracy
    if major_x >=  U256([41, 0, 0, 0]) { return Err(()) };

    let mut intermediate = ONE_X64; // 2**64
    for i in 1..(20-1) {
        let cond = x & (P_XX_ONE << ((P_XX as usize) - i));
        if !cond.is_zero() { //TODO remove if
            intermediate = (intermediate * TWO_MINUS_TWO_MINUS_I[i-1]) >> P_XX;
        }
    }

    // Since we are computing 2^(-x) we are not worried about the increase the
    // major contributes with, but with how many decimals it removes from the
    // calculation. We prefer to do it later than sooner to not waste decimals.
    Ok(intermediate >> major_x)
}
    
pub fn pow_x64(x: U256, p: U256) -> Result<U256, ()> {
    Ok(pow2_x64(
      mul_x64(p, log2_x64(x)?)? 
    )?)
}
    
pub fn inv_pow_x64(x: U256, p: U256) -> Result<U256, ()> {
    Ok(inv_pow2_x64(
      mul_x64(log2_x64(x)?, p)? 
    )?)
}
    
pub fn exp_x64(x: U256) -> Result<U256, ()> {
    if (x >> U256([64, 0, 0, 0])) > U256([134, 0, 0, 0]) { return Err(()) };

    let mut exp_intermediate = ONE_X64; // 2**64
    for i in 0..8 {
        let cond = x & (P_XX_ONE << (i + (P_XX as usize)));
        if !cond.is_zero() {
            exp_intermediate = mul_x64(exp_intermediate, EXP_POS_LOOKUP[i])?;
        }
    }

    for i in 0..20 {
        let cond = x & (P_XX_ONE << ((P_XX as usize) - i - 1));
        if !cond.is_zero() {
            exp_intermediate = mul_x64(exp_intermediate, EXP_NEG_LOOKUP[i])?;
        }
    }

    Ok(exp_intermediate)
}
    
pub fn inv_exp_x64(x: U256) -> Result<U256, ()> {
    if x > (P_XX_ONE << ((P_XX as usize) + 4)) { return Err(()) };

    let mut exp_intermediate = ONE_X64; // 2**64
    for i in 0..8 {
        let cond = x & (P_XX_ONE << (i + (P_XX as usize)));
        if !cond.is_zero() {
            exp_intermediate = mul_x64(exp_intermediate, INV_EXP_POS_LOOKUP[i])?;
        }
    }

    for i in 0..20 {
        let cond = x & (P_XX_ONE << ((P_XX as usize) - i - 1));
        if !cond.is_zero() {
            exp_intermediate = mul_x64(exp_intermediate, INV_EXP_NEG_LOOKUP[i])?;
        }
    }

    Ok(exp_intermediate)
}
    
pub fn safe_pow_x64(a: U256, b: U256, p: U256) -> Result<U256, ()> {
    if a < b {
        return Ok(inv_pow_x64(div_x64(b, a)?, p)?);
    }
    Ok(pow_x64(div_x64(a, b)?, p)?)
}


// https://stackoverflow.com/questions/12168348/ways-to-do-modulo-multiplication-with-primitive-types
fn mulmod(a: U256, b: U256, m: U256) -> U256 {
    let mut res = ZERO_X64;
    let mut a   = a.clone();
    let mut b   = b.clone();

    /* Only needed if b >= m */
    if b >= m {
        if m > (U256::max_value() >> P_XX_ONE) { b -= m }
        else { b %= m };
    }

    while !a.is_zero() {
        if !(a & P_XX_ONE).is_zero() {
            /* Add b to res, modulo m, without overflow */
            if b >= m - res {
                res = res.overflowing_sub(m).0;
            }
            res = res.overflowing_add(b).0;
        }
        a >>= P_XX_ONE;

        /* Double b, modulo m */
        let mut temp_b = b.clone();
        if b >= m - b { /* Equiv to if (2 * b >= m), without overflow */
            temp_b = temp_b.overflowing_sub(m).0
        }
        b = b.overflowing_add(temp_b).0;
    }

    res
}


