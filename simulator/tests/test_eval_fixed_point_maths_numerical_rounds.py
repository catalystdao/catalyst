
from typing import List

from integer import Uint256
from typing import List
from utils.eval_utils import evaluate_implementation_1_var, sample_space

from utils.math_utils import real_pow2_x64, real_inv_pow2_x64, real_log2_x64

import matplotlib.pyplot as plt

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


# Copied functions from fixed_point_math.py *************************************************************************************
# Modified for testing (add 'rounds' parameters)
def none_on_exception(func):

    def wrapper(*args, **kwargs) -> Uint256 | None:
        try:
            return func(*args, **kwargs)
        except:
            return None
    
    return wrapper


@none_on_exception
def log2_rounds_x64(x: Uint256, decimal_rounds = 32) -> Uint256:

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

    for i in range(decimal_rounds):
        if x_i >= TWO_POW_P_XX_PLUS_1:
            log2_intermediate += P_XX_ONE << (P_XX - i)
            x_i = x_i >> 1
        
        x_i = (x_i * x_i) >> P_XX
    
    return log2_intermediate

@none_on_exception
def pow2_rounds_x64(x: Uint256, rounds = 19) -> Uint256:
    major_x = x >> P_XX
    if major_x >= 192:
        raise ArithmeticError()

    intermediate = U256_ONE.copy()
    for i in range(1, rounds + 1):
        if x & (Uint256(1) << (P_XX - i)):
            intermediate = (intermediate * TWO_TWO_MINUS_I[i-1]) >> P_XX

    return intermediate << major_x

@none_on_exception
def inv_pow2_rounds_x64(x: Uint256, rounds = 18) -> Uint256:
    major_x = x >> P_XX

    # dev: Major larger than fixed points. Reserve a few (64-41=23) bits for accuracy
    if major_x >= Uint256(41):
        raise ArithmeticError
    
    intermediate = U256_ONE.copy()
    for i in range(1, rounds+1):
        if x & (Uint256(1) << (P_XX - i)):
            intermediate = (intermediate * TWO_MINUS_TWO_MINUS_I[i-1]) >> P_XX

    # Since we are computing 2^(-x) we are not worried about the increase the
    # major contributes with, but with how many decimals it removes from the
    # calculation. We prefer to do it later than sooner to not waste decimals.
    return intermediate >> major_x
# Testing functions *************************************************************************************************************

def test_eval_log2_rounds():
    
    eval_ranges = [
        [1 * 2**64, 2 * 2**64],
        [1 * 2**64, 1 * 2**64 + 2**37],
        [1 * 2**64 + 2**54, 1 * 2**64 + 2**60]
    ]

    min_rounds = 16
    max_rounds = 32
    rounds_range = range(min_rounds, max_rounds+1)


    for eval_range in eval_ranges:

        avg_error = []
        max_error = []

        for rounds in rounds_range:

            def wrapped_log2_x64(x: Uint256):
                return log2_rounds_x64(x, rounds)

            result = evaluate_implementation_1_var(
                sample_space(
                    10000,
                    range_start = eval_range[0],
                    range_end   = eval_range[1]
                ),
                wrapped_log2_x64,
                real_log2_x64,
                "$\log_2(x)$",
                show_plots=False,
                save_plots=False
            )

            avg_error.append(result.avg_error)
            max_error.append(result.max_error)
        
        
        fig_func, ax = plt.subplots()

        ax.set_yscale('log')

        ax.scatter(
            rounds_range,
            avg_error,
            s=10,
            label="Average error"
        )

        ax.scatter(
            rounds_range,
            max_error,
            s=10,
            c="orange",
            label="Maximum error"
        )

        ax.set_title("$\log_2(x)$ Rel. Error vs Rounds", fontsize=20)
        ax.grid(True, which='major')
        ax.set_axisbelow(True)
        ax.set_xlabel("Rounds")
        ax.set_ylabel("Relative error (-2 to 2) - Log Scale")

        # Save plots
        ax.legend()
        plt.savefig("log2_rounds", dpi=300, bbox_inches='tight')

        plt.show()


def test_eval_pow2_rounds():
    
    eval_ranges = [
        [0, 256 * 2**64]
    ]

    min_rounds = 10
    max_rounds = 20
    rounds_range = range(min_rounds, max_rounds+1)


    for eval_range in eval_ranges:

        avg_error = []
        max_error = []

        for rounds in rounds_range:

            def wrapped_pow2_x64(x: Uint256):
                return pow2_rounds_x64(x, rounds)

            result = evaluate_implementation_1_var(
                sample_space(
                    10000,
                    range_start = eval_range[0],
                    range_end   = eval_range[1]
                ),
                wrapped_pow2_x64,
                real_pow2_x64,
                "$2^x$",
                show_plots=False,
                save_plots=False
            )

            avg_error.append(result.avg_error)
            max_error.append(result.max_error)
        
        
        fig_func, ax = plt.subplots()

        ax.set_yscale('log')

        ax.scatter(
            rounds_range,
            avg_error,
            s=10,
            label="Average error"
        )

        ax.scatter(
            rounds_range,
            max_error,
            s=10,
            c="orange",
            label="Maximum error"
        )

        ax.set_title("$2^x$ Rel. Error vs Rounds", fontsize=20)
        ax.grid(True, which='major')
        ax.set_axisbelow(True)
        ax.set_xlabel("Rounds")
        ax.set_ylabel("Relative error (-2 to 2) - Log Scale")

        # Save plots
        ax.legend()
        plt.savefig("pow2_rounds", dpi=300, bbox_inches='tight')

        plt.show()


def test_eval_inv_pow2_rounds():
    
    eval_ranges = [
        [0, 55 * 2**64],
        [0, 2**(64-40)]
    ]

    min_rounds = 10
    max_rounds = 20
    rounds_range = range(min_rounds, max_rounds+1)


    for eval_range in eval_ranges:

        avg_error = []
        max_error = []

        for rounds in rounds_range:

            def wrapped_inv_pow2_x64(x: Uint256):
                return inv_pow2_rounds_x64(x, rounds)

            result = evaluate_implementation_1_var(
                sample_space(
                    10000,
                    range_start = eval_range[0],
                    range_end   = eval_range[1]
                ),
                wrapped_inv_pow2_x64,
                real_inv_pow2_x64,
                "$2^{-x}$",
                show_plots=False,
                save_plots=False
            )

            avg_error.append(result.avg_error)
            max_error.append(result.max_error)
        
        
        fig_func, ax = plt.subplots()

        ax.set_yscale('log')

        ax.scatter(
            rounds_range,
            avg_error,
            s=10,
            label="Average error"
        )

        ax.scatter(
            rounds_range,
            max_error,
            s=10,
            c="orange",
            label="Maximum error"
        )

        ax.set_title("$2^{-x}$ Rel. Error vs Rounds", fontsize=20)
        ax.grid(True, which='major')
        ax.set_axisbelow(True)
        ax.set_xlabel("Rounds")
        ax.set_ylabel("Relative error (-2 to 2) - Log Scale")

        # Save plots
        ax.legend()
        plt.savefig("inv_pow2_rounds", dpi=300, bbox_inches='tight')

        plt.show()
