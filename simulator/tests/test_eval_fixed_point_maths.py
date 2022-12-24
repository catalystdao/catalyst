
import math
from typing import List, Tuple
from utils.eval_utils import evaluate_implementation_1_var, evaluate_implementation_2_vars, filter_lists, get_all_combinations, get_powers_of_2_x64, get_powers_of_2_minus_1_x64, remove_duplicates_and_sort, sample_2d_space, sample_space

from fixed_point_math import U256_MAX, pow2_x64, inv_pow2_x64, exp_x64, inv_exp_x64, ln_x64, log2_x64, mul_x64, div_x64, pow_x64, inv_pow_x64
from utils.math_utils import real_pow2_x64, real_inv_pow2_x64, real_exp_x64, real_inv_exp_x64, real_ln_x64, real_log2_x64, real_mul_x64, real_div_x64, real_pow_x64, real_inv_pow_x64


# Functions of 1 variable *******************************************************************************************************

def test_eval_pow2():

    max_input = (256-64)*2**64-1

    # Evaluate points of interest
    points_of_interest = remove_duplicates_and_sort([
        0,
        1*2**64,
        *get_powers_of_2_x64(-64, 8),
        *get_powers_of_2_minus_1_x64(-64, 8),
        max_input,    # Maximum allowed input
        max_input+1    # Should fail
    ])

    # evaluate_implementation_1_var(
    #     points_of_interest,
    #     pow2_x64,
    #     real_pow2_x64,
    #     "$2^x$",
    #     comp_plot_legend_loc="upper left",
    #     show_plots=True,
    #     save_plots=True
    # )


    # Evaluate ranges

    evaluate_implementation_1_var(
        sample_space(
            20000,
            range_start = 0,
            range_end   = 256 * 2**64
        ),
        pow2_x64,
        real_pow2_x64,
        "$2^x$",
        comp_plot_legend_loc="upper left",
        show_plots=True,
        save_plots=True
    )


def test_eval_inv_pow2():

    # Evaluate points of interest
    max_input = 41*2**64-1
    points_of_interest = remove_duplicates_and_sort([
        0,
        1*2**64,
        *get_powers_of_2_x64(-64, 6),
        *get_powers_of_2_minus_1_x64(-64, 6),
        max_input,    # Maximum allowed input
        max_input+1    # Should fail
    ])

    # evaluate_implementation_1_var(
    #     points_of_interest,
    #     inv_pow2_x64,
    #     real_inv_pow2_x64,
    #     "$2^{-x}$",
    #     show_plots=True,
    #     save_plots=True
    # )


    # Evaluate ranges

    evaluate_implementation_1_var(
        sample_space(
            20000,
            range_start = 0,
            range_end   = 55 * 2**64
        ),
        inv_pow2_x64,
        real_inv_pow2_x64,
        "$2^{-x}$",
        show_plots=True,
        save_plots=True
    )


def test_eval_exp():

    # Find maximum input
    max_input = 2454971266624766607359

    # Evaluate points of interest
    points_of_interest = remove_duplicates_and_sort([
        0,
        1*2**64,
        *get_powers_of_2_x64(-64, 8),
        *get_powers_of_2_minus_1_x64(-64, 8),
        max_input,
        max_input+1
    ])

    # evaluate_implementation_1_var(
    #     points_of_interest,
    #     exp_x64,
    #     real_exp_x64,
    #     "$e^{x}$",
    #     show_plots=True,
    #     save_plots=True
    # )

    evaluate_implementation_1_var(
        sample_space(
            20000,
            range_start = 0,
            range_end   = 150 * 2**64
        ),
        exp_x64,
        real_exp_x64,
        "$e^{x}$",
        show_plots=True,
        save_plots=True
    )


def test_eval_inv_exp():

    # Find maximum input
    max_input = 16*2**64

    # Evaluate points of interest
    points_of_interest = remove_duplicates_and_sort([
        0,
        1*2**64,
        *get_powers_of_2_x64(-64, 5),
        *get_powers_of_2_minus_1_x64(-64, 5),
        max_input,
        max_input+1
    ])

    # evaluate_implementation_1_var(
    #     points_of_interest,
    #     inv_exp_x64,
    #     real_inv_exp_x64,
    #     "$e^{-x}$",
    #     show_plots=True,
    #     save_plots=True
    # )

    evaluate_implementation_1_var(
        sample_space(
            20000,
            range_start = 0,
            range_end   = 20 * 2**64
        ),
        inv_exp_x64,
        real_inv_exp_x64,
        "$e^{-x}$",
        show_plots=True,
        save_plots=True
    )


def test_eval_log2():

    # Evaluate points of interest
    min_input = 2**64
    max_input = 2**256-1
    points_of_interest = remove_duplicates_and_sort([
        0,              # Must fail!
        1,              # Must fail!
        min_input-1,    # Must fail!
        min_input, 
        *get_powers_of_2_x64(-64, 0),         # Must fail!
        *get_powers_of_2_x64(0, 256-64),
        *get_powers_of_2_minus_1_x64(-64, 1), # Must fail!
        *get_powers_of_2_minus_1_x64(1, 256-64+1),
        max_input       # Note that it doesn't makes sense to test for max_input + 1, since it cannot be represented by a u256 number
    ])

    # evaluate_implementation_1_var(
    #     points_of_interest,
    #     log2_x64,
    #     real_log2_x64,
    #     "$\log_2(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )


    # evaluate_implementation_1_var(
    #     sample_space(
    #         20000,
    #         range_start = 1 * 2**64,
    #         range_end   = 2**(256-64) * 2**64
    #     ),
    #     log2_x64,
    #     real_log2_x64,
    #     "$\log_2(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_1_var(
    #     sample_space(
    #         20000,
    #         range_start = 1 * 2**64,
    #         range_end   = 2 * 2**64
    #     ),
    #     log2_x64,
    #     real_log2_x64,
    #     "$\log_2(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_1_var(
    #     sample_space(
    #         20000,
    #         range_start = 1 * 2**64,
    #         range_end   = 1 * 2**64 + 2**37
    #     ),
    #     log2_x64,
    #     real_log2_x64,
    #     "$\log_2(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    evaluate_implementation_1_var(
        sample_space(
            20000,
            range_start = 1 * 2**64 + 2**54,
            range_end   = 1 * 2**64 + 2**60
        ),
        log2_x64,
        real_log2_x64,
        "$\log_2(x)$",
        show_plots=True,
        save_plots=True
    )


def test_eval_ln():

    # Evaluate points of interest
    min_input = 2**64
    max_input = 2**256-1
    points_of_interest = remove_duplicates_and_sort([
        0,              # Must fail!
        1,              # Must fail!
        min_input-1,    # Must fail!
        min_input, 
        *get_powers_of_2_x64(-64, 0),         # Must fail!
        *get_powers_of_2_x64(0, 256-64),
        *get_powers_of_2_minus_1_x64(-64, 1), # Must fail!
        *get_powers_of_2_minus_1_x64(1, 256-64+1),
        max_input       # Note that it doesn't makes sense to test for max_input + 1, since it cannot be represented by a u256 number
    ])

    # evaluate_implementation_1_var(
    #     points_of_interest,
    #     ln_x64,
    #     real_ln_x64,
    #     "$\ln(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # # Evaluate ranges

    # evaluate_implementation_1_var(
    #     sample_space(
    #         20000,
    #         range_start = 1 * 2**64,
    #         range_end   = 2**(256-64) * 2**64
    #     ),
    #     ln_x64,
    #     real_ln_x64,
    #     "$\ln(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_1_var(
    #     sample_space(
    #         20000,
    #         range_start = 1 * 2**64,
    #         range_end   = 2 * 2**64
    #     ),
    #     ln_x64,
    #     real_ln_x64,
    #     "$\ln(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_1_var(
    #     sample_space(
    #         20000,
    #         range_start = 1 * 2**64,
    #         range_end   = 1 * 2**64 + 2**37
    #     ),
    #     ln_x64,
    #     real_ln_x64,
    #     "$\ln(x)$",
    #     show_plots=True,
    #     save_plots=True
    # )

    evaluate_implementation_1_var(
        sample_space(
            20000,
            range_start = 1 * 2**64 + 2**54,
            range_end   = 1 * 2**64 + 2**60
        ),
        ln_x64,
        real_ln_x64,
        "$\ln(x)$",
        show_plots=True,
        save_plots=True
    )




# Functions of 2 variables ******************************************************************************************************

