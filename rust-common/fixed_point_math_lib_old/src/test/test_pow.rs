#[cfg(test)]
mod test_pow {

    use std::ops::{Shl, Shr};

    use itertools::Itertools;
    use rug::ops::Pow;

    use crate::u256::U256;
    use crate::test::test_common::test_common::*;
    use crate::fixed_point_math_x64::*;



    // Test pow_x64 ************************************************************************************************************

    // Set test bounds
    const POW_MAX_ABS_ERROR_BOUND: f64 = 1e-5;
    const POW_AVG_ABS_ERROR_BOUND: f64 = 1e-6;

    /// Compute accurately the power function (a^b) using floating point numbers
    pub fn target_pow_x64(a: U256, b: U256) -> Result<U256, String> {

        // ! a must be larger than 1. This is specific to the pow_x64 implementation.
        if a < ONE_X64 {
            return Err("Can't compute inv_pow for x < 1.".to_string())
        }

        // Compute pow using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let b = uint_x64_to_high_precision_float(&b);
        let out = a.pow(b);

        high_precision_float_to_uint_x64(out)
    }

    /// Test pow_x64 for a set of interest points
    #[test]
    fn test_pow_poi() -> Result<(), ()> {

        let mut points_of_interest_x64 = vec![
            ZERO_X64,
            U256::one(),
            ONE_X64,
            U256_MAX
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64, 256 - 64, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64, 256 - 64 + 1, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        // Get all permutations between the elements of points_of_interest_x64
        let points_of_interest_x64: Vec<(&U256, &U256)> = points_of_interest_x64.iter().cartesian_product(points_of_interest_x64.iter()).collect();

        let result = evaluate_impl::<(&U256, &U256)>(
            |(a, b): &(&U256, &U256)| pow_x64(**a, **b),
            |(a, b): &(&U256, &U256)| target_pow_x64(**a, **b).map_err(|_err| ()),
            points_of_interest_x64.clone()
        );

        println!("\npow_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// A helper function to generate a set of random samples from a 2d space within the specified range.
    /// The generated samples should all yield valid outputs.
    fn get_pow_randrange_only_valid(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256
    ) -> Vec<(U256, U256)> {

        // Define the limiting functions: when picked a random value of x, compute the range of values from which to pick y (and vice-versa)
        let x_range_given_y_fn = |y: U256| -> Result<(U256, U256), ()> {

            let y_f = uint_x64_to_float(&y);

            let x_min = x_range_start;

            let x_max_f = 2_f64.powf((256_f64-64_f64)/y_f);    // 2^( (256-64)/y )    (derived from condition x <= 2^(256/p))
            let mut x_max = match float_to_uint_x64(&x_max_f) {
                Ok(val) => val,
                Err(msg) => if msg.contains("overflow") { U256_MAX } else { panic!("Unable to compute range limit for pow function. Unknown error at uint_x64_to_float") }
            };

            if x_max > x_range_end { x_max = x_range_end }
    
            if x_max < x_min { return Err(()); }

            Ok((x_min, x_max))

        };

        let y_range_given_x_fn = |x: U256| -> Result<(U256, U256), ()> {

            let x_f = uint_x64_to_float(&x);

            let y_min = y_range_start;

            let y_max_f = (256_f64 - 64_f64) / x_f.log2();    // (256 - 64)/log2(x)    (derived from condition p <= 256/log2(x))
            let mut y_max = match float_to_uint_x64(&y_max_f) {
                Ok(val) => val,
                Err(msg) => if msg.contains("overflow") { U256_MAX } else { panic!("Unable to compute range limit for pow function. Unknown error at uint_x64_to_float") }
            };

            if y_max > y_range_end { y_max = y_range_end }

            if y_max < y_min { return Err(()); }

            Ok((y_min, y_max))

        };

        sample_2d_space(
            sample_count,
            x_range_start,
            x_range_end,
            y_range_start,
            y_range_end,
            Some(x_range_given_y_fn),
            Some(y_range_given_x_fn),

        )
    }

    /// A helper function to generate a set of random samples from a 2d space within the specified range.
    /// No condition is set on the generated samples (may yield invalid outputs)
    fn get_pow_randrange_all(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256
    ) -> Vec<(U256, U256)> {
        sample_2d_space(
            sample_count,
            x_range_start,
            x_range_end,
            y_range_start,
            y_range_end,
            None::<fn(U256) -> Result<(U256, U256), ()>>,
            None::<fn(U256) -> Result<(U256, U256), ()>>

        )
    }

    /// Test pow_x64 for a random set of numbers - full range (expect all invalid points, as the power function very easily overflows)
    #[test]
    fn test_pow_randrange_all() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_all(
            200000_usize,
            ONE_X64,
            U256_MAX,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1, max_x64), p: [0, max_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// Test pow_x64 for a random set of numbers - full range (expect all invalid points, as the power function very easily overflows)
    #[test]
    fn test_pow_randrange_x_less_than_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_all(
            200000_usize,
            ZERO_X64,
            ONE_X64,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [0, 1_x64), p: [0, max_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }


    /// Test pow_x64 for a random set of numbers - analysis of a large range of x given a small range of y
    #[test]
    fn test_pow_randrange_subrange_small_y_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256::from(2).pow(U256::from(25)) * ONE_X64,
            U256::zero(),
            U256::from(8) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^(25)_x64), p: [0, 8_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }

    #[test]

    /// Test pow_x64 for a random set of numbers - analysis of a large range of x given a small range of y
    fn test_pow_randrange_subrange_small_y_2() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256::from(2).pow(U256::from(50)) * ONE_X64,
            ONE_X64,
            U256::from(8) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^(50)_x64), p: [1_x64, 8_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }


    /// Test pow_x64 for a random set of numbers - analysis of the full range of x given a small range of y
    #[test]
    fn test_pow_randrange_subrange_small_y_3() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_only_valid(
            2000_usize,
            ONE_X64,
            U256_MAX,
            ONE_X64,
            U256::from(8) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^(256-64)_x64), p: [1_x64, 8_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }



    /// Test pow_x64 for a random set of numbers - analysis of a y given a small range of x
    #[test]
    fn test_pow_randrange_subrange_small_x_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256::from(8) * ONE_X64,
            U256::zero(),
            U256::from(2) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 8_x64), p: [0, 2_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }


    /// Test pow_x64 for a random set of numbers - analysis of a y given a small range of x
    #[test]
    fn test_pow_randrange_subrange_small_x_2() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256::from(8) * ONE_X64,
            U256::zero(),
            U256::from(2).pow(U256::from(10)) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 8_x64), p: [0, 2^10_x64)\n{}", result);

        assert!(result.max_abs_error <= POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }

    /// Test pow_x64 for a random set of numbers - analysis of a y given a small range of x
    #[test]
    fn test_pow_randrange_subrange_small_x_3() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256::from(8) * ONE_X64,
            U256::zero(),
            U256::from(2).pow(U256::from(36)) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 8_x64), p: [0, 2^36_x64)\n{}", result);

