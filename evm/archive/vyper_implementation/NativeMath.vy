# @version =0.3.3

"""
@author Polymer
Copyright reserved by Polymer
@notice 
    Mathematical functions used by Polymer.

    Functions   |   Diviation   |   range              | Cost avg (high)
    exp         |   0.0011%     | 0 ... 133.6254122101 | 25909 (26652)
    ln          |   0.001%      | 0 ... 2**92          | 35032 (36115)
    x^(n/d)     |   0.0066%     | x:0...2**92          | 38554 (41956)
    floor(log2) |   Exact       | uint256              | 25757 (26531)

    All numbers are brutedforced and does not gurantee certain performance.

"""

LN2: constant(uint256) = 6931471806 #6931471806

# https://medium.com/wicketh/mathemagic-full-multiply-27650fec525d
# @pure
# @internal
# def _chinesemul(a : uint256, b : uint256) -> uint256[2]:
#     r0: uint256 = uint256_mulmod(a,b, 2**256)
#     r1: uint256 = uint256_mulmod(a,b, MAX_UINT256)
#     r1 = (r-r0) - convert(r1 < r0, uint256)

#     return [r0, r1]

@pure
@internal
def _taylorexp(x : decimal) -> decimal:
    """
    1 + x + 1/2*x**2 + 1/6*x**3 + 1/24*x**4 + 1/120*x**5 + 1/720*x**6
    @dev take decimal, which have 10 decimal places of resolution
    """
    # return 1.0 + x + 0.5*(x*x) + 0.1666666667*(x*x*x) + 0.0416666667*(x*x*x*x) + 0.0083333333*(x*x*x*x*x) + 0.0013888889 * (x*x*x*x*x*x)
    return 1.0 + x*(1.0 + 0.5*x*(1.0 + 0.3333333333*x*(1.0 + 0.25*x*(1.0 + 0.2*x*(1.0 + 0.1666666666*x*(1.0 + 0.1428571428*x))))))


# Bug in vyper, has to be in relevant function.
# EXPLOOKUP: constant(uint256[8]) = [2718281828, 7389056099, 5459815003, 2980957987*10**3, 8886110521*10**6, 78962960180*10**12, 6235149081*10**27, 38877084060*10**54]  # div 2^30

@view
@internal
def _exp(x: decimal) -> uint256:
    """
    This function uses the exponential property:
        exp(v) => exp(2^x) => exp(2^0 + 2^1 + 2^2 + ...)
        = exp(2^0) · exp(2^1) · exp(2^2) · ...
    This only works for values which we can index. 
    For the decimal portion the taylor series is used. Such that
    exp(deci + 2^0 + 2^1 + 2^2 + ...) = exp(deci) · exp(2^0) · exp(2^1) · exp(2^2)

    @dev Reverts by overflow if x >= 133.6254122102, last safe is 133.6254122101
    Returns with 10 decimals
    """
    # Bug in Vyper, preferably as a constant.
    # EXPLOOKUP: uint256[8] = [2718281828, 7389056099, 54598150033, 2980957987042, 8886110520507873, 78962960182680695160978, 6235149080811616882909238708928469745, 38877084059945950922226736883574780727281750630829988860000000000]

    EXPLOOKUP: uint256[8] = [2918732889, 7933938573, 58624317204, 3200779266274, 9541388518555713, 84785832894990942831737, 6694940346942588912244160397776260189, 41743951150327690684646218602937660698511553484260542869998347594]  # Terms of 2^30


    dec: decimal = x-convert(floor(x), decimal)
    x_int: uint256 = convert(x, uint256)
    exp_intermediate: uint256 = 2**30
    for i in range(8):
        cond: uint256 = bitwise_and(x_int, shift(1,i))
        if cond > 0:
            exp_intermediate = shift(exp_intermediate*EXPLOOKUP[i], -30)  # Divide by 2^30
        
    return shift(convert(self._taylorexp(dec)*10000000000.0, uint256) * exp_intermediate, -30)