def test_eval_mul():

    # Points of interest

    x_points_of_interest = remove_duplicates_and_sort([
        0,
        1,
        2**64,
        *get_powers_of_2_x64(-64, 256-64, 4),
        *get_powers_of_2_minus_1_x64(-64, 256-64+1, 4),
        2**(256-64)*2**64-1
    ])
    y_points_of_interest = x_points_of_interest.copy()

    x_points_of_interest, y_points_of_interest = get_all_combinations(x_points_of_interest, y_points_of_interest)

    # Separate expected valid and invalid inputs to better verify correct operation of the implementation
    (x_points_valid, y_points_valid), (x_points_invalid, y_points_invalid) = filter_lists(
        x_points_of_interest,
        y_points_of_interest,
        lambda x, y: real_mul_x64(x, y) <= U256_MAX
    )

    evaluate_implementation_2_vars(
        (x_points_valid, y_points_valid),
        mul_x64,
        real_mul_x64,
        "$x\cdot y$",
        show_plots=False,
        save_plots=False
    )

    evaluate_implementation_2_vars(
        (x_points_invalid, y_points_invalid),
        mul_x64,
        real_mul_x64,
        "$x\cdot y$",
        show_plots=False,
        save_plots=False
    )


    # Range Analysis

    def generate_samples(
        sample_count  : int,
        x_range_start : int,
        x_range_end   : int,
        y_range_start : int,
        y_range_end   : int
    ) -> Tuple[List[int], List[int]]:

        return sample_2d_space(
            sample_count  = sample_count,
            x_range_start = x_range_start,
            x_range_end   = x_range_end,
            y_range_start = y_range_start,
            y_range_end   = y_range_end,  
            x_max_func    = lambda y: int((2**256/y)*2**64),
            y_max_func    = lambda x: int((2**256/x)*2**64),
        )

    
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 2*2**64, 
    #         y_range_start = 0,
    #         y_range_end   = 2*2**64
    #     ),
    #     mul_x64,
    #     real_mul_x64,
    #     "$x\cdot y$",
    #     show_plots=True,
    #     save_plots=True
    # )

    
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 2**98*2**64, 
    #         y_range_start = 0,
    #         y_range_end   = 2**98*2**64
    #     ),
    #     mul_x64,
    #     real_mul_x64,
    #     "$x\cdot y$",
    #     show_plots=True,
    #     save_plots=True
    # )

    
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 2**(256-64)*2**64, 
    #         y_range_start = 0,
    #         y_range_end   = 2**(256-64)*2**64
    #     ),
    #     mul_x64,
    #     real_mul_x64,
    #     "$x\cdot y$",
    #     show_plots=True,
    #     save_plots=True
    # )



def test_eval_div():

    # Points of interest

    x_points_of_interest = remove_duplicates_and_sort([
        0,
        1,
        2**64,
        *get_powers_of_2_x64(-64, 256-64, 4),
        *get_powers_of_2_minus_1_x64(-64, 256-64+1, 4),
        2**(256-64)*2**64-1
    ])
    y_points_of_interest = x_points_of_interest.copy()

    x_points_of_interest, y_points_of_interest = get_all_combinations(x_points_of_interest, y_points_of_interest)

    # Separate expected valid and invalid inputs to better verify correct operation of the implementation
    def filter_func(x: int, y: int) -> bool:
        if y == 0: return False

        if (((2**64-1) % y) + 1)*x > U256_MAX:  # ! This filters out all invalid outputs of the div implementation that are due to the current implementation bug
            return False

        expected_val = real_div_x64(x, y)
        return expected_val is not None and expected_val <= U256_MAX

    (x_points_valid, y_points_valid), (x_points_invalid, y_points_invalid) = filter_lists(
        x_points_of_interest,
        y_points_of_interest,
        filter_func
    )

    evaluate_implementation_2_vars(
        (x_points_valid, y_points_valid),
        div_x64,
        real_div_x64,
        "$x/y$",
        show_plots=True,
        save_plots=False
    )

    evaluate_implementation_2_vars(
        (x_points_invalid, y_points_invalid),
        div_x64,
        real_div_x64,
        "$x/y$",
        show_plots=False,
        save_plots=False
    )


    # Range Analysis

    def generate_samples(
        sample_count  : int,
        x_range_start : int,
        x_range_end   : int,
        y_range_start : int,
        y_range_end   : int
    ) -> Tuple[List[int], List[int]]:

        return sample_2d_space(
            sample_count  = sample_count,
            x_range_start = x_range_start,
            x_range_end   = x_range_end,
            y_range_start = y_range_start,
            y_range_end   = y_range_end,
            x_min_func    = lambda y: int(y*2**(-64)),         # (y/2**64)*2**-64*2**64
            x_max_func    = lambda y: int(y*2**(256-64)),      # (y/2**64)*2**(256-64)*2**64
            y_min_func    = lambda x: int(x/2**(256-64)),      # x/2**64/2**(256-64)*2**64
            y_max_func    = lambda x: int(x/2**(-64)),         # x/2**64/2**(-64)*2**64
        )

    # y positive, small
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 2*2**64,
    #         y_range_start = 2**64,
    #         y_range_end   = 8*2**64,
    #     ),
    #     div_x64,
    #     real_div_x64,
    #     "$x/y$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 2**128*2**64,
    #         y_range_start = 2**64,
    #         y_range_end   = 8*2**64,
    #     ),
    #     div_x64,
    #     real_div_x64,
    #     "$x/y$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 2**(256-64-63)*2**64,
    #         y_range_start = 2**64,
    #         y_range_end   = 8*2**64,
    #     ),
    #     div_x64,
    #     real_div_x64,
    #     "$x/y$",
    #     show_plots=True,
    #     save_plots=True
    # )



    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 8*2**64,
    #         y_range_start = 2**60,
    #         y_range_end   = 2**64,
    #     ),
    #     div_x64,
    #     real_div_x64,
    #     "$x/y$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 8*2**64,
    #         y_range_start = 2**52,
    #         y_range_end   = 2**60,
    #     ),
    #     div_x64,
    #     real_div_x64,
    #     "$x/y$",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 0,
    #         x_range_end   = 8*2**64,
    #         y_range_start = 0,
    #         y_range_end   = 2**52,
    #     ),
    #     div_x64,
    #     real_div_x64,
    #     "$x/y$",
    #     show_plots=True,
    #     save_plots=True
    # )



