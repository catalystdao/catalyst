#[cfg(test)]
mod test_mul {
    use std::ops::{Mul, Shr};
    use itertools::Itertools;

    use crate::u256::U256;
    use crate::test::test_common::test_common::*;
    use crate::fixed_point_math_x64::*;



    // Test mul_x64 ************************************************************************************************************

    // Set test bounds
    const MUL_MAX_ABS_ERROR_BOUND: f64 = 0_f64;
    const MUL_AVG_ABS_ERROR_BOUND: f64 = 0_f64;

    /// Compute accurately mul of two numbers using U512 numbers
    pub fn target_mul_x64(a: U256, b: U256) -> Result<U256, String> {

        // Convert to U512 to avoid any overflows

        let a = U512([a.0[0], a.0[1], a.0[2], a.0[3], 0, 0, 0, 0]);
        let b = U512([b.0[0], b.0[1], b.0[2], b.0[3], 0, 0, 0, 0]);

        let c = a.mul(b).shr(64);

        if c >= U512([0, 0, 0, 0, 1, 0, 0, 0]) { return Err("Mul overflow".to_owned()) }

        Ok(U256([c.0[0], c.0[1], c.0[2], c.0[3]]))
    }

    /// Test mul_x64 for a set of interest points
    #[test]
    fn test_mul_poi() -> Result<(), ()> {

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
            |(a, b): &(&U256, &U256)| mul_x64(**a, **b),
            |(a, b): &(&U256, &U256)| target_mul_x64(**a, **b).map_err(|_err| ()),
            points_of_interest_x64.clone()
        );

        println!("\nmul_x64 - Points of interest\n{}", result);

        assert!(result.max_abs_error <= MUL_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= MUL_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        Ok(())  
    }

    /// A helper function to generate a set of random samples from a 2d space within the specified range.
    /// The generated samples should all yield valid outputs.
    fn get_mul_randrange_only_valid(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256
    ) -> Vec<(U256, U256)> {

        // Define the limiting functions: when picked a random value of x, compute the range of values from which to pick y (and vice-versa)
        let x_range_given_y_fn = |y: U256| -> Result<(U256, U256), ()> {

            let x_min = x_range_start;
            let mut x_max = safe_u256_div_x64(U256_MAX, y)?;

            if x_max > x_range_end { x_max = x_range_end }
    
            if x_max < x_min { return Err(()); }

            Ok((x_min, x_max))

        };

        let y_range_given_x_fn = |x: U256| -> Result<(U256, U256), ()> {

            let y_min = y_range_start;
            let mut y_max = safe_u256_div_x64(U256_MAX, x)?;

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
    fn get_mul_randrange_all(
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

    /// Test mul_x64 for a random set of numbers within the entire space of accepted inputs
    /// (all expected to generate valid outputs)
    #[test]
    fn test_mul_randrange_valid() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_mul_randrange_only_valid(
            200000_usize,
            U256::zero(),
            U256_MAX,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| mul_x64(*a, *b),
            |(a, b): &(U256, U256)| target_mul_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nmul_x64 - Randrange (only valid) x: [0, MAX_x64), y: [0, MAX_x64)\n{}", result);

        assert!(result.max_abs_error <= MUL_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= MUL_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }

    /// Test mul_x64 for a random set of numbers within the entire space of accepted inputs
    /// (many, if not all, will result in invalid outputs)
    #[test]
    fn test_mul_randrange_any() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_mul_randrange_all(
            200000_usize,
            U256::zero(),
            U256_MAX,
            U256::zero(),
            U256_MAX,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| mul_x64(*a, *b),
            |(a, b): &(U256, U256)| target_mul_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nmul_x64 - Randrange (any) x: [0, MAX_x64), y: [0, MAX_x64)\n{}", result);

        assert!(result.max_abs_error <= MUL_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= MUL_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count > 0);

        Ok(())  
    }

    /// Test mul_x64 for a random set of numbers within a section of the space of accepted inputs
    /// (all expected to generate valid outputs)    
    #[test]
    fn test_mul_randrange_subrange_1() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_mul_randrange_only_valid(
            200000_usize,
            U256::zero(),
            U256::from(2) * ONE_X64,
            U256::zero(),
            U256::from(2) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| mul_x64(*a, *b),
            |(a, b): &(U256, U256)| target_mul_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nmul_x64 - Randrange x: [0, 2_x64), y: [0, 2_x64)\n{}", result);

        assert!(result.max_abs_error <= MUL_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= MUL_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }

    /// Test mul_x64 for a random set of numbers within a section of the space of accepted inputs
    /// (all expected to generate valid outputs)    
    #[test]
    fn test_mul_randrange_subrange_2() -> Result<(), ()> {

        let points_of_interest_x64: Vec<(U256, U256)> = get_mul_randrange_only_valid(
            200000_usize,
            U256::zero(),
            (U256::from(2) << 98) * ONE_X64,
            U256::zero(),
            (U256::from(2) << 98) * ONE_X64,
        );

        let result = evaluate_impl(
            |(a, b): &(U256, U256)| mul_x64(*a, *b),
            |(a, b): &(U256, U256)| target_mul_x64(*a, *b).map_err(|_err| ()),
            points_of_interest_x64
        );

        println!("\nmul_x64 - Randrange x: [0, 2^34_x64), y: [0, 2^34_x64)\n{}", result);

        assert!(result.max_abs_error <= MUL_MAX_ABS_ERROR_BOUND);
        assert!(result.avg_abs_error <= MUL_AVG_ABS_ERROR_BOUND);
        assert!(result.invalid_count_expected_none == 0);
        assert!(result.invalid_count_expected_some == 0);

        assert!(result.expected_none_count == 0);

        Ok(())  
    }

}