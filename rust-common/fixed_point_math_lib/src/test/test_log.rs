
#[cfg(test)]
mod test_log {

    use crate::u256::U256;
    use crate::test::test_common::test_common::*;
    use crate::fixed_point_math_x64::*;



    // Test log2_x64 ************************************************************************************************************

    // Set test bounds
    const LOG2_MAX_ABS_ERROR_BOUND: f64 = 1e-6;
    const LOG2_AVG_ABS_ERROR_BOUND: f64 = 1e-6;

    /// Compute accurately the log2 of a number using floating point numbers
    pub fn target_log2_x64(a: U256) -> Result<U256, String> {
        
        if a < ONE_X64 {
            return Err("Can't compute log2 of a value lower than 1 (negative output).".to_string())
        }

        // Compute log2 using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let out = a.log2();
        high_precision_float_to_uint_x64(out)
    }

    /// Test log2_x64 for a set of interest points
    #[test]
    fn test_log2_poi() -> Result<(), ()> {

        let min_input = ONE_X64;
        let max_input = U256_MAX;

        let mut points_of_interest_x64 = vec![
            ZERO_X64,       // Must fail
            U256::one(),    // Must fail
            min_input - 1,  // Must fail
            min_input,
            max_input,
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64_i64, 0, 1));             // Must fail
        points_of_interest_x64.append(&mut get_powers_of_2_x64(0, 256-64, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64_i64, 1, 1));     // Must fail
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(1, 256-64+1, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        let result = evaluate_impl(
            |a: &U256| log2_x64(*a),
            |a: &U256| target_log2_x64(*a).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nlog2_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= LOG2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LOG2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// Test log2_x64 for a random set of numbers (all expected to generate valid outputs)
    #[test]
    fn test_log2_randrange_all_valid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64, 
            U256_MAX,
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| log2_x64(*a),
            |a: &U256| target_log2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nlog2_x64 - Randrange [ 1_x64, MAX_x64 )\n{}", result);

        assert!(result.max_abs_error <= LOG2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LOG2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }

    /// Test log2_x64 for a random set of numbers (all expected to generate invalid outputs)
    #[test]
    fn test_log2_randrange_all_invalid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            U256::zero(), 
            ONE_X64,
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| log2_x64(*a),
            |a: &U256| target_log2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nlog2_x64 - Randrange [ 0, 1_x64 )\n{}", result);

        assert!(result.max_abs_error <= LOG2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LOG2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_some_count == 0);

        Ok(())
    }

    /// Test log2_x64 for a random set of numbers close to 1_x64
    #[test]
    fn test_log2_randrange_subrange_1() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64 + U256::from(2_u64.pow(54_u32)), 
            ONE_X64 * 2,
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| log2_x64(*a),
            |a: &U256| target_log2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nlog2_x64 - Randrange [ 1_x64 + 2^(-10)_x64, 2_x64 )\n{}", result);

        assert!(result.max_abs_error <= LOG2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LOG2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }


    /// Test log2_x64 for a random set of numbers close to 1_x64
    #[test]
    fn test_log2_randrange_subrange_2() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64 + U256::from(2_u64.pow(37_u32)),
            ONE_X64 + U256::from(2_u64.pow(54_u32)), 
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| log2_x64(*a),
            |a: &U256| target_log2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nlog2_x64 - Randrange [ 1_x64 + 2^(-27)_x64, 1_x64 + 2^(-10)_x64 )\n{}", result);

        assert!(result.max_abs_error <= 0.1); // ! Error increases very rapidly for values close to 1
        assert!(result.avg_abs_error <= 0.0001);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }


    /// Test log2_x64 for a random set of numbers close to 1_x64
    #[test]
    fn test_log2_randrange_subrange_3() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64,
            ONE_X64 + U256::from(2_u64.pow(37_u32)),
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| log2_x64(*a),
            |a: &U256| target_log2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nlog2_x64 - Randrange [1_x64, 1_x64 + 2^(-27)_x64)\n{}", result);

