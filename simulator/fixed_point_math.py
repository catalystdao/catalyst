from typing import List

from integer import Uint256

UINT256_MAX = Uint256(2**256 - 1)

P_XX      = Uint256(64)
P_XX_MAX  = Uint256(2**64-1)
P_XX_ONE  = Uint256(1)

U256_ZERO = Uint256(0)
U256_ONE  = Uint256(2**64)
U256_MAX  = Uint256(2**256-1)
LN2       = Uint256(12786308645202655660)

TWO_POW_P_XX_PLUS_1 = Uint256(2) << P_XX


TWO_TWO_MINUS_I       : List[Uint256] = [Uint256(26087635650665564425), Uint256(21936999301089678047), Uint256(20116317054877281742), Uint256(19263451207323153962), Uint256(18850675170876015534), Uint256(18647615946650685159), Uint256(18546908069882975960), Uint256(18496758270674070881), Uint256(18471734244850835106), Uint256(18459234930309000272), Uint256(18452988445124272033), Uint256(18449865995240371898), Uint256(18448304968436414829), Uint256(18447524504564044946), Uint256(18447134285009651015), Uint256(18446939178327825412), Uint256(18446841625760745902), Uint256(18446792849670663277), Uint256(18446768461673986097), Uint256(18446756267687738522)]

TWO_MINUS_TWO_MINUS_I : List[Uint256] = [Uint256(13043817825332782212), Uint256(15511800964685064948), Uint256(16915738899553466670), Uint256(17664662643191237676), Uint256(18051468387014017850), Uint256(18248035989933441396), Uint256(18347121020861646923), Uint256(18396865112328554661), Uint256(18421787711448657617), Uint256(18434261669329232139), Uint256(18440501815349552981), Uint256(18443622680442407997), Uint256(18445183311048607332), Uint256(18445963675871538003), Uint256(18446353870663572145), Uint256(18446548971154807802), Uint256(18446646522174239825), Uint256(18446695297877410579), Uint256(18446719685777359790), Uint256(18446731879739425374)]

EXP_POS_LOOKUP        : List[Uint256] = [Uint256(50143449209799256682), Uint256(136304026803256390412), Uint256(1007158100559408451354), Uint256(54988969081439155412736), Uint256(163919806582506698591828152), Uint256(1456609517792428406714055862390917), Uint256(115018199355157251870643531501709554553678249259), Uint256(717155619985916044695037432918736248907406552372215529479395529955709617329)]

EXP_NEG_LOOKUP        : List[Uint256] = [Uint256(30413539329486470295), Uint256(23686088245777032822), Uint256(20902899511243624348), Uint256(19636456851539679189), Uint256(19032306587872971150), Uint256(18737238023755501946), Uint256(18591423680622123788),  Uint256(18518942588666869714), Uint256(18482808078018285842), Uint256(18464767271176066525), Uint256(18455753472345505796), Uint256(18451248223137477973), Uint256(18448996010967782912), Uint256(18447870007976831669), Uint256(18447307032252994902), Uint256(18447025550833756842), Uint256(18446884811734779221), Uint256(18446814442587947178), Uint256(18446779258115194901), Uint256(18446761665903984642)]

INV_EXP_POS_LOOKUP    : List[Uint256] = [Uint256(6786177901268885274), Uint256(2496495334008788799), Uint256(337863903126961437), Uint256(6188193243211692), Uint256(2075907567336), Uint256(233612), Uint256(0), Uint256(0)]

INV_EXP_NEG_LOOKUP    : List[Uint256] = [Uint256(11188515852577165299), Uint256(14366338729722795843), Uint256(16279194507819420732), Uint256(17329112349219823218), Uint256(17879197424118840458), Uint256(18160753814917686419), Uint256(18303190372430456779),  Uint256(18374827034086858296), Uint256(18410750438167364677), Uint256(18428738468430479223), Uint256(18437739073120195921), Uint256(18442241023793258495), Uint256(18444492411329227605), Uint256(18445618208161748319), Uint256(18446181132345977515), Uint256(18446462600880313685), Uint256(18446603336758065834), Uint256(18446673705099591509), Uint256(18446708889371017194), Uint256(18446726481531895805)]

def none_on_exception(func):

    def wrapper(*args, **kwargs) -> Uint256 | None:
        try:
            return func(*args, **kwargs)
        except:
            return None
    
    return wrapper

def mulmod_uint256(a: Uint256, b: Uint256, mod: Uint256) -> Uint256:

    return Uint256((a._value * b._value) % mod._value)

@none_on_exception
def mul_x64(a: Uint256, b: Uint256) -> Uint256:

    r0 = a.overflowing_mul(b)
    r1 = mulmod_uint256(a, b, UINT256_MAX)

    r1 = r1.overflowing_sub(r0) - Uint256(int(r1 < r0))

    if r1 > P_XX_MAX:
        raise ArithmeticError

    return (r1 << (Uint256(256) - P_XX)) + (r0 >> P_XX)


@none_on_exception
def div_x64(a: Uint256, b: Uint256) -> Uint256:
    
    m = P_XX_MAX % b; # 2**p-1 % b
    r = P_XX_MAX / b; # 2**p-1 / b

    return r * a + (m + P_XX_ONE) * a / b


