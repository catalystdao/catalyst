from tests.catalyst.utils.common_utils import assert_abs_relative_error


def test_security_limit_init(pool, get_pool_max_unit_inflow, get_pool_amp):
    """
        Make sure the security limit gets correctly initialized
    """

    expected_source_max_capacity = get_pool_max_unit_inflow()
    observed_source_max_capacity = pool._maxUnitCapacity()

    assert_abs_relative_error(observed_source_max_capacity, expected_source_max_capacity, 1e-10)
    
    if get_pool_amp() != 10**18:
        assert observed_source_max_capacity/2 == pool.getUnitCapacity() # Since there have been no swaps, max capacity == current capacity
        return
    
    assert observed_source_max_capacity == pool.getUnitCapacity()     # Since there have been no swaps, max capacity == current capacity
    
