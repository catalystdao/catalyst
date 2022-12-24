# Tests

The tests mostly follow those implemented for the Catalyst simulator. Refer to those and also on the maths library profiling report on Notion for further insight into implemented tests.

## Dependecies
The tests use the Rust crate **rug**. This crate requires the following libraries to be installed on the system running the tests:
* GMP
* MPFR
* MPC

Try to run the tests and install any packages that are missing.

## Run Tests
* To run the tests:
    ```
    cargo test
    ```
    * In order to get stats show on the console, pass the `--nocapture` flag
        ```
        cargo test -- --nocapture
        ```
    * Specific tests can be run. See `cargo test` documentation