def test_eval_pow():

    # Points of interest

    x_points_of_interest = remove_duplicates_and_sort([
        0,
        1,
        2**64,
        *get_powers_of_2_x64(-64, 256-64, 4),
        *get_powers_of_2_minus_1_x64(-64, 256-64+1, 4),
        2**(256-64)*2**64-1
    ])
    y_points_of_interest = x_points_of_interest.copy()

    x_points_of_interest, y_points_of_interest = get_all_combinations(x_points_of_interest, y_points_of_interest)

    # Separate expected valid and invalid inputs to better verify correct operation of the implementation
    def filter_func(x: int, y: int) -> bool:
        if x < 2**64: return False

        expected_val = real_pow_x64(x, y)
        # ! This condition excludes all points that SHOULD fail, however, the pow implementation returns 'valid' outputs
        # ! for inputs which should fail => e.g. x = 2**12 * 2**64, y = 2**4 * 2**16 => output is exactly 2**192 * 2**64
        # ! which is out of the U256_x64 range. For this input, the implementation returns a number which is slightly less
        # ! than the true one, hence fitting in a U256_x64 number
        # ! To test for this when running the valid and invalid tests, just change the '<' sign to '<=' in the expression below
        return expected_val is not None and expected_val < 2**256

    (x_points_valid, y_points_valid), (x_points_invalid, y_points_invalid) = filter_lists(
        x_points_of_interest,
        y_points_of_interest,
        filter_func
    )

    # evaluate_implementation_2_vars(
    #     (x_points_valid, y_points_valid),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     show_plots=False,
    #     save_plots=False
    # )

    # evaluate_implementation_2_vars(
    #     (x_points_invalid, y_points_invalid),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     show_plots=True,
    #     save_plots=False
    # )

    def generate_samples(
        sample_count  : int,
        x_range_start : int,
        x_range_end   : int,
        p_range_start : int,
        p_range_end   : int
    ) -> Tuple[List[int], List[int]]:

        return sample_2d_space(
            sample_count  = sample_count,
            x_range_start = x_range_start,
            x_range_end   = x_range_end,
            y_range_start = p_range_start,
            y_range_end   = p_range_end,
            x_max_func    = lambda p: int(2**((256-64)/(p/2**64))*2**64),
            y_max_func    = lambda x: int((256-64)/math.log2(x/2**64)*2**64)
        )


    # # Analysis of x for small p
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**25*2**64,
    #         p_range_start = 0,
    #         p_range_end   = 8*2**64,
    #     ),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     y_label="p",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**50*2**64,
    #         p_range_start = 2**64,
    #         p_range_end   = 8*2**64,
    #     ),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     y_label="p",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**(256-64)*2**64,
    #         p_range_start = 2**64,
    #         p_range_end   = 8*2**64,
    #     ),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     y_label="p",
    #     show_plots=True,
    #     save_plots=True
    # )


    # # Analysis of p
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 8*2**64,
    #         p_range_start = 0,
    #         p_range_end   = 2*2**64,
    #     ),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     y_label="p",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 8*2**64,
    #         p_range_start = 0,
    #         p_range_end   = 2**10*2**64,
    #     ),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     y_label="p",
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 8*2**64,
    #         p_range_start = 0,
    #         p_range_end   = 2**36*2**64,
    #     ),
    #     pow_x64,
    #     real_pow_x64,
    #     "$x^p$",
    #     y_label="p",
    #     show_plots=True,
    #     save_plots=True
    # )



