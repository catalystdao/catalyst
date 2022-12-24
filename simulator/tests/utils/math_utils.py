from math import exp, log2, log


# Accurate mathematical calculations for testing of the fixed point math library ***********************************************

U256_ONE: int = 2**64

def real_mul_x64(a: int, b: int) -> int:
    return a * b // U256_ONE


def real_div_x64(a: int, b: int) -> int | None:
    if b == 0:
        return None
    return (a * U256_ONE) // b


def real_log2_x64(n: int) -> int | None:
    if n < 2**64: return None   # If this check is not here, 2**64-1 will return 0 due to rounding errors

    return int(log2(n / U256_ONE) * U256_ONE)


def real_ln_x64(n: int) -> int | None:
    if n < 2**64: return None   # If this check is not here, 2**64-1 will return 0 due to rounding errors

    return int(log(n / U256_ONE) * U256_ONE)


def real_pow2_x64(x: int) -> int | None:
    # Cap result to <= U256_x64.max, that is x < (256 - 64)*2**64, but allow some extra margin for margin errors, 
    # otherwise (256-64)*2**64 - 1 will fail

    val = x / U256_ONE
    if (val >= (256 - 64) + 1):
        return None

    return int(2 ** val * U256_ONE)


def real_inv_pow2_x64(x: int) -> int:
    return int(2 ** (- x / U256_ONE) * U256_ONE)


def real_pow_x64(x: int, p: int) -> int | None:
    try:
        return (int((x / U256_ONE) ** (p / U256_ONE) * U256_ONE))
    except:
        return None     # Calculation overflow for extremely large numbers


def real_inv_pow_x64(x: int, p: int) -> int | None:
    try:
        return int((x / U256_ONE) ** (- p / U256_ONE) * U256_ONE)
    except:
        return None     # Calculation overflow for extremely large numbers


def real_exp_x64(p: int) -> int | None:
    # Cap result <= U256_x64.max, that is p <= int(ln(U256_x64.max >> 64) * 2**64) -> p <= 134 * 2**64 (allow margin for rounding errors)
    if p > 134 * 2**64:
        return None
    return int(exp(p / U256_ONE) * U256_ONE)


def real_inv_exp_x64(p: int) -> int:
    # TODO cap result as above?
    return int(exp(- p / U256_ONE) * U256_ONE)
