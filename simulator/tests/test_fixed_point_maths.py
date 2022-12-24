from hypothesis import given, strategies as st

from tests.utils.helper_utils import verify_numerical_calculation
from utils.math_utils import real_div_x64, real_exp_x64, real_inv_exp_x64, real_inv_pow2_x64, real_inv_pow_x64, real_ln_x64, real_log2_x64, real_mul_x64, real_pow2_x64, real_pow_x64


# ! TODO
# ! The current tests implementation defines the valid range of the math functions within each test. Each argument is then 
# ! tested for the full range of Uint256 values.
# ! This does not allow hypothesis to test for edge cases, as hypothesis has no information on the valid range of inputs for 
# ! each function.

# !     ==> Separate each test function into two: one for the valid range of inputs, and one for the invalid range of inputs


U256_MAX = 2**256-1


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    b = st.integers(min_value=0, max_value=U256_MAX)
)
def test_mul(a: int, b: int):

    expected_result = real_mul_x64(a, b)
    expected_valid  = expected_result <= U256_MAX
    max_rel_error   = 0

    verify_numerical_calculation('mul_x64', [a, b], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    b = st.integers(min_value=0, max_value=U256_MAX)
)
def test_div(a: int, b: int):

    expected_result = real_div_x64(a, b)
    expected_valid  = b != 0 and a < 2**(256-64) and expected_result is not None and expected_result <= U256_MAX
    max_rel_error   = 0

    verify_numerical_calculation('div_x64', [a, b], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
def test_log2(a: int):
    expected_result = real_log2_x64(a)
    expected_valid  = a >= 2**64        # Input must be larger or equal to one (else result is a negative value)
    max_rel_error   = 1e-10

    verify_numerical_calculation('log2_x64', [a], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
def test_ln(a: int):
    expected_result = real_ln_x64(a)
    expected_valid  = a >= 2**64
    max_rel_error   = 1e-10

    verify_numerical_calculation('ln_x64', [a], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
def test_pow2(a: int):
    expected_result = real_pow2_x64(a)
    expected_valid  = a < 192*2**64
    max_rel_error   = 1e-5

    verify_numerical_calculation('pow2_x64', [a], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
def test_inv_pow2(a: int):
    expected_result = real_inv_pow2_x64(a)
    expected_valid  = (a >> 64) < 41
    max_rel_error   = 1e-5

    verify_numerical_calculation('inv_pow2_x64', [a], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    p = st.integers(min_value=0, max_value=150*2**64),
)
def test_pow(a: int, p: int):
    expected_result = real_pow_x64(a, p)
    expected_valid  = a >= 2**64 and (real_mul_x64(real_log2_x64(a), p) >> 64) < 192 and expected_result <= U256_MAX  #type: ignore
    max_rel_error   = 1e-5

    verify_numerical_calculation('pow_x64', [a, p], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    p = st.integers(min_value=0, max_value=150*2**64),
)
def test_inv_pow(a: int, p: int):
    expected_result = real_inv_pow_x64(a, p)
    expected_valid  = a >= 2**64 and (real_mul_x64(real_log2_x64(a), p) >> 64) < 41 and expected_result <= U256_MAX  #type: ignore
    max_rel_error   = 1e-5

    verify_numerical_calculation('inv_pow_x64', [a, p], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
def test_exp(a: int):
    expected_result = real_exp_x64(a)
    expected_valid  = a <= 2454971259878909673472   # a <= int(ln(U256_x64.max >> 64) * 2**64)
    max_rel_error   = 1e-5
    verify_numerical_calculation('exp_x64', [a], expected_result if expected_valid else None, max_rel_error)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
def test_inv_exp(a: int):
    expected_result = real_inv_exp_x64(a)
    expected_valid  = a <= 2**(64+5)   # TODO there should be no limit ==> document behavior
    max_rel_error   = 1e-6
    verify_numerical_calculation('inv_exp_x64', [a], expected_result if expected_valid else None, max_rel_error)


# def test_safe_pow_x64():
#     raise Exception("Test not implemented")