@none_on_exception
def log2_x64(x: Uint256) -> Uint256:

    if x.is_zero():
        raise ArithmeticError()

    x_i = x.copy()
    log2_intermediate = Uint256(0)

    if x_i >= 2**128:
        x_i = x_i >> 128
        log2_intermediate += 128

    if x_i >= 2**64:
        x_i = x_i >> 64
        log2_intermediate += 64

    if x_i >= 2**32:
        x_i = x_i >> 32
        log2_intermediate += 32

    if x_i >= 2**16:
        x_i = x_i >> 16
        log2_intermediate += 16

    if x_i >= 2**8:
        x_i = x_i >> 8
        log2_intermediate += 8

    if x_i >= 2**4:
        x_i = x_i >> 4
        log2_intermediate += 4

    if x_i >= 2**2:
        x_i = x_i >> 2
        log2_intermediate += 2

    if x_i >= 2**1:
        # x_i = x_i >> 1 
        log2_intermediate += 1
    
    log2_intermediate -= P_XX

    # Secure the decimal point
    x_i = x / (P_XX_ONE << log2_intermediate)
    log2_intermediate = log2_intermediate << P_XX

    for i in range(32):
        if x_i >= TWO_POW_P_XX_PLUS_1:
            log2_intermediate += P_XX_ONE << (P_XX - i)
            x_i = x_i >> 1
        
        x_i = (x_i * x_i) >> P_XX
    
    return log2_intermediate


@none_on_exception
def ln_x64(x: Uint256) -> Uint256:

    return mul_x64(log2_x64(x), LN2)


@none_on_exception
def pow2_x64(x: Uint256) -> Uint256:
    major_x = x >> P_XX
    if major_x >= 192:
        raise ArithmeticError()

    intermediate = U256_ONE.copy()
    for i in range(1, 20):
        if x & (Uint256(1) << (P_XX - i)):
            intermediate = (intermediate * TWO_TWO_MINUS_I[i-1]) >> P_XX

    return intermediate << major_x


@none_on_exception
def inv_pow2_x64(x: Uint256) -> Uint256:
    major_x = x >> P_XX

    # dev: Major larger than fixed points. Reserve a few (64-41=23) bits for accuracy
    if major_x >= Uint256(41):
        raise ArithmeticError
    
    intermediate = U256_ONE.copy()
    for i in range(1, 20-1):
        if x & (Uint256(1) << (P_XX - i)):
            intermediate = (intermediate * TWO_MINUS_TWO_MINUS_I[i-1]) >> P_XX

    # Since we are computing 2^(-x) we are not worried about the increase the
    # major contributes with, but with how many decimals it removes from the
    # calculation. We prefer to do it later than sooner to not waste decimals.
    return intermediate >> major_x


@none_on_exception
def pow_x64(x: Uint256, p: Uint256) -> Uint256:
    return pow2_x64(
        mul_x64(p, log2_x64(x))
    )


@none_on_exception
def inv_pow_x64(x: Uint256, p: Uint256) -> Uint256:
    return inv_pow2_x64(
        mul_x64(log2_x64(x), p)
    )


@none_on_exception
def exp_x64(x: Uint256) -> Uint256:
    if (x >> Uint256(64)) > Uint256(134):
        raise ArithmeticError()

    exp_intermediate = U256_ONE.copy()

    for i in range(9):
        if x & (P_XX_ONE << (P_XX + i)):
            exp_intermediate = mul_x64(exp_intermediate, EXP_POS_LOOKUP[i])

    for i in range(20):
        if x & (P_XX_ONE << (P_XX - i - 1)):
            exp_intermediate = mul_x64(exp_intermediate, EXP_NEG_LOOKUP[i])
    
    return exp_intermediate


@none_on_exception
def inv_exp_x64(x: Uint256) -> Uint256:
    if x > (P_XX_ONE << (P_XX + 4)):
        raise ArithmeticError()

    exp_intermediate = U256_ONE.copy()

    for i in range(8):
        if x & (P_XX_ONE << (P_XX + i)):
            exp_intermediate = mul_x64(exp_intermediate, INV_EXP_POS_LOOKUP[i])

    for i in range(20):
        if x & (P_XX_ONE << (P_XX - i - 1)):
            exp_intermediate = mul_x64(exp_intermediate, INV_EXP_NEG_LOOKUP[i])
    
    return exp_intermediate


@none_on_exception
def safe_pow_x64(a: Uint256, b: Uint256, p: Uint256) -> Uint256:
    if a < b:
        return inv_pow_x64(div_x64(b, a), p)
    
    return pow_x64(div_x64(a, b), p)


@none_on_exception
def binomial_expansion_neg_pow_x64(x: Uint256, n: Uint256, rounds: int) -> Uint256:
    if x >= U256_ONE:
        raise ArithmeticError

    prev_term = Uint256(2**64)

    pos_acc = Uint256(2**64)
    neg_acc = Uint256(0)

    i = 0
    while i < int(rounds/2)*2:
        prev_term = ((prev_term * (n+(i<<64)) * x)/(i+1)) >> 128
        neg_acc += prev_term
        i += 1

        prev_term = ((prev_term * (n+(i<<64)) * x)/(i+1)) >> 128 # Overflow safe, as x < 1
        pos_acc += prev_term
        i += 1

    if i < rounds:
        # Compute the last term (required for odd rounds)
        neg_acc += ((prev_term * (n+(i<<64)) * x)/(i+1)) >> 128

    # ! The last term is desired to be a negative one, such that 
    return pos_acc - neg_acc


def mulmod(a: Uint256, b: Uint256, m: Uint256) -> Uint256:
    res = Uint256(0)
    a = a.copy()
    b = b.copy()

    if b >= m:
        if m > (U256_MAX >> P_XX_ONE):
            b -= m
        else:
            b %= m
    

    while a:
        if a & P_XX_ONE:
            if b >= m - res:
                res = res.overflowing_sub(m)
            res = res.overflowing_add(b)
        a >>= P_XX_ONE

        # Double b, module m
        temp_b = b.copy()
        if b >= m - b:  # Equiv to if (2 * b >= m), without overflow
            temp_b = temp_b.overflowing_sub(m)
        b = b.overflowing_add(temp_b)
    
    return res
    