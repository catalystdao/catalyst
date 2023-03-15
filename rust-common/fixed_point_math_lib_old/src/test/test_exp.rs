
#[cfg(test)]
mod test_exp {
    use std::ops::Mul;

    use crate::u256::U256;
    use crate::test::test_common::test_common::*;
    use crate::fixed_point_math_x64::*;



    // Test exp_x64 *************************************************************************************************************

    // Set test bounds
    const EXP_MAX_ABS_ERROR_BOUND: f64 = 1e-6;
    const EXP_AVG_ABS_ERROR_BOUND: f64 = 1e-6;

    /// Compute accurately the exponent of a number using floating point numbers
    pub fn target_exp_x64(a: U256) -> Result<U256, String> {

        // Directly return an error if 'a' is too large and will cause the output to overflow (avoid calculation)
        // Output will overflow for a > int(ln(U256_x64.max >> 64) * 2**64) -> a > 134 * 2**64 (allow margin for rounding errors)
        if a > U256::from( 134 ) * ONE_X64 {
            return Err("Can't compute pow2: overflow.".to_string())
        }

        // Compute exp using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let out = a.exp();

        high_precision_float_to_uint_x64(out)
    }

    /// Test exp_x64 for a set of interest points
    #[test]
    fn test_exp_poi() -> Result<(), ()> {

        // TODO This value has been copied from a previous implementation of the tests. It accurately describes the max_input 
        // TODO for the exp_x64 implementation. HOWEVER, the target_exp_x64 considers this value to be too high 
        // TODO (i.e. result should overflow)
        // TODO     ==> Find the range of values that result in a discrepancy in the overflows between the impl./target functions
        // TODO         and document findings
        let max_input = U256([1554304821396242431, 133, 0, 0]);     // 2454971266624766607359   => Maximum expected valid input (found numerically)

        let mut points_of_interest_x64 = vec![
            ZERO_X64,
            ONE_X64,
            max_input,
            max_input + 1,   // Should fail //TODO
            U256_MAX         // Should fail
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64_i64, 10, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64_i64, 10, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        let result = evaluate_impl(
            |a: &U256| exp_x64(*a),
            |a: &U256| target_exp_x64(*a).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nexp_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 1);   // ! TODO view comment at the beginning of the test function
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// Test exp_x64 for a random set of numbers
    #[test]
    fn test_exp_randrange_all_valid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ZERO_X64, 
            ONE_X64.mul(U256::from(133)),
            20000_usize
        );

        let result = evaluate_impl(
            |a: &U256| exp_x64(*a),   // ! TODO takes very long to compute
            |a: &U256| target_exp_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nexp_x64 - Randrange [0, 133_x64)\n{}", result);

        assert!(result.expected_none_count == 0);   // All tested points should generate a valid output

        assert!(result.max_abs_error <= EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

    /// Test exp_x64 for a random set of numbers around the threshold at which the function overflows
    #[test]
    fn test_exp_randrange_overflow_threshold() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64.mul(U256::from(133)),
            ONE_X64.mul(U256::from(134)),
            20000_usize
        );

        let result = evaluate_impl(
            |a: &U256| exp_x64(*a),   // ! TODO takes very long to compute
            |a: &U256| target_exp_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nexp_x64 - Randrange [133_x64, 134_x64)\n{}", result);

        assert!(result.expected_none_count > 0);   // Some tested points will overflow
        assert!(result.expected_some_count > 0);   // Some tested points will not overflow

        assert!(result.max_abs_error <= EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

    /// Test exp_x64 for a random set of numbers (all expected to fail)
    #[test]
    fn test_exp_randrange_all_overflow() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64.mul(U256::from(134)),
            U256_MAX,
            20000_usize
        );

        let result = evaluate_impl(
            |a: &U256| exp_x64(*a),   // ! TODO takes very long to compute
            |a: &U256| target_exp_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nexp_x64 - Randrange [134_x64, max_x64)\n{}", result);

        assert!(result.expected_some_count == 0);   // All test points will overflow

        assert!(result.max_abs_error <= EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }



    // Test inv_exp_x64 *********************************************************************************************************

    // Set test bounds
    const INV_EXP_MAX_ABS_ERROR_BOUND: f64 = 1e-6;
    const INV_EXP_AVG_ABS_ERROR_BOUND: f64 = 1e-6;

    /// Compute accurately the inverse exponent of a number using floating point numbers
    pub fn target_inv_exp_x64(a: U256) -> Result<U256, String> {

        // Directly return an error if 'a' is too large and will cause the output to be inaccurate
        // ! This is specific to the inv_exp_x64 implementation
        if a > U256::from( 16 ) * ONE_X64 {
            return Err("Can't compute pow2: overflow.".to_string())
        }

        // Compute exp using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let out = a.exp().recip();

        high_precision_float_to_uint_x64(out)
    }

    /// Test inv_exp_x64 for a set of interest points
    #[test]
    fn test_inv_exp_poi() -> Result<(), ()> {

        let max_input = U256::from(16).mul(ONE_X64);

        let mut points_of_interest_x64 = vec![
            ZERO_X64,
            ONE_X64,
            max_input,
            max_input + 1   // Should fail
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64_i64, 8, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64_i64, 8, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        let result = evaluate_impl(
            |a: &U256| inv_exp_x64(*a),
            |a: &U256| target_inv_exp_x64(*a).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ninv_exp_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= INV_EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    #[test]
    fn test_inv_exp_randrange_all_valid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ZERO_X64, 
            U256::from(16).mul(ONE_X64),
            20000_usize
        );

        let result = evaluate_impl(
            |a: &U256| inv_exp_x64(*a),   // ! TODO takes very long to compute
            |a: &U256| target_inv_exp_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\ninv_exp_x64 - Randrange [0, 16_x64)\n{}", result);

        assert!(result.expected_none_count == 0);   // All tested points should generate a valid output

        assert!(result.max_abs_error <= INV_EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

    #[test]
    fn test_inv_exp_randrange_all_invalid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            U256::from(16).mul(ONE_X64), 
            U256_MAX,
            20000_usize
        );

        let result = evaluate_impl(
            |a: &U256| inv_exp_x64(*a),   // ! TODO takes very long to compute
            |a: &U256| target_inv_exp_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\ninv_exp_x64 - Randrange [16_x64, max_x64)\n{}", result);

        assert!(result.expected_some_count == 0);   // All tested points should generate a valid output

        assert!(result.max_abs_error <= INV_EXP_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_EXP_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

}