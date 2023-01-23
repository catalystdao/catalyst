from tests.catalyst.utils.common_utils import assert_abs_relative_error


def test_security_limit_init(pool_1, get_pool_1_max_unit_inflow, pool_2, get_pool_2_max_unit_inflow):
    """
        Make sure the security limit gets correctly initialized
    """

    expected_source_max_capacity = get_pool_1_max_unit_inflow()
    observed_source_max_capacity = pool_1._max_unit_inflow()

    assert_abs_relative_error(observed_source_max_capacity, expected_source_max_capacity, 1e-10)
    assert observed_source_max_capacity == pool_1.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity


    expected_target_max_capacity = get_pool_2_max_unit_inflow()
    observed_target_max_capacity = pool_2._max_unit_inflow()

    assert_abs_relative_error(observed_target_max_capacity, expected_target_max_capacity, 1e-10)
    assert observed_target_max_capacity == pool_2.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity

