from math import inf
import fixed_point_math
from integer import Uint256

def verify_numerical_calculation(target_fn_name: str, args: list[int], expected_result: int | None, max_error: float):
    target_fn = getattr(fixed_point_math, target_fn_name)

    try:
        result = target_fn(*[Uint256(arg) for arg in args]).value
    except:
        result = None

    if result is None:
        assert expected_result is None, f"Function failed for an expected valid output {expected_result}"
    
    else:
        assert expected_result is not None, f"Function did not fail for an expected invalid output, output: {result}"

        error = compute_relative_error(result, expected_result)

        assert error <= max_error, f"Function output {result} does not match the expected result {expected_result}, error {error}"


def compute_relative_error(value: int, target: int) -> float:
    if target == 0:
        return 0 if value == target else inf
    
    return abs(1 - value/target)