
#[cfg(test)]
pub mod test_common {
    use std::ops::Shr;
    use crate::u256::U256;

    use std::{ops::{Shl, Sub}, f64::{INFINITY, NEG_INFINITY}, fmt};

    use cached::proc_macro::cached;

    use rug::{Float, integer::Order};
    use uint::construct_uint;

    construct_uint! {
        pub struct U512(8);
    }

    pub fn u256_to_u512(val: U256) -> U512 {
        U512([val.0[0], val.0[1], val.0[2], val.0[3], 0, 0, 0, 0])
    }

    pub fn u512_to_expanded_u256(val: U512) -> (U256, U256) {
        (U256([val.0[0], val.0[1], val.0[2], val.0[3]]), U256([val.0[4], val.0[5], val.0[6], val.0[7]]))
    }

    pub fn u512_to_u256(val: U512) -> Result<U256, ()> {
        let (a, b) = u512_to_expanded_u256(val);
        if b.is_zero() { Ok(a)   }
        else           { Err(()) }
    }

    pub fn safe_u256_mul_x64(a: U256, b: U256) -> Result<U256, ()> {
        // Will compute the product of two U256_x64 numbers guaranteeing no overflows within the calculation 
        // logic (result may still overflow)

        let (a, b) = safe_u256_expanded_mul_x64(a, b);

        if b.is_zero() { Ok(a)   }
        else           { Err(()) }
    }

    pub fn safe_u256_expanded_mul_x64(a: U256, b: U256) -> (U256, U256) {
        // Will compute the product of two U256_x64 numbers guaranteeing no overflows by expanding into two U256 numbers
        let a = u256_to_u512(a);
        let b = u256_to_u512(b);
        let c = (a * b) >> 64;

        u512_to_expanded_u256(c)
    }

    pub fn safe_u256_div_x64(a: U256, b: U256) -> Result<U256, ()> {
        // Will compute the division of two U256_x64 numbers guaranteeing no overflows within the calculation 
        // logic (result may still overflow)

        let (a, b) = safe_u512_expanded_div_x64(a, b);

        if b.is_zero() { Ok(a)   }
        else           { Err(()) }
    }

    pub fn safe_u512_expanded_div_x64(a: U256, b: U256) -> (U256, U256) {
        // Will compute the division of two U256_x64 numbers guaranteeing no overflows by expanding into two U256 numbers
        let a = u256_to_u512(a);
        let b = u256_to_u512(b);
        let c = (a << 64) / b;

        u512_to_expanded_u256(c)
    }

    #[cached]
    pub fn get_powers_of_2_x64(start: i64, stop: i64, step: usize) -> Vec<U256> {
       let iter = (start..stop).step_by(step);
       iter.map(|val| U256::one().shl(val + 64)).collect()
    }

    #[cached]
    pub fn get_powers_of_2_minus_1_x64(start: i64, stop: i64, step: usize) -> Vec<U256> {
        if stop == 256-64+1 {   // Equality condition and not greater than, as greater stop values will fail in either case
            // Special condition, as for this case (max stop value allowed), get_powers_of_2_x64 will overflow
            let mut vec: Vec<U256> = get_powers_of_2_x64(start, stop-1, step).iter().map(|val| val.sub(1)).collect();
            vec.push(U256([0xFFFFFFFFFFFFFFFFu64, 0xFFFFFFFFFFFFFFFFu64, 0xFFFFFFFFFFFFFFFFu64, 0xFFFFFFFFFFFFFFFFu64]));
            return vec;
        }
        get_powers_of_2_x64(start, stop, step).iter().map(|val| val.sub(1)).collect()
    }

    pub fn remove_duplicates_and_sort(l: Vec<U256>) -> Vec<U256> {
        let mut l = l.to_owned();
        l.sort();
        l.dedup();
        l
    }