        // ! Not checking for errors, as these are very large (see printed output)
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }



    // Test ln_x64 **************************************************************************************************************

    // Set test bounds
    const LN_MAX_ABS_ERROR_BOUND: f64 = 1e-6;
    const LN_AVG_ABS_ERROR_BOUND: f64 = 1e-6;

    /// Compute accurately the ln of a number using floating point numbers
    pub fn target_ln_x64(a: U256) -> Result<U256, String> {

        if a < ONE_X64 {
            return Err("Can't compute ln of a value lower than 1 (negative output).".to_string())
        }

        // Compute log2 using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let out = a.ln();
        high_precision_float_to_uint_x64(out)
    }

    /// Test ln_x64 for a set of interest points
    #[test]
    fn test_ln_poi() -> Result<(), ()> {

        let min_input = ONE_X64;
        let max_input = U256_MAX;

        let mut points_of_interest_x64 = vec![
            ZERO_X64,       // Must fail
            U256::one(),    // Must fail
            min_input - 1,  // Must fail
            min_input,
            max_input,
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64_i64, 0, 1));             // Must fail
        points_of_interest_x64.append(&mut get_powers_of_2_x64(0, 256-64, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64_i64, 1, 1));     // Must fail
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(1, 256-64+1, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        let result = evaluate_impl(
            |a: &U256| ln_x64(*a),
            |a: &U256| target_ln_x64(*a).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nln_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= LN_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LN_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// Test ln_x64 for a random set of numbers (all expected to generate valid outputs)
    #[test]
    fn test_ln_randrange_all_valid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64, 
            U256_MAX,
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| ln_x64(*a),
            |a: &U256| target_ln_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nln_x64 - Randrange [1_x64, MAX_x64)\n{}", result);

        assert!(result.max_abs_error <= LN_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LN_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }

    /// Test ln_x64 for a random set of numbers (all expected to generate invalid outputs)
    #[test]
    fn test_ln_randrange_all_invalid() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            U256::zero(), 
            ONE_X64,
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| ln_x64(*a),
            |a: &U256| target_ln_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nln_x64 - Randrange [ 0, 1_x64 )\n{}", result);

        assert!(result.max_abs_error <= LN_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LN_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_some_count == 0);

        Ok(())
    }

    /// Test ln_x64 for a random set of numbers close to 1_x64
    #[test]
    fn test_ln_randrange_subrange_1() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64 + U256::from(2_u64.pow(54_u32)), 
            ONE_X64 * 2,
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| ln_x64(*a),
            |a: &U256| target_ln_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nln_x64 - Randrange [ 1_x64 + 2^(-10)_x64, 2_x64 )\n{}", result);

        assert!(result.max_abs_error <= LN_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= LN_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }

    /// Test ln_x64 for a random set of numbers close to 1_x64
    #[test]
    fn test_ln_randrange_subrange_2() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64 + U256::from(2_u64.pow(37_u32)),
            ONE_X64 + U256::from(2_u64.pow(54_u32)), 
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| ln_x64(*a),
            |a: &U256| target_ln_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nln_x64 - Randrange [ 1_x64 + 2^(-27)_x64, 1_x64 + 2^(-10)_x64 )\n{}", result);

        assert!(result.max_abs_error <= 0.1); // ! Error increases very rapidly for values close to 1
        assert!(result.avg_abs_error <= 0.0001);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }

    /// Test ln_x64 for a random set of numbers close to 1_x64
    #[test]
    fn test_ln_randrange_subrange_3() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ONE_X64,
            ONE_X64 + U256::from(2_u64.pow(37_u32)),
            200000_usize
        );

        let result = evaluate_impl(
            |a: &U256| ln_x64(*a),
            |a: &U256| target_ln_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\nln_x64 - Randrange [1_x64, 1_x64 + 2^(-27)_x64)\n{}", result);

        // ! Not checking for errors, as these are very large (see printed output)
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())
    }

}