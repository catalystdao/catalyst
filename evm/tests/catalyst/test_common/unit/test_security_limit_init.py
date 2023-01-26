from tests.catalyst.utils.common_utils import assert_abs_relative_error


def test_security_limit_init(pool, get_pool_max_unit_inflow):
    """
        Make sure the security limit gets correctly initialized
    """

    expected_source_max_capacity = get_pool_max_unit_inflow()
    observed_source_max_capacity = pool._max_unit_inflow()

    assert_abs_relative_error(observed_source_max_capacity, expected_source_max_capacity, 1e-10)
    assert observed_source_max_capacity == pool.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity
