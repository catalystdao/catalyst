use std::ops::{Shr, Shl};

use cosmwasm_std::Uint128;
use catalyst_types::{U256, I256};


pub fn u256_to_f64(val: U256) -> f64 {
    let bytes = val.to_le_bytes();

    bytes.iter().enumerate().fold(
        0.,
        |acc, (i, val)| {
            acc + (*val as f64) * 2_f64.powf((i as f64)*8.)
        }
    )
}

pub fn i256_to_f64(val: I256) -> f64 {
    
    if val < I256::zero() {
        -1. * u256_to_f64(val.wrapping_neg().as_u256())
    }
    else {
        u256_to_f64(val.as_u256())
    }

}

pub fn uint128_to_f64(val: Uint128) -> f64 {
    u256_to_f64(U256::from(val.u128()))
}

pub fn f64_to_u256(val: f64) -> Result<U256, String> {
    // f64 standard => See IEEE-754-2008
    //      exponent: 11 bits
    //      mantissa: 52 bits

    let val_be_bytes = val.to_be_bytes();

    // Verify provided f64 value is not a negative number
    if val_be_bytes[0] & 0x80 != 0 {
        return Err("Failed to convert f64 to U256: provided f64 is a negative number.".to_string());
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
                                                                    

    // Convert the exponent into the net bit shift required to move the significant figure into the U256 number
    exponent = exponent - 52;   // -52 as the mantissa is 52 bits long


    if exponent <= -64 {
        return Ok(U256::zero());
    }
    
    if exponent >= 256 || (exponent >= 193 && significant_figure.shr(256-(exponent as u32)) != 0) {
        return Err("Failed to convert f64 to U256: overflow".to_string());
    }

    // Create a U256 from the mantissa_arr given the computed exponent
    let words: [u64; 4] = [
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
    ];

    let words = [
        words[0].to_le_bytes(),
        words[1].to_le_bytes(),
        words[2].to_le_bytes(),
        words[3].to_le_bytes(),
    ];

    let bytes = unsafe {
        std::mem::transmute::<[[u8; 8]; 4], [u8; 32]>(words)
    };

    Ok(U256::from_le_bytes(bytes))
}

pub fn f64_to_uint128(val: f64) -> Result<Uint128, String> {
    f64_to_u256(val)?
        .try_into()
        .map_err(|_| "Overflow when casting from U256 to Uint128".to_string())
}