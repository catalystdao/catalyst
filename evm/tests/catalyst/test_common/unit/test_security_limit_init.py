from tests.catalyst.utils.common_utils import assert_abs_relative_error


def test_security_limit_init(source_pool, get_source_pool_max_unit_inflow, target_pool, get_target_pool_max_unit_inflow):
    """
        Make sure the security limit gets correctly initialized
    """

    expected_source_max_capacity = get_source_pool_max_unit_inflow()
    observed_source_max_capacity = source_pool._max_unit_inflow()

    assert_abs_relative_error(observed_source_max_capacity, expected_source_max_capacity, 1e-10)
    assert observed_source_max_capacity == source_pool.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity


    expected_target_max_capacity = get_target_pool_max_unit_inflow()
    observed_target_max_capacity = target_pool._max_unit_inflow()

    assert_abs_relative_error(observed_target_max_capacity, expected_target_max_capacity, 1e-10)
    assert observed_target_max_capacity == target_pool.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity

