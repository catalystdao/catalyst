#[macro_export]
macro_rules! u256 {
    ($integer:literal) => {{
        use $crate::U256;
        use $crate::macros::internal::uint;
        let (hi, lo) = uint!($integer).into_words();
        U256::from_words(hi, lo)
    }};
}

#[macro_export]
macro_rules! i256 {
    ($integer:literal) => {{
        use $crate::I256;
        use $crate::macros::internal::int;
        let (hi, lo) = int!($integer).into_words();
        I256::from_words(hi, lo)
    }};
}

// Re-export ethnum's uint and int macro
#[doc(hidden)]
pub mod internal {
    pub use ethnum::{uint, int};
}