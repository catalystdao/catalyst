import pytest
from brownie import convert

def relative_error(a, b):
    if a is None or b is None:
        return None

    if a == 0 and b == 0:
        return 0

    return 2*(a - b)/(abs(a) + abs(b))


def assert_relative_error(a, b, neg_error_bound, pos_error_bound, error_id=None):
    error = relative_error(a, b)
    if not (neg_error_bound <= error <= pos_error_bound):
        pytest.fail(
            f"RELATIVE ERROR{'' if error_id is None else ' '+error_id}: Error {error} is outside allowed range [{neg_error_bound}, {pos_error_bound}]",
            pytrace=False   # Do not print the python traceback
        )


def assert_abs_relative_error(a, b, error_bound, error_id=None):
    error = abs(relative_error(a, b))
    if error > error_bound:
        pytest.fail(
            f"RELATIVE ERROR{'' if error_id is None else ' '+error_id}: Absolute error {error} is larger than the maximum allowed {error_bound}",
            pytrace=False   # Do not print the python traceback
        )


def convert_64_bytes_address(address):
    return convert.to_bytes(20, "bytes1")+convert.to_bytes(0)+convert.to_bytes(address.replace("0x", ""))