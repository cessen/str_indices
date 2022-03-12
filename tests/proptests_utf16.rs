#[macro_use]
extern crate proptest;

use proptest::string::string_regex;
use proptest::test_runner::Config;
use str_indices::utf16;

//===========================================================================

proptest! {
    #![proptest_config(Config::with_cases(512))]
}
