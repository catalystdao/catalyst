# @version =0.3.3

"""
@author Polymer
Copyright reserved by Polymer
@notice 
    Fixed point mathematics used by Polymer.
    If a fixed point number is stored inside uint256, the variable
    should be clearly marked. For examplf, if 24 bits are reserved
    for the decimal, the variable should have p24 or X24 appened.

    Contains the following mathematical descriptions in Vyper:
    pMULX64
    a : X64, b : X64
    a · b => y : X64, as long as y does not overflow X64.
    
    pMULX64_abc
    a : X64, b : X64, c : X64
    a · b · c => y : X64, as long as y does not overflow X64.
    
    bigdiv64
    a : uint256, b : uint256
    (a << 64)/b => y, as long as y does not overflow uint256.
    
    _log2X64
    x : uint256
    log2(x) => y : X64, as long as y >= 0

    _lnX64
    x : X64
    ln(x) => y : X64, as long as y >= 0

    _p2X64
    x : X64, x < 192
    2**x => y : X64

    _fpowX64
    x : X64, p : X64
    x**p => y : X64, depends on p2 and log2

    _expX64
    x : X64
    exp(x) => y : X64

    All power functions have inverse companions, which may have other limitations.
"""

#
# Polymer LIB[FixedPointMathX64.vy] v0.1
#

LN2: constant(uint256) = 12786308645202655660 #6931471806

pXX: constant(int128) = 64
inverseLN2: constant(uint256) = 26613026195688644983  # 1/ln(2)
ONE: constant(uint256) = 2**64


# Credit: https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d
@pure
@internal 
def pMULX64(a : uint256, b : uint256) -> uint256:
    """
     @notice Safely calculates a : X64 times b : X64 while returning X64.
     @dev Reverts if a · b > 2**(256-64)-1
     @param a uint256, X64 Factor 1
     @param b uint256, X64 Factor 2
     @return uint256, X64:  a times b
    """
    r0: uint256 = unsafe_mul(a, b)  # same as uint256_mulmod(a,b, 2**256)
    r1: uint256 = uint256_mulmod(a, b, MAX_UINT256)
    r1 = (r1 - r0) - convert(r1 < r0, uint256)  # (r1 - r0) - {r1 < r0 ? 1 : 0}

    # !!CRITICAL!! Check for second order overflow.
    # We know r1 expand with shift(r1, 256-pXX)
    # r1 · 2^(256-64) = r1 · 2^192 < 2^256
    # r1 < 2^64
    # r1 < 2**64 - 1. Minus because we are indexing from 0.
    assert r1 < 2**64 - 1

    # The overflow part is now stored in r1 while the remainder is in r0.
    # The true number is thus r1 · 2^256 + r0
    # This could be improved to 
    points: uint256 = shift(r0, -pXX)
    # minor: uint256 = shift(r0, 256-pXX*2) # Shifting major away
    # minor = shift(minor, pXX-256) # Shifting minor in place
    # major: uint256 = shift(r0, -pXX*2) # Shifting minor away
    # major = shift(major, pXX) # Shifting major in place

    return shift(r1, 256-pXX) + points


@view
@external
def imul(a : uint256, b : uint256) -> uint256:
    return self.pMULX64(a, b)


# While one could theoretically make these dynamically, hardcoding
# the shift is cheaper.
# Credit: https://github.com/vyperlang/vyper/issues/1086
@view
@internal
def bigdiv64(a : uint256, b : uint256) -> uint256:
    """
     @notice Safely calculates (a << 64)/b
     @dev 
        Reverts normally if result overflows.
        To get (a << p)/b replace (2**64 - 1) by (2**p - 1)
     @param a uint256 numerator
     @param b uint256 denuminator
     @return uint256 (a << 64)/b
    """
    m: uint256 = (2**64 - 1) % b
    r: uint256 = (2**64 - 1) / b
    return r * a + (m + 1) * a / b