        // ! Error bounds not checked as the error grows rapidly for large powers
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }




    // Test inv_pow_x64 ********************************************************************************************************

    // Set test bounds
    const INV_POW_MAX_ABS_ERROR_BOUND: f64 = 1e-5;
    const INV_POW_AVG_ABS_ERROR_BOUND: f64 = 1e-5;

    /// Compute accurately the inverse power function (a^{-b}) using floating point numbers
    pub fn target_inv_pow_x64(a: U256, b: U256) -> Result<U256, String> {

        // ! a must be larger than 1. This is specific to the inv_pow_x64 implementation.
        if a < ONE_X64 {
            return Err("Can't compute inv_pow for x < 1.".to_string())
        }

        // Compute pow using high precision float
        let a = uint_x64_to_high_precision_float(&a);
        let b = uint_x64_to_high_precision_float(&b);
        let out = a.pow(b).recip();

        high_precision_float_to_uint_x64(out)
    }

    /// Test inv_pow_x64 for a set of interest points
    #[test]
    fn test_inv_pow_poi() -> Result<(), ()> {

        let mut points_of_interest_x64 = vec![
            ZERO_X64,
            U256::one(),
            ONE_X64,
            U256_MAX
        ];
        points_of_interest_x64.append(&mut get_powers_of_2_x64(-64, 256 - 64, 1));
        points_of_interest_x64.append(&mut get_powers_of_2_minus_1_x64(-64, 256 - 64 + 1, 1));

        let points_of_interest_x64 = remove_duplicates_and_sort(points_of_interest_x64);

        // Get all permutations between the elements of points_of_interest_x64
        let points_of_interest_x64: Vec<(&U256, &U256)> = points_of_interest_x64.iter().cartesian_product(points_of_interest_x64.iter()).collect();

        let result = evaluate_impl::<(&U256, &U256)>(
            |(a, b): &(&U256, &U256)| inv_pow_x64(**a, **b),
            |(a, b): &(&U256, &U256)| target_inv_pow_x64(**a, **b).map_err(|_err| ()),
            points_of_interest_x64.clone()
        );

        println!("\ninv_pow_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= INV_POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }

    /// A helper function to generate a set of random samples from a 2d space within the specified range.
    /// The generated samples should all yield valid outputs.
    fn get_inv_pow_randrange_only_valid(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256
    ) -> Vec<(U256, U256)> {

        // Define the limiting functions: when picked a random value of x, compute the range of values from which to pick y (and vice-versa)
        let x_range_given_y_fn = |y: U256| -> Result<(U256, U256), ()> {

            let y_f = uint_x64_to_float(&y);

            let x_min = x_range_start;

            let x_max_f = 2_f64.powf((64_f64/y_f).min(256_f64 - 64_f64));    // 2^( min(64/(y/2^64), 256-64) )*2^64     min used to clip values, as otherwise, for very small p, the power grows to infinity
            let mut x_max = match float_to_uint_x64(&x_max_f) {
                Ok(val) => val,
                Err(msg) => if msg.contains("overflow") { U256_MAX } else { panic!("Unable to compute range limit for pow function. Unknown error at uint_x64_to_float") }
            };

            if x_max > x_range_end { x_max = x_range_end }
    
            if x_max < x_min { return Err(()); }

            Ok((x_min, x_max))

        };

        let y_range_given_x_fn = |x: U256| -> Result<(U256, U256), ()> {

            let x_f = uint_x64_to_float(&x);

            let y_min = y_range_start;

            let y_max_f = 64_f64 / x_f.log2();    // (256 - 64)/log2(x / 2^64)*2^64
            let mut y_max = match float_to_uint_x64(&y_max_f) {
                Ok(val) => val,
                Err(msg) => if msg.contains("overflow") { U256_MAX } else { panic!("Unable to compute range limit for pow function. Unknown error at uint_x64_to_float") }
            };

            if y_max > y_range_end { y_max = y_range_end }

            if y_max < y_min { return Err(()); }

            Ok((y_min, y_max))

        };

        sample_2d_space(
            sample_count,
            x_range_start,
            x_range_end,
            y_range_start,
            y_range_end,
            Some(x_range_given_y_fn),
            Some(y_range_given_x_fn),

        )
    }

    /// A helper function to generate a set of random samples from a 2d space within the specified range.
    /// No condition is set on the generated samples (may yield invalid outputs)
    fn get_inv_pow_randrange_all(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256
    ) -> Vec<(U256, U256)> {
        sample_2d_space(
            sample_count,
            x_range_start,
            x_range_end,
            y_range_start,
            y_range_end,
            None::<fn(U256) -> Result<(U256, U256), ()>>,
            None::<fn(U256) -> Result<(U256, U256), ()>>

        )
    }

    /// Test pow_x64 for a random set of numbers - full range (expect all invalid points, as the power function very easily overflows)
    #[test]
    fn test_inv_pow_randrange_all() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_all(
            200000_usize,
            ONE_X64,
            U256_MAX,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ninv_pow_x64 - Randrange x: [1, max_x64), p: [0, max_x64)\n{}", result);

        assert!(result.max_abs_error <= INV_POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }

    /// Test inv_pow_x64 for a random set of numbers - full range (expect all invalid points, as the inverse power function very easily goes to 0)
    #[test]
    fn test_inv_pow_randrange_x_less_than_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_all(
            200000_usize,
            ZERO_X64,
            ONE_X64,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ninv_pow_x64 - Randrange x: [0, 1_x64), p: [0, max_x64)\n{}", result);

        assert!(result.max_abs_error <= INV_POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );
        Ok(())  
    }


    /// Test inv_pow_x64 for a random set of numbers - analysis of a medium x given a (very) small range of y
    #[test]
    fn test_inv_pow_randrange_subrange_small_y_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256::from(2).shl(64) * ONE_X64,
            U256::zero(),
            ONE_X64.shr(2) + ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^64_x64), p: [0, 2^(-2)_x64 + 1_x64)\n{}", result);

        assert!(result.max_abs_error <= INV_POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }

    /// Test inv_pow_x64 for a random set of numbers - analysis of all x given a (very) small range of y
    #[test]
    fn test_inv_pow_randrange_subrange_small_y_2() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            U256_MAX,
            U256::zero(),
            ONE_X64.shr(2) + ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, max_x64), p: [0, 2^(-2)_x64 + 1_x64)\n{}", result);

        assert!(result.max_abs_error <= INV_POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }


    
    /// Test inv_pow_x64 for a random set of numbers - analysis of y given a (small) range of x
    #[test]
    fn test_inv_pow_randrange_subrange_small_x_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            ONE_X64.shr(29) + ONE_X64,
            U256::zero(),
            ONE_X64.shl(10),
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^(-29)_x64 + 1_x64), p: [0, 2^(-33)_x64 + 1_x64)\n{}", result);

        assert!(result.max_abs_error <= INV_POW_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= INV_POW_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }

    /// Test inv_pow_x64 for a random set of numbers - analysis of medium y given a (small) range of x
    #[test]
    fn test_inv_pow_randrange_subrange_small_x_2() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            ONE_X64.shr(29) + ONE_X64,
            ONE_X64.shl(10),
            ONE_X64.shl(40),
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^(-29)_x64 + 1_x64), p: [0, 2^(-33)_x64 + 1_x64)\n{}", result);

        // ! Error bounds not tested as the error rapidly increases for medium/large y
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }

    /// Test inv_pow_x64 for a random set of numbers - analysis of medium y given a (small) range of x
    #[test]
    fn test_inv_pow_randrange_subrange_small_x_3() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_inv_pow_randrange_only_valid(
            200000_usize,
            ONE_X64,
            ONE_X64.shr(29) + ONE_X64,
            ONE_X64.shl(40),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| inv_pow_x64(*a, *b),
            |(a, b): &(U256, U256)| target_inv_pow_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\npow_x64 - Randrange x: [1_x64, 2^(-29)_x64 + 1_x64), p: [0, 2^(-33)_x64 + 1_x64)\n{}", result);

        // ! Error bounds not tested as the error rapidly increases for medium/large y
        assert!(result.invalid_count_expected_none == 0);

        // ! For outputs that result in extremely small values ( ~<2^{-44}*2^{64} ), the output of the implemented function
        // ! fails instead of returning 0. This is checked here (any failures must be for targets < 2^{-40}*2^{64}):
        assert!(
            result.relative_errors.iter().zip(&result.target_points).all(|(rel_error, target)| -> bool {
                match rel_error {
                    Ok(_) => true,
                    Err(EvalError::CalcForInvalidTarget) => false,  // This should not have happened (as per assert!(result.invalid_count_expected_none == 0) above)
                    Err(EvalError::NoCalcForValidTarget) => target.unwrap() < (ONE_X64 >> 40)
                }
            })
        );

        Ok(())  
    }
    

}