    pub fn uint_x64_to_high_precision_float(val: &U256) -> Float {
        let mut out: Float = Float::with_val(256, val.0[0]);
        out = out >> 64;
        out += val.0[1];
        out += Float::with_val(256, val.0[2]) << 64;
        out += Float::with_val(256, val.0[3]) << 128;

        out
    }

    pub fn high_precision_float_to_uint_x64(val: Float) -> Result<U256, String> {
        
        let val: Float = val << 64;
        let out_int = val.to_integer().ok_or("Uknown error (infinity?)")?;

        let out_vec = out_int.to_digits::<u64>(Order::LsfLe);
        if out_vec.len() > 4 { return Err("".to_owned())}

        Ok(U256([
            *out_vec.get(0).unwrap_or(&0_u64),
            *out_vec.get(1).unwrap_or(&0_u64),
            *out_vec.get(2).unwrap_or(&0_u64),
            *out_vec.get(3).unwrap_or(&0_u64)
        ]))
    }

    pub fn uint_x64_to_float(val: &U256) -> f64 {
        let val_arr = val.0;

        let mut out: f64 = (val_arr[0] as f64) / 2_f64.powf(64_f64);  // Decimal part
        out += val_arr[1] as f64;
        out += (val_arr[2] as f64) * 2_f64.powf(64_f64);
        out += (val_arr[3] as f64) * 2_f64.powf(128_f64);

        out
    }

    pub fn float_to_uint_x64(val: &f64) -> Result<U256, String> {
        // f64 standard => See IEEE-754-2008
        //      exponent: 11 bits
        //      mantissa: 52 bits

        let val_be_bytes = val.to_be_bytes();

        // Verify provided f64 value is not a negative number
        if val_be_bytes[0] & 0x80 != 0 {
            return Err("Failed to convert f64 to U256_x64: provided f64 is a negative number.".to_string());
        }
        // TODO nan or infinity

        // Get the floating point number exponent
        let mut exponent_arr = [0_u8; 2];
        exponent_arr[..2].clone_from_slice(&val_be_bytes[..2]);     // Copy the first 2 bytes (16 bits)

        let mut exponent: i16 = i16::from_be_bytes(exponent_arr);       // Create number from the first 2 bytes
        exponent = exponent.shr(4);                                     // Shift the number 4 bits right, as we are only intersted in 12 bits
        exponent -= 1023_i16;                                           // Subtract exponent offset (see IEEE-754-2008)


        // Get the floating point mantissa, and convert it into a u64 number
        let mut mantissa_arr = [0_u8; 8];
        mantissa_arr[1..].clone_from_slice(&val_be_bytes[1..]);    // Copy the last 7 bytes of the floating point value (56 bits)

        // Remove the first 4 bits of the first byte copied (as we only care for the last 52 bits of the floating point value)
        // and set the bit that is right before the copied bytes to '1' (as the mantissa is the decimal part of a number which
        // always starts with 1)
        mantissa_arr[1] = (mantissa_arr[1] & 0x0Fu8) | 0x10u8;   

        let significant_figure = u64::from_be_bytes(mantissa_arr);
                                                                        

        // Convert the exponent into the net bit shift required to move the significant figure into the U256_x64 number
        exponent = exponent + 64 - 52;   // +64 because of the x64 notation, -52 as the mantissa is 52 bits long


        if exponent <= -64 {
            return Ok(U256::zero());
        }
        
        if exponent >= 256 || (exponent >= 193 && significant_figure.shr(256-(exponent as u32)) != 0) {
            return Err("Failed to convert f64 to U256_x64: overflow".to_string());
        }

        // Create a U256 from the mantissa_arr given the computed exponent
        Ok(U256([
            if exponent < 64 {
                if      exponent == 0   { significant_figure                  }
                else if exponent < 0    { significant_figure.shr(-exponent)   }
                else                    { significant_figure.shl(exponent)    }
            } else {0_u64},
    
            if exponent > 0 && exponent < 128 {
                if      exponent == 64  { significant_figure                  }
                else if exponent < 64   { significant_figure.shr(64-exponent) }
                else                    { significant_figure.shl(exponent-64) }
            } else {0_u64},

            if exponent > 64 && exponent < 192 {
                if      exponent == 128 { significant_figure                   }
                else if exponent < 128  { significant_figure.shr(128-exponent) }
                else                    { significant_figure.shl(exponent-128) }
            } else {0_u64},

            if exponent > 128 && exponent < 256 {
                if      exponent == 192 { significant_figure                   }
                else if exponent < 192  { significant_figure.shr(192-exponent) }
                else                    { significant_figure.shl(exponent-192) }
            } else {0_u64},
        ]))
    }

    pub fn rand_range(start: U256, end: U256) -> U256 {
        // TODO better implementation? Use U512
    
        let range = end.checked_sub(start).unwrap();
        let range_f = uint_x64_to_float(&range);
        
        float_to_uint_x64(&(range_f * rand::random::<f64>())).unwrap().checked_add(start).unwrap()
    }


    pub fn sample_space(start: U256, end: U256, sample_count: usize) -> Vec<U256> {
        (0..sample_count).map(|_| rand_range(start, end)).collect()
    }

    pub fn sample_2d_space(
        sample_count: usize,
        x_range_start: U256,
        x_range_end: U256,
        y_range_start: U256,
        y_range_end: U256,
        x_range_given_y_fn: Option<impl Fn(U256) -> Result<(U256, U256), ()>>,
        y_range_given_x_fn: Option<impl Fn(U256) -> Result<(U256, U256), ()>>,
    ) -> Vec<(U256, U256)> {

        let range_given_x = (0..(sample_count/2)).map(|_| -> (U256, U256) {
            loop {
                let x = rand_range(x_range_start, x_range_end);
    
                let y = match &y_range_given_x_fn {
                    Some(range_fn) => {
                        match range_fn(x) {
                            Ok(range) => rand_range(range.0, range.1),
                            Err(_) => continue
                        }
                    },
                    None => rand_range(y_range_start, y_range_end)
                };
    
                return (x, y);
            }
        });

        let range_given_y = (0..(sample_count/2)).map(|_| -> (U256, U256) {
            loop {
                let y = rand_range(y_range_start, y_range_end);

                let x = match &x_range_given_y_fn {
                    Some(range_fn) => {
                        match range_fn(y) {
                            Ok(range) => rand_range(range.0, range.1),
                            Err(_) => continue
                        }
                    },
                    None => rand_range(x_range_start, x_range_end)
                };
    
                return (x, y);
            }
        });

        range_given_x.chain(range_given_y).collect()

    }

    pub fn get_rel_error(value: U256, target: U256) -> f64 {
        if value.is_zero() && target.is_zero() {
            return 0.;
        }

        let value_f  = uint_x64_to_float(&value);
        let target_f = uint_x64_to_float(&target);

        2.*(value_f - target_f)/(value_f + target_f)
    }

    pub type EvalRelError = Option<f64>;

    #[derive(Debug)]
    pub enum EvalError {
        NoCalcForValidTarget,
        CalcForInvalidTarget
    }

    pub struct EvaluateImplResult<T> {
        pub low_error       : f64,
        pub high_error      : f64,
        pub avg_error       : f64,
        pub min_abs_error   : f64,
        pub max_abs_error   : f64,
        pub avg_abs_error   : f64,
        pub eval_points     : Vec<T>,
        pub calc_points     : Vec<Result<U256, ()>>,
        pub target_points   : Vec<Result<U256, ()>>,
        pub relative_errors : Vec<Result<EvalRelError, EvalError>>,
        pub valid_count     : u64,
        pub invalid_count_expected_none: u64,
        pub invalid_count_expected_some: u64,
        pub expected_some_count : u64,
        pub expected_none_count : u64,
    }

