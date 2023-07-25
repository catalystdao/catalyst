
// NOTE: The following macros are wrappers around 'parse_str_radix'. These are here for
// backwards compatibility.

#[macro_export]
macro_rules! u256 {
    ($integer:literal) => {{
        use $crate::U256;
        U256::parse_str_radix($integer, 10)
    }};
    ($integer:literal, $radix:expr) => {{
        use $crate::U256;
        U256::parse_str_radix($integer, $radix)
    }};
}

#[macro_export]
macro_rules! i256 {
    ($integer:literal) => {{
        use $crate::I256;
        I256::parse_str_radix($integer, 10)
    }};
    ($integer:literal, $radix:expr) => {{
        use $crate::I256;
        I256::parse_str_radix($integer, $radix)
    }};
}
