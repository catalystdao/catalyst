from anchorpy import Program
from hypothesis import given, strategies as st

from solana.publickey import PublicKey
from utils import U256_MAX, verify_program_result

# Test the Solana fixed point mathematical library against the mathematical library of the Catalyst simulator

# ! NOTE that the following tests do not check the accuracy of the numerical methods, but rather they just
# ! match the values against the fixed point mathematical library of the simulator.

# ! TODO
# ! Split test functions into valid/invalid ranges ==> Allow hypothesis to better test edge cases


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    b = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_mul(math_program: Program, result_account: PublicKey, a: int, b: int):

    await verify_program_result('mul_x64', [a, b], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    b = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_div(math_program: Program, result_account: PublicKey, a: int, b: int):

    await verify_program_result('div_x64', [a, b], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_log2(math_program: Program, result_account: PublicKey, a: int):

    await verify_program_result('log2_x64', [a], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_ln(math_program: Program, result_account: PublicKey, a: int):

    await verify_program_result('ln_x64', [a], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_pow2(math_program: Program, result_account: PublicKey, a: int):

    await verify_program_result('pow2_x64', [a], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_inv_pow2(math_program: Program, result_account: PublicKey, a: int):

    await verify_program_result('inv_pow2_x64', [a], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    p = st.integers(min_value=0, max_value=U256_MAX),
)
async def test_pow(math_program: Program, result_account: PublicKey, a: int, p: int):

    await verify_program_result('pow_x64', [a, p], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX),
    p = st.integers(min_value=0, max_value=U256_MAX),
)
async def test_inv_pow(math_program: Program, result_account: PublicKey, a: int, p: int):

    await verify_program_result('inv_pow_x64', [a, p], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_exp(math_program: Program, result_account: PublicKey, a: int):

    await verify_program_result('exp_x64', [a], math_program, result_account)


@given(
    a = st.integers(min_value=0, max_value=U256_MAX)
)
async def test_inv_exp(math_program: Program, result_account: PublicKey, a: int):

    await verify_program_result('inv_exp_x64', [a], math_program, result_account)


# ! TODO
# def test_safe_pow_x64():
#     raise Exception("Test not implemented")