    impl<T> fmt::Display for EvaluateImplResult<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            writeln!(f, "Out of {} samples (of which {} are expected to fail)", self.relative_errors.len(), self.target_points.iter().filter(|target_result| target_result.is_err()).count())?;
            writeln!(f, "")?;
            writeln!(f, "- Lowest error\t: {:+.4e}", self.low_error)?;
            writeln!(f, "- Highest error\t: {:+.4e}", self.high_error)?;
            writeln!(f, "- Avg error\t: {:+.4e}", self.avg_error)?;
            writeln!(f, "")?;
            writeln!(f, "- Min abs error\t: {:+.4e}", self.min_abs_error)?;
            writeln!(f, "- Max abs error\t: {:+.4e}", self.max_abs_error)?;
            writeln!(f, "- Avg abs error\t: {:+.4e}", self.avg_abs_error)?;
            writeln!(f, "")?;
            writeln!(f, "- ERROR counts:")?;
            writeln!(f, "\t- None output for expected valid\t: {}", self.invalid_count_expected_some)?;
            writeln!(f, "\t- Some output for expected invalid\t: {}", self.invalid_count_expected_none)?;
            Ok(())
        }
    }

    pub fn evaluate_impl<T> (
        impl_fn: fn(&T) -> Result<U256, ()>,
        target_fn: fn(&T) -> Result<U256, ()>,
        eval_points: Vec<T>
    ) -> EvaluateImplResult<T> {

        let calc_points   : Vec<Result<U256, ()>> = eval_points.iter().map(|p_x64| impl_fn(p_x64)).collect();
        let target_points : Vec<Result<U256, ()>> = eval_points.iter().map(|p_x64| target_fn(p_x64)).collect();

        let mut high_error: f64 = NEG_INFINITY;
        let mut low_error: f64 = INFINITY;
        let mut avg_error: f64 = 0.0;
        let mut avg_abs_error: f64 = 0.0;
        let mut valid_count: u64 = 0;
        let mut invalid_count_expected_none: u64 = 0;
        let mut invalid_count_expected_some: u64 = 0;
        let mut expected_some_count: u64 = 0;
        let mut expected_none_count: u64 = 0;

        let relative_errors: Vec<Result<EvalRelError, EvalError>> = calc_points.iter().zip(&target_points).map(|(eval, target)| -> Result<EvalRelError, EvalError> {
            match (eval, target) {
                (Err(()), Err(())) => {
                    expected_none_count += 1;
                    Ok(None)
                },
                (Ok(e), Ok(t)) => {

                    let rel_error = get_rel_error(*e, *t);

                    if rel_error > high_error { high_error = rel_error }
                    if rel_error < low_error  { low_error = rel_error }

                    valid_count   += 1;
                    avg_error     += rel_error;
                    avg_abs_error += rel_error.abs();

                    expected_some_count += 1;

                    Ok(Some(rel_error))
                },
                (Ok(_), Err(())) => {
                    expected_none_count += 1;
                    invalid_count_expected_none += 1;
                    Err(EvalError::CalcForInvalidTarget)
                },
                (Err(()), Ok(_)) => {
                    expected_some_count += 1;
                    invalid_count_expected_some += 1;
                    Err(EvalError::NoCalcForValidTarget)
                }
            }
        }).collect();

        if valid_count == 0 {
            high_error = 0_f64;
            low_error  = 0_f64;
        }
        else {
            avg_error     /= valid_count as f64;
            avg_abs_error /= valid_count as f64;
        }

        let min_abs_error = low_error.abs().min(high_error.abs());
        let max_abs_error = low_error.abs().max(high_error.abs());


        EvaluateImplResult {
            high_error,
            low_error,
            avg_error,
            min_abs_error,
            max_abs_error,
            avg_abs_error,
            eval_points,
            calc_points,
            target_points,
            relative_errors,
            valid_count,
            invalid_count_expected_none,
            invalid_count_expected_some,
            expected_some_count,
            expected_none_count
        }
    }
}