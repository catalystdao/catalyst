#[cfg(test)]
mod test_div {
    use itertools::Itertools;

    use crate::u256::U256;
    use crate::test::test_common::test_common::*;
    use crate::fixed_point_math_x64::*;



    // Test div_x64 ************************************************************************************************************

    // Set test bounds
    const DIV_MAX_ABS_ERROR_BOUND: f64 = 0_f64;
    const DIV_AVG_ABS_ERROR_BOUND: f64 = 0_f64;

    /// Compute accurately div of two numbers using U512 numbers
    pub fn target_div_x64(a: U256, b: U256) -> Result<U256, String> {

        // Convert to U512 to avoid any overflows within the calculation

        if b.is_zero() {
            return Err("Can't divide by 0.".to_string())
        }

        let a = U512([a.0[0], a.0[1], a.0[2], a.0[3], 0, 0, 0, 0]);
        let b = U512([b.0[0], b.0[1], b.0[2], b.0[3], 0, 0, 0, 0]);

        let c = (a << 64) / b;

        if c >= U512([0, 0, 0, 0, 1, 0, 0, 0]) { return Err("Div overflow".to_owned()) }

        Ok(U256([c.0[0], c.0[1], c.0[2], c.0[3]]))
    }

    /// Test div_x64 for a set of interest points
    #[test]
    fn test_div_poi() -> Result<(), ()> {

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
            |(a, b): &(&U256, &U256)| div_x64(**a, **b),
            |(a, b): &(&U256, &U256)| target_div_x64(**a, **b).map_err(|_err| ()),
            points_of_interest_x64.clone()
        );

        println!("\ndiv_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())
    }

    /// A helper function to generate a set of random samples from a 2d space within the specified range.
    /// The generated samples should all yield valid outputs.
    fn get_div_randrange_only_valid(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256
    ) -> Vec<(U256, U256)> {

        // Define the limiting functions
        let x_range_given_y_fn = |y: U256| -> Result<(U256, U256), ()> {

            let mut x_min = y >> 64;                                                     //  (y/2**64)*2**-64*2**64
            let mut x_max = if (y >> 64).is_zero() { y << (256-64) } else { U256_MAX };  //  (y/2**64)*2**(256-64)*2**64

            if x_min < x_range_start { x_min = x_range_start }
            if x_max > x_range_end   { x_max = x_range_end   }

            if x_max < x_min { return Err(()) }
            
            Ok((x_min, x_max))
        };

        let y_range_given_x_fn = |x: U256| -> Result<(U256, U256), ()> {

            let mut y_min = x >> 256-64;                                                     // x/2**64/2**(256-64)*2**64
            let mut y_max = if (x >> (256 - 64)).is_zero() { x << 64 } else { U256_MAX };    // x/2**64/2**(-64)*2**64

            if y_min < y_range_start { y_min = y_range_start }
            if y_max > y_range_end   { y_max = y_range_end   }

            if y_max < y_min { return Err(()) }
            
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
    fn get_div_randrange_all(
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


    /// Test div_x64 for a random set of numbers within the entire space of accepted inputs
    /// (all expected to generate valid outputs)
    #[test]
    fn test_div_randrange_valid() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            200000_usize,
            U256::zero(),
            U256_MAX,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange (only valid) x: [0, MAX_x64), y: [0, MAX_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }

    /// Test div_x64 for a random set of numbers within the entire space of accepted inputs
    /// (many, if not all, will result in invalid outputs)
    #[test]
    fn test_div_randrange_any() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_all(
            200000_usize,
            U256::zero(),
            U256_MAX,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange (any) x: [0, MAX_x64), y: [0, MAX_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }



    /// Test pow_x64 for a random set of numbers - analysis of a x given a small range of y
    /// (all expected to generate valid outputs)  
    #[test]
    fn test_div_randrange_subrange_small_y_1() -> Result<(), ()> {

        // y > 1, small
        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            20000_usize,
            U256::zero(),
            U256::from(2) * ONE_X64,
            ONE_X64,
            U256::from(8) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange x: [0, 2_x64), y: [1_x64, 8_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }

    /// Test pow_x64 for a random set of numbers - analysis of a x given a small range of y
    /// (all expected to generate valid outputs)  
    #[test]
    fn test_div_randrange_subrange_small_y_2() -> Result<(), ()> {

        // y positive, small
        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            20000_usize,
            U256::zero(),
            U256::from(2).pow(U256::from(128)) * ONE_X64,
            ONE_X64,
            U256::from(8) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange x: [0, 2^128_x64), y: [1_x64, 8_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }

    /// Test pow_x64 for a random set of numbers - analysis of a x given a small range of y
    /// (all expected to generate valid outputs)  
    #[test]
    fn test_div_randrange_subrange_small_y_3() -> Result<(), ()> {

        // y positive, small
        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            20000_usize,
            U256::zero(),
            U256::from(2).pow(U256::from(256-64-63)) * ONE_X64,
            ONE_X64,
            U256::from(8) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange x: [0, 2^(192+1)_x64), y: [1_x64, 8_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }



    /// Test pow_x64 for a random set of numbers - analysis of a (small) y given a small range of x
    /// (all expected to generate valid outputs)  
    #[test]
    fn test_div_randrange_subrange_4() -> Result<(), ()> {

        // y positive, small
        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            20000_usize,
            U256::zero(),
            U256::from(8) * ONE_X64,
            ONE_X64 >> 4,
            ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange x: [0, 8_x64), y: [2^(-4)_x64, 1_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }

    /// Test pow_x64 for a random set of numbers - analysis of a (small) y given a small range of x
    /// (all expected to generate valid outputs)  
    #[test]
    fn test_div_randrange_subrange_5() -> Result<(), ()> {

        // y positive, small
        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            20000_usize,
            U256::zero(),
            U256::from(8) * ONE_X64,
            ONE_X64 >> 12,
            ONE_X64 >> 4,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange x: [0, 8_x64), y: [2^(-12)_x64, 2^(-4)_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }

    /// Test pow_x64 for a random set of numbers - analysis of a (small) y given a small range of x
    /// (all expected to generate valid outputs)  
    #[test]
    fn test_div_randrange_subrange_6() -> Result<(), ()> {

        // y positive, small
        let points_of_interest_x64: Vec<(U256, U256)> = get_div_randrange_only_valid(
            20000_usize,
            U256::zero(),
            U256::from(8) * ONE_X64,
            U256::zero(),
            ONE_X64 >> 12,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| div_x64(*a, *b),
            |(a, b): &(U256, U256)| target_div_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\ndiv_x64 - Randrange x: [0, 8_x64), y: [0, 2^(-12)_x64)\n{}", result);

        assert!(result.max_abs_error <= DIV_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= DIV_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }

}