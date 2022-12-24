
#[cfg(test)]
mod test_pow2 {
    use std::ops::{Mul, Sub};

    use rug::Float;
    use rug::ops::Pow;

    use crate::u256::U256;
    use crate::test::test_common::test_common::*;
    use crate::fixed_point_math_x64::*;



    // Test power of 2: pow2_x64 ************************************************************************************************

    // Set test bounds
    const POW2_MAX_ABS_ERROR_BOUND: f64 = 1e-5;
    const POW2_AVG_ABS_ERROR_BOUND: f64 = 1e-5;
    
    /// Compute accurately the power of 2 of a number using floating point numbers
    pub fn target_pow2_x64(a: U256) -> Result<U256, String> {

        // Directly return an error if 'a' is too large and will cause the output to overflow (avoid calculation)
        // Output will overflow for a >= (256 - 64)*2**64, but allow some extra margin for possible rounding errors
        if a > U256::from( 256 - 64 + 1 ) * ONE_X64 {
            return Err("Can't compute pow2: overflow.".to_string())
        }

        // Compute pow2 using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let out = Float::with_val(256, 2).pow(a);

        high_precision_float_to_uint_x64(out)
    }

    /// Test pow2_x64 for a set of interest points
    #[test]
    fn test_pow2_poi() -> Result<(), ()> {

        let max_theoretical_input = (U256::from(256 - 64) * ONE_X64) - 1;

        let mut points_of_interest_x64 = vec![
            ZERO_X64,
            ONE_X64,
            max_theoretical_input,      // Should work
            max_theoretical_input + 1,  // Should fail
            U256_MAX
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64_i64, 10, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64_i64, 10, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        let result = evaluate_impl(
            |a: &U256| pow2_x64(*a),
            |a: &U256| target_pow2_x64(*a).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow2_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= POW2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// Test pow2_x64 for a random set of numbers
    #[test]
    fn test_pow2_randrange() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ZERO_X64, 
            ONE_X64.mul(U256::from(256)),   // Note values from 256-64 to 256 should fail
            2000000_usize
        );

        let result = evaluate_impl(
            |a: &U256| pow2_x64(*a),
            |a: &U256| target_pow2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\npow2_x64 - Randrange [ 0, 256_x64 )\n{}", result);

        assert!(result.max_abs_error <= POW2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

    /// Test pow2_x64 for a random set of numbers (all expected to generate invalid outputs)
    #[test]
    fn test_pow2_randrange_invalid() -> Result<(), ()> {

        // All values should fail
        let rand_sample_x64 = sample_space(
            ONE_X64.mul(U256::from(256)), 
            U256_MAX,   
            2000000_usize
        );

        let result = evaluate_impl(
            |a: &U256| pow2_x64(*a),
            |a: &U256| target_pow2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\npow2_x64 - Randrange [ 256_x64, max_x64 )\n{}", result);

        assert!(result.max_abs_error <= POW2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }



    // Test inv_pow2_x64 ********************************************************************************************************

    // Set test bounds
    const INV_POW2_MAX_ABS_ERROR_BOUND: f64 = 1e-5;
    const INV_POW2_AVG_ABS_ERROR_BOUND: f64 = 1e-5;
    
    /// Compute accurately the inverse power of 2 of a number using floating point numbers
    pub fn target_inv_pow2_x64(a: U256) -> Result<U256, String> {

        // Fail for powers larger than 41_x64 // ! This is specific to the implementation used
        if a >= U256([0, 41, 0, 0]) { return Err("".to_owned()) }

        // Compute inv_pow2 using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let out = Float::with_val(256, 2).pow(-a);

        high_precision_float_to_uint_x64(out)
    }

    /// Test inv_pow2_x64 for a set of interest points
    #[test]
    fn test_inv_pow2_poi() -> Result<(), ()> {

        let max_input = U256::from(41).mul(ONE_X64).sub(U256::one());   // 41*2**64 - 1

        let mut points_of_interest_x64 = vec![
            ZERO_X64,
            ONE_X64,
            max_input,
            max_input + 1   // Should fail
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64_i64, 10, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64_i64, 10, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        let result = evaluate_impl(
            |a: &U256| inv_pow2_x64(*a),
            |a: &U256| target_inv_pow2_x64(*a).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ninv_pow2_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= INV_POW2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// Test inv_pow2_x64 for a random set of numbers
    #[test]
    fn test_inv_pow2_randrange() -> Result<(), ()> {

        let rand_sample_x64 = sample_space(
            ZERO_X64, 
            ONE_X64.mul(U256::from(55)),   // Note values larger than 41_x64 should fail
            2000000_usize
        );

        let result = evaluate_impl(
            |a: &U256| inv_pow2_x64(*a),
            |a: &U256| target_inv_pow2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\ninv_pow2_x64 - Randrange [ 0, 55_x64 )\n{}", result);

        assert!(result.max_abs_error <= INV_POW2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

    /// Test inv_pow2_x64 for a random set of numbers (all expected to generate invalid outputs)
    #[test]
    fn test_inv_pow2_randrange_invalid() -> Result<(), ()> {

        // All values should fail
        let rand_sample_x64 = sample_space(
            ONE_X64.mul(U256::from(55)), 
            U256_MAX,   
            2000000_usize
        );

        let result = evaluate_impl(
            |a: &U256| inv_pow2_x64(*a),
            |a: &U256| target_inv_pow2_x64(*a).map_err(|_err| ()),
            rand_sample_x64
        );

        println!("\ninv_pow2_x64 - Randrange [ 55_x64, max_x64 )\n{}", result);

        assert!(result.max_abs_error <= INV_POW2_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW2_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

}