@pure
@internal
def _log2X64(x : uint256) -> uint256:
    """
    @notice
        Fixed point number can be written as
        x = m · 2^(-pXX)
        log2(x) = log2(m · 2^(-pXX)) = log2(m) + log2(2^(-pXX))
        log2(x) = log2(m) - pXX
        This finds the integer part

        Let a be the integer part of log2, then
        log2(x) - a is the decimal part.
        log2(x) - log2(2^a) = log2(x/2^a)
        x/2^a is definitly in [1, 2) and only 1 if number could be expressed as 2^a.
     @dev 
        for i in range(V) goes through the remaining bits. 
        Set v to the smaller bit one wants included
     @param x uint256, X64
     @return uint256, X64 as log2(x/2**64)*2**64
    """

    assert x != 0
    x_i: uint256 = x
    log2_intermediate: uint256 = 0
    
    if x_i >= 2**128: 
        x_i = shift(x_i, -128)
        log2_intermediate += 128
    if x_i >= 2**64: 
        x_i = shift(x_i, -64)
        log2_intermediate += 64
    if x_i >= 2**32: 
        x_i = shift(x_i, -32)
        log2_intermediate += 32
    if x_i >= 2**16: 
        x_i = shift(x_i, -16)
        log2_intermediate += 16
    if x_i >= 2**8: 
        x_i = shift(x_i, -8)
        log2_intermediate += 8
    if x_i >= 2**4: 
        x_i = shift(x_i, -4)
        log2_intermediate += 4
    if x_i >= 2**2: 
        x_i = shift(x_i, -2)
        log2_intermediate += 2
    if x_i >= 2**1: 
        # x_i = shift(x_i, -1)
        log2_intermediate += 1
    log2_intermediate -= convert(pXX, uint256) 
    # TODO: Is it cheaper to get the major and then run this on the major
    # instead of running it on the whole and then removing 64.

    # Secure the decimal point
    x_i = x/shift(1, convert(log2_intermediate, int128))
    log2_intermediate = shift(log2_intermediate, pXX)
    for i in range(32):  # Supposedly: 1/2**24 => .0.0000059605% diviation, but I am getting more like 1/2**20 diviation => .0000953674% diviation
        if x_i >= shift(2, pXX):
            log2_intermediate += shift(1, pXX - i)
            x_i = shift(x_i, -1)
        x_i = shift(x_i * x_i, -pXX)  # Since x_i is max 2**64*2*2 => (2**66) ** 2  = 2**132 => x_i · x_i < 2**132. No overflow
    
    return log2_intermediate


@view
@external
def ilog2X64(x : uint256) -> uint256:
    return self._log2X64(x)


@view
@internal
def _lnX64(x : uint256) -> uint256:
    """
    @notice
        log2(x) = log(x)/log(2) => log(x) = log2(x) · log(2)
     @param x uint256, X64
     @return uint256, X64
    """
    return self.pMULX64(self._log2X64(x), LN2)


@view
@external
def ln(x : uint256) -> uint256:
    return self._lnX64(x)


@pure
@internal
def _p2X64(x : uint256) -> uint256:
    """
    @notice
        We can write x as
        x = 2^y = 2^v + 2^-1 + 2^-2 + ...
        
        2^x = 2^(2^v + 2^-1 + 2^-2 + ...) = 2^major · 2^(2^-1) · 2^(2^-2) · ...
        2^(2^-i) is precomputed.
     @dev 
        for i in range(1, 20-1) surfs over the 63 to 0 bits.
     @param x uint256, X64
     @return uint256, X64 as 2**(x/2**64)*2**64
    """
    
    # Get major of x
    major_x: uint256 = shift(x, -64)
    assert major_x < 192

    # 2^(2^(-i)) * 2^64, i = 1..
    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    TWOTWOMINUSI: uint256[20] = [
    26087635650665564425, 21936999301089678047, 20116317054877281742, 19263451207323153962, 18850675170876015534, 18647615946650685159, 18546908069882975960, 18496758270674070881, 18471734244850835106, 18459234930309000272, 18452988445124272033, 18449865995240371898, 18448304968436414829, 18447524504564044946, 18447134285009651015, 18446939178327825412, 18446841625760745902, 18446792849670663277, 18446768461673986097, 18446756267687738522 ]
    # 18446750170697637486, 18446747122203342655, 18446745597956384162, 18446744835832952145,
    # 18446744454771247945, 18446744264240398796, 18446744168974974960, 18446744121342263227,
    # 18446744097525907406, 18446744085617729507, 18446744079663640561, 18446744076686596088,
    # 18446744075198073852, 18446744074453812734, 18446744074081682175, 18446744073895616895,
    # 18446744073802584256, 18446744073756067936, 18446744073732809776, 18446744073721180696,
    # 18446744073715366156, 18446744073712458886, 18446744073711005251, 18446744073710278433,
    # 18446744073709915024, 18446744073709733320, 18446744073709642468, 18446744073709597042,
    # 18446744073709574329, 18446744073709562973, 18446744073709557294, 18446744073709554455,
    # 18446744073709553036, 18446744073709552326, 18446744073709551971, 18446744073709551793,
    # 18446744073709551705, 18446744073709551660, 18446744073709551638, 18446744073709551627,
    # 18446744073709551622, 18446744073709551619, 18446744073709551617, 18446744073709551617 ]


    intermediate: uint256 = 2**64 
    # for i in range(1, 64-16-1):
    for i in range(1, 20-1):
        cond: uint256 = bitwise_and(x, shift(1, 64-i))
        if cond > 0:  # TODO: Remove if
            intermediate = shift(intermediate*TWOTWOMINUSI[i-1], -64)
    

    # The major part is added here to increase the size of the number we can compute.
    return shift(intermediate, convert(major_x, int128))


@view
@external
def ip2X64(x : uint256) -> uint256:
    return self._p2X64(x)


@pure
@internal
def _invp2X64(x : uint256) -> uint256:
    """
    @notice
        We can write x as
        x = 2^y = 2^v + 2^-1 + 2^-2 + ...
        
        2^-x = 2^(-(2^v + 2^-1 + 2^-2 + ...)) = 2^-major · 2^(-2^-1) · 2^(-2^-2) · ...
        2^(-2^-i) is precomputed.
     @dev 
        for i in range(1, 20-1) surfs over the 63 to 0 bits.
     @param x uint256, X64
     @return uint256, X64 as 2**(-x/2**64)*2**64
    """
    
    # Get major of x
    major_x: uint256 = shift(x, -64)
    assert major_x < 41  # dev: Major larger then fixed points. Reserve a few (64-41=23) bits for accuracy

    # 2^(-2^(-i)) * 2^64, i = 1..
    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    TWOTWOMINUSI: uint256[20] = [13043817825332782212, 15511800964685064948, 16915738899553466670, 17664662643191237676, 18051468387014017850, 18248035989933441396, 18347121020861646923, 18396865112328554661, 18421787711448657617, 18434261669329232139, 18440501815349552981, 18443622680442407997, 18445183311048607332, 18445963675871538003, 18446353870663572145, 18446548971154807802, 18446646522174239825, 18446695297877410579, 18446719685777359790,  18446731879739425374 ] #, 18446737976723480912, 18446741025216264368, 18446742549462845018, 18446743311586182573, 18446743692647863158, 18446743883178706403, 18446743978444128763, 18446744026076840128, 18446744049893195856, 18446744061801373732, 18446744067755462673, 18446744070732507144]


    intermediate: uint256 = 2**64 
    for i in range(1, 20-1): 
        cond: uint256 = bitwise_and(x, shift(1, 64-i))
        if cond > 0:  # TODO: Remove if
            intermediate = shift(intermediate*TWOTWOMINUSI[i-1], -64)
    

    # Since we are computing 2^(-x) we are not worried about the increased the
    # major contributes with, but with how many decimals it removes from the
    # calculation. We prefer to do it later than sooner to now waste decimals.
    return shift(intermediate, -convert(major_x, int128))


@view
@external
def iinvp2X64(x : uint256) -> uint256:
    return self._invp2X64(x)


@view
@internal
def _fpowX64(x : uint256, p : uint256) -> uint256:
    """
    @notice
        x^p = 2^(p · log2(x))
     @dev Depends heavily on log2 and p2. Remember that. 
     @param x uint256, X64
     @return uint256, X64
    """
    return self._p2X64(
        self.pMULX64(p, self._log2X64(x))
    )


@view
@external
def ifpowX64(x : uint256, p : uint256) -> uint256:
    return self._fpowX64(x, p)


@view
@internal
def _invfpowX64(x : uint256, p : uint256) -> uint256:
    """
    @notice
        x^p = 2^(-(p · log2(x)))
     @dev Depends heavily on log2 and invp2. Remember that. 
     @param x uint256, X64
     @return uint256, X64
    """
    return self._invp2X64(
        self.pMULX64(self._log2X64(x), p)
    )


@view
@external
def iinvfpowX64(x : uint256, p : uint256) -> uint256:
    return self._invfpowX64(x, p)


@view
@internal
def _expX64(x: uint256) -> uint256:
    """
    @notice
        This function uses the exponential property:
        exp(v) => exp(2^x) => exp(... 2^-2 + 2^-1 + 2^0 + 2^1 + 2^2 + ...)
        = ... · exp(2^-2) · exp(2^-1) · exp(2^0) · exp(2^1) · exp(2^2) · ...
        
        Each element is then precomputed and searched for.
     @param x uint256, X64
     @return uint256, X64 as exp(x/2**64)*2**64
    """

    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    posLOOKUP: uint256[8] = [50143449209799256682, 136304026803256390412, 1007158100559408451354, 54988969081439155412736, 163919806582506698591828152, 1456609517792428406714055862390917, 115018199355157251870643531501709554553678249259, 717155619985916044695037432918736248907406552372215529479395529955709617329]


    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    negLOOKUP: uint256[20] = [30413539329486470295, 23686088245777032822, 20902899511243624348, 19636456851539679189, 19032306587872971150, 18737238023755501946, 18591423680622123788,  18518942588666869714, 18482808078018285842, 18464767271176066525, 18455753472345505796, 18451248223137477973, 18448996010967782912, 18447870007976831669, 18447307032252994902, 18447025550833756842, 18446884811734779221, 18446814442587947178, 18446779258115194901, 18446761665903984642]


    exp_intermediate: uint256 = 2**64
    for i in range(8):
        cond: uint256 = bitwise_and(x, shift(1, i+64))
        if cond > 0:
            exp_intermediate = self.pMULX64(exp_intermediate, posLOOKUP[i])
    
    for i in range(20):
        cond: uint256 = bitwise_and(x, shift(1, 64-i-1))
        if cond > 0:
            exp_intermediate = self.pMULX64(exp_intermediate, negLOOKUP[i])

    return exp_intermediate


@view
@external
def expX64(x: uint256) -> uint256:
    return self._expX64(x)



@view
@internal
def _invExpX64(x: uint256) -> uint256:
    """
    @notice
        This function uses the exponential property:
        exp(-v) => exp(-2^x) => exp(... - 2^-2 - 2^-1 - 2^0 - 2^1 - 2^2 - ...)
        = ... · exp(-2^-2) · exp(-2^-1) · exp(-2^0) · exp(-2^1) · exp(-2^2) · ...
        
        Each element is then precomputed and searched for.
     @param x uint256, X64
     @return uint256, X64 as exp(-x/2**64)*2**64
    """
    assert x <= 2**(64+5)

    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    posLOOKUP: uint256[8] = [6786177901268885274, 2496495334008788799, 337863903126961437, 6188193243211692, 2075907567336, 233612, 0, 0]

    # Bug in Vyper, preferably as a constant.
    # https://github.com/vyperlang/vyper/issues/2156
    negLOOKUP: uint256[20] = [11188515852577165299, 14366338729722795843, 16279194507819420732, 17329112349219823218, 17879197424118840458, 18160753814917686419, 18303190372430456779,  18374827034086858296, 18410750438167364677, 18428738468430479223, 18437739073120195921, 18442241023793258495, 18444492411329227605, 18445618208161748319, 18446181132345977515, 18446462600880313685, 18446603336758065834, 18446673705099591509, 18446708889371017194, 18446726481531895805]


    exp_intermediate: uint256 = 2**64
    for i in range(8):
        cond: uint256 = bitwise_and(x, shift(1, i+64))
        if cond > 0:
            exp_intermediate = self.pMULX64(exp_intermediate, posLOOKUP[i])
    
    for i in range(20):
        cond: uint256 = bitwise_and(x, shift(1, 64-i-1))
        if cond > 0:
            exp_intermediate = self.pMULX64(exp_intermediate, negLOOKUP[i])

    return exp_intermediate


@view
@external
def invExpX64(x: uint256) -> uint256:
    return self._invExpX64(x)


@view
@internal
def _safe_fpowX64(a : uint256, b : uint256, p : uint256) -> uint256:
    """
    @notice
        To calculate (a/b)^p using the identitiy: (a/b)^p = 2^(log2(a/b)*p)
        with log2(a/b) only working for a/b > 1 and 2^x only working for x > 0,
        one can use the trick: a/b < 1 => b > a => b/a > 1.
        Selectivly using fpow and invfpow thus allows one to compute (a/b)^p
        for any a/b.
        The alternative would be wrap 2^x to handle and x and use
        log2(a/b) = log2(a) - log2(b). However, this requires 1 more log2 calculation
        and wrapping 2^x is still needed, since that is a based around lookups.
     @param a uint256, X64 Factor 1
     @param b uint256, X64 Factor 2
     @param p uint256, X64 power.
     @return uint256, X64 as (a/b)^p
    """
    if a < b:
        return self._invfpowX64(self.bigdiv64(b,a), p)
    return self._fpowX64(self.bigdiv64(a, b), p)


@view
@external
def isafe_fpowX64(a : uint256, b : uint256, p : uint256) -> uint256:
    return self._safe_fpowX64(a, b, p)