@view
@external
def iexp(x: uint256) -> uint256:
    """
    @dev Takes in and returns with 10 decimals resolution
    """

    return self._exp(convert(x, decimal)/10000000000.0)



@pure
@internal
def _log2(x: uint256) -> uint256:
    assert x != 0
    x_i: uint256 = x
    log2_intermediate: uint256 = 0
      
    for i in range(8):
        p: int128 = convert(shift(1, 7 - i), int128)
        mask: uint256 = shift(1, p) - 1
        y_i: uint256 = shift(x_i, -p)
        cond: uint256 = bitwise_and(y_i, mask)
        if cond > 0 : # real code would have no jump
          x_i = y_i
          log2_intermediate += convert(p, uint256)
    
    return log2_intermediate


@view
@external
def il2(x: uint256) -> uint256:
    return self._log2(x)




@pure
@internal
def _taylorln(x: decimal) -> decimal:
    """

    (x-1) - 1/2 (x-1)**2 + 1/3 (x-1)**3 - 1/4 (x-1)**4 +...
    (-1)^(i+1) · 1/i (x-1)**i, i = 1, 2, 3, ...


    """
    a: decimal = (1.0 - x)
    b: decimal = -1.0
    ln_intermediate: decimal = 0.0
    for i in range(1, 6):  # (1, r), if r is uneven: then _ln is always weakly less. if r is even: then _ln is always weakly larger.
        b *= a
        ln_intermediate += 1.0/convert(i, decimal) * b

    return ln_intermediate

@view
@internal
def _ln(x: uint256) -> decimal:
    """
    This function uses the ln property:
        ln(v) => ln(2^x) => ln(2^y · deci) = ln(2^y) + ln(deci)
        = ln(2) · ln2(2^y) + ln(deci)
    @dev 
        Reverts by overflow somewhere in the area above 2**92
    Returns with 10 decimals

    1 newton and 15 taylor, max 0.001 divation (not tested lower)
    └─ iln         -  avg:  37329  avg (confirmed):  40335  low:  21552  high:  41471

    Without newton and 15 taylor, passes 1% diviation test
    └─ iln         -  avg:  32732  avg (confirmed):  34869  low:  21552  high:  35648

    Without newton and 7 taylor, fails 1% diviation test
    └─ iln         -  avg:  29104  avg (confirmed):  30241  low:  21552  high:  30896

    1 Newton and 6 taylor, passes 0.001% diviation test
    └─ iln         -  avg:  32457  avg (confirmed):  34239  low:  21552  high:  36103

    From tests, newton costs 5400
    From tests, 8 taylor costs 4628

    """
    l2: uint256 = self._log2(x)
    filled: uint256 = shift(1, convert(l2, int128))
    if x - filled != 0:
        multipl: decimal = convert(x, decimal)/convert(filled, decimal)
        ln_part: decimal = self._taylorln(multipl)

        y_i: decimal = convert(LN2 * l2, decimal)/10000000000.0 + ln_part

        # Newtons
        if y_i <= 133.6254122101:  # remove this if?
            x_l: uint256 = x*10000000000  
            exp_yi: uint256 = self._exp(y_i)
            if x_l > exp_yi:
                y_i = y_i + 2.0 * convert(x_l-exp_yi, decimal)/convert(x_l+exp_yi, decimal)
            else:
                y_i = y_i - 2.0 * convert(exp_yi-x_l, decimal)/convert(x_l+exp_yi, decimal)

        return y_i
    return convert(LN2 * l2, decimal)/10000000000.0


@view
@external
def iln(x: uint256) -> uint256:
    return convert(self._ln(x)*10000000000.0, uint256)


@view
@internal
def _nd_pow(x: uint256, n: uint256, d: uint256) -> uint256:
    """
    Calculates x^(n/d) by exp(ln(x)*n/d) = x^(n/d)
    """
    ln: decimal = self._ln(x)
    power: decimal = (ln * convert(n, decimal))/convert(d, decimal)
    return self._exp(power)


@view
@external
def nd_pow(x: uint256, n: uint256, d: uint256) -> uint256:
    return self._nd_pow(x, n, d)



