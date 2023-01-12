
def relative_error(val, target):

    if val is None or target is None:
        return None
    
    if val == 0 and target == 0:
        return 0

    return 2*(val - target)/(abs(val) + abs(target))


def assert_relative_error(val, target, low_error_bound, high_error_bound, error_id=None):
    error = relative_error(val, target)
    error_id_string = f"(ERR: {error_id})" if error_id is not None else ""
    assert low_error_bound <= error <= high_error_bound, f"Error {error} is outside allowed range [{low_error_bound}, {high_error_bound}] {error_id_string}"


def get_expected_decayed_units_capacity(
    ref_capacity,
    ref_capacity_timestamp,
    change_timestamp,
    change_capacity_delta,
    current_timestamp,
    max_capacity,
    decayrate=24*60*60
):
    # Since the units capacity is time dependant, it must be taken into account two changes:
    #   - The capacity change since the ref_capacity value was taken until the capacity was modified by a transaction (the change_timestamp and change_capacity_delta)
    #   - The capacity change since the transaction until now

    # Compute the capacity at the time of the change
    ref_capacity_at_change = min(max_capacity, ref_capacity + int(max_capacity*(change_timestamp - ref_capacity_timestamp)/decayrate))

    # Compute the capacity after the change
    change_capacity = max(0, min(max_capacity, ref_capacity_at_change + change_capacity_delta))

    # Compute the capacity at the current time
    return min(max_capacity, change_capacity + int(max_capacity*(current_timestamp - change_timestamp)/decayrate))