def test_eval_inv_pow():

    x_points_of_interest = remove_duplicates_and_sort([
        0,
        1,
        2**64,
        *get_powers_of_2_x64(-64, 256-64, 4),
        *get_powers_of_2_minus_1_x64(-64, 256-64+1, 4),
        2**(256-64)*2**64-1
    ])
    y_points_of_interest = x_points_of_interest.copy()

    x_points_of_interest, y_points_of_interest = get_all_combinations(x_points_of_interest, y_points_of_interest)

    # Separate expected valid and invalid inputs to better verify correct operation of the implementation
    def filter_func(x: int, y: int) -> bool:
        if x < 2**64: return False

        expected_val = real_inv_pow_x64(x, y)
        # ! This condition excludes all points that SHOULD fail, however, the pow implementation returns 'valid' outputs
        # ! for inputs which should fail => e.g. x = 2**12 * 2**64, y = 2**4 * 2**16 => output is exactly 2**192 * 2**64
        # ! which is out of the U256_x64 range. For this input, the implementation returns a number which is slightly less
        # ! than the true one, hence fitting in a U256_x64 number
        # ! To test for this when running the valid and invalid tests, just change the '<' sign to '<=' in the expression below
        return expected_val is not None and expected_val < 2**256 and expected_val > 1048576

    (x_points_valid, y_points_valid), (x_points_invalid, y_points_invalid) = filter_lists(
        x_points_of_interest,
        y_points_of_interest,
        filter_func
    )

    # evaluate_implementation_2_vars(
    #     (x_points_valid, y_points_valid),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     show_plots=True,
    #     save_plots=False
    # )

    # evaluate_implementation_2_vars(
    #     (x_points_invalid, y_points_invalid),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     show_plots=True,
    #     save_plots=False
    # )


    # Range Analysis

    def generate_samples(
        sample_count  : int,
        x_range_start : int,
        x_range_end   : int,
        p_range_start : int,
        p_range_end   : int
    ) -> Tuple[List[int], List[int]]:

        return sample_2d_space(
            sample_count  = sample_count,
            x_range_start = x_range_start,
            x_range_end   = x_range_end,
            y_range_start = p_range_start,
            y_range_end   = p_range_end,
            x_max_func    = lambda p: int((2**( min(64/(p/2**64), (256-64)) ))*2**64), # min used to clip values, as otherwise, for very small p, the power grows to infinity
            y_max_func    = lambda x: int((64/math.log2(x/(2**64)))*2**64)
        )


    # General Overview - medium x, large p
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         20000,
    #         x_range_start = 2**64,
    #         x_range_end   = 50*2**64, 
    #         p_range_start = 0,
    #         p_range_end   = 2**31*2**64,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )



    # Analysis of x for very small p

    # # Medium x
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**64*2**64, 
    #         p_range_start = 0,
    #         p_range_end   = 2**64 + 2**62,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )

    # # Large x
    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**(256-64)*2**64, 
    #         p_range_start = 0,
    #         p_range_end   = 2**64 + 2**62,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )



    # Analysis of p for very small x

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**64 + 2**35,
    #         p_range_start = 0,
    #         p_range_end   = 2**31*2**64,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**64 + 2**35,
    #         p_range_start = 0,
    #         p_range_end   = 2**35*2**64,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         10000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**64 + 2**35,
    #         p_range_start = 0,
    #         p_range_end   = 2**40*2**64,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )

    # evaluate_implementation_2_vars(
    #     generate_samples(
    #         5000,
    #         x_range_start = 2**64,
    #         x_range_end   = 2**64 + 2**35,
    #         p_range_start = 0,
    #         p_range_end   = 2**60*2**64,
    #     ),
    #     inv_pow_x64,
    #     real_inv_pow_x64,
    #     "$x^{-p}$",
    #     y_label="p",
    #     plot_azim=30,
    #     plot_elev=30,
    #     show_plots=True,
    #     save_plots=True
    # )




# Functions of 3 variables ******************************************************************************************************

# def test_eval_safe_pow_x64():